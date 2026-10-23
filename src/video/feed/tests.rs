use super::*;
use crate::video::{VideoFrames, VideoInfo};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

#[derive(Default)]
struct TestDecoder {
    requests: Mutex<Vec<FrameRequest>>,
    drops: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
    count: usize,
    invalid: bool,
}

struct Frames {
    drops: Arc<AtomicUsize>,
    reads: Arc<AtomicUsize>,
    remaining: usize,
    bytes: usize,
}

impl Drop for Frames {
    fn drop(&mut self) {
        self.drops.fetch_add(1, Ordering::SeqCst);
    }
}

impl Iterator for Frames {
    type Item = Result<Vec<u8>, LoadError>;

    fn next(&mut self) -> Option<Self::Item> {
        self.reads.fetch_add(1, Ordering::SeqCst);
        if self.remaining == 0 {
            return None;
        }
        self.remaining -= 1;
        Some(Ok(vec![42; self.bytes]))
    }
}

impl VideoDecoder for TestDecoder {
    fn info(&self, _: &str) -> Result<VideoInfo, LoadError> {
        Ok(VideoInfo {
            width: 2,
            height: 1,
            duration: None,
            fps: None,
        })
    }

    fn frames(&self, _: &str, request: &FrameRequest) -> Result<VideoFrames, LoadError> {
        self.requests.lock().unwrap().push(request.clone());
        Ok(Box::new(Frames {
            drops: self.drops.clone(),
            reads: self.reads.clone(),
            remaining: self.count,
            bytes: if self.invalid {
                1
            } else {
                request.frame_bytes()?
            },
        }))
    }
}

fn request() -> FrameRequest {
    FrameRequest {
        start: Duration::from_secs(1),
        width: 2,
        height: 1,
        fps: 10,
    }
}

fn wait(mut predicate: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !predicate() {
        assert!(
            Instant::now() < deadline,
            "video worker did not reach expected state"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

fn ready(feed: &FrameFeed, count: usize) {
    wait(|| feed.shared.state.lock().unwrap().ready.len() == count);
}

#[test]
fn frame_times_png_queue_bound_and_late_frame_skipping() {
    let decoder = Arc::new(TestDecoder {
        count: 20,
        ..Default::default()
    });
    let feed = FrameFeed::new(decoder.clone(), "test".into(), request(), false).unwrap();
    ready(&feed, 3);
    std::thread::sleep(Duration::from_millis(10));
    assert_eq!(decoder.reads.load(Ordering::SeqCst), 3);
    let early = feed.poll(Duration::ZERO);
    assert!(early.frame.is_none());
    assert_eq!(early.next_frame, Some(Duration::from_secs(1)));
    assert!(!early.ended);
    let update = feed.poll(Duration::from_millis(1150));
    let frame = update.frame.unwrap();
    assert_eq!(frame.time, Duration::from_millis(1100));
    assert_eq!(update.next_frame, Some(Duration::from_millis(1200)));
    let image = image::load_from_memory(&frame.png).unwrap().into_rgb8();
    assert_eq!(image.dimensions(), (2, 1));
    assert_eq!(image.into_raw(), vec![42; 6]);
    let mut fractional = request();
    fractional.fps = 3;
    assert_eq!(frame_time(&fractional, 3).unwrap(), Duration::from_secs(2));
}

#[test]
fn seeking_discards_ready_frames_and_restarts_at_target() {
    let decoder = Arc::new(TestDecoder {
        count: 20,
        ..Default::default()
    });
    let feed = FrameFeed::new(decoder.clone(), "test".into(), request(), false).unwrap();
    ready(&feed, 3);
    let generation = feed.seek(Duration::from_secs(5));
    ready(&feed, 3);
    assert_eq!(decoder.drops.load(Ordering::SeqCst), 1);
    let update = feed.poll(Duration::from_secs(5));
    assert_eq!(update.start, Duration::from_secs(5));
    assert_eq!(update.generation, generation);
    let frame = update.frame.unwrap();
    assert_eq!(frame.time, Duration::from_secs(5));
    assert_eq!(
        decoder.requests.lock().unwrap()[1].start,
        Duration::from_secs(5)
    );
}

#[test]
fn looping_waits_for_the_last_interval_and_restarts_at_zero() {
    let decoder = Arc::new(TestDecoder {
        count: 2,
        ..Default::default()
    });
    let feed = FrameFeed::new(decoder.clone(), "test".into(), request(), true).unwrap();
    wait(|| feed.shared.state.lock().unwrap().end.is_some());
    let last = feed.poll(Duration::from_millis(1100));
    assert_eq!(last.frame.unwrap().time, Duration::from_millis(1100));
    assert_eq!(last.end, Some(Duration::from_millis(1200)));
    assert!(!last.ended);
    let restarted = feed.poll(Duration::from_millis(1200));
    assert_eq!(restarted.generation, 1);
    assert_eq!(restarted.start, Duration::ZERO);
    assert!(restarted.frame.is_none());
    ready(&feed, 2);
    assert_eq!(decoder.requests.lock().unwrap()[1].start, Duration::ZERO);
    let first = feed.poll(Duration::ZERO).frame.unwrap();
    assert_eq!(first.time, Duration::ZERO);
    feed.set_looping(false);
    wait(|| feed.shared.state.lock().unwrap().end.is_some());
    let ended = feed.poll(Duration::from_millis(200));
    assert!(ended.ended);
    assert_eq!(ended.generation, 1);
}

#[test]
fn dropping_a_full_feed_drops_the_iterator() {
    let decoder = Arc::new(TestDecoder {
        count: 20,
        ..Default::default()
    });
    let feed = FrameFeed::new(decoder.clone(), "test".into(), request(), false).unwrap();
    ready(&feed, 3);
    drop(feed);
    wait(|| decoder.drops.load(Ordering::SeqCst) == 1);
    assert_eq!(decoder.reads.load(Ordering::SeqCst), 3);
}

#[test]
fn seeking_invalidates_a_decoder_call_in_flight() {
    use std::sync::mpsc;

    struct GatedDecoder {
        entered: mpsc::Sender<()>,
        release: Mutex<Option<mpsc::Receiver<()>>>,
    }

    impl VideoDecoder for GatedDecoder {
        fn info(&self, _: &str) -> Result<VideoInfo, LoadError> {
            unreachable!("the feed does not choose dimensions")
        }

        fn frames(&self, _: &str, request: &FrameRequest) -> Result<VideoFrames, LoadError> {
            let entered = self.entered.clone();
            let release = self.release.lock().unwrap().take();
            let bytes = request.frame_bytes()?;
            Ok(Box::new(std::iter::once_with(move || {
                if let Some(release) = release {
                    entered.send(()).unwrap();
                    release.recv().unwrap();
                    Err(LoadError::new("old generation failed"))
                } else {
                    Ok(vec![42; bytes])
                }
            })))
        }
    }

    let (entered, waiting) = mpsc::channel();
    let (release, blocked) = mpsc::channel();
    let feed = FrameFeed::new(
        Arc::new(GatedDecoder {
            entered,
            release: Mutex::new(Some(blocked)),
        }),
        "test".into(),
        request(),
        false,
    )
    .unwrap();
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    let generation = feed.seek(Duration::from_secs(5));
    let pending = feed.poll(Duration::from_secs(5));
    assert!(pending.frame.is_none());
    assert_eq!(pending.generation, generation);
    release.send(()).unwrap();
    ready(&feed, 1);
    let update = feed.poll(Duration::from_secs(5));
    assert!(update.error.is_none());
    let frame = update.frame.unwrap();
    assert_eq!(frame.time, Duration::from_secs(5));
}

#[test]
fn invalid_frames_and_empty_videos_report_errors() {
    let decoder = Arc::new(TestDecoder {
        count: 1,
        invalid: true,
        ..Default::default()
    });
    let feed = FrameFeed::new(decoder, "test".into(), request(), false).unwrap();
    wait(|| feed.shared.state.lock().unwrap().error.is_some());
    let update = feed.poll(Duration::MAX);
    assert!(update.error.is_some());
    assert!(update.frame.is_none());
    // A looping stream past the last frame restarts at zero, where no frames is an error.
    let decoder = Arc::new(TestDecoder::default());
    let feed = FrameFeed::new(decoder, "test".into(), request(), true).unwrap();
    wait(|| feed.shared.state.lock().unwrap().end.is_some());
    assert_eq!(feed.poll(Duration::from_secs(1)).generation, 1);
    wait(|| feed.shared.state.lock().unwrap().error.is_some());
    let update = feed.poll(Duration::ZERO);
    assert!(update.error.is_some());
    assert_eq!(update.generation, 1);
    let mut invalid = request();
    invalid.fps = 0;
    assert!(
        FrameFeed::new(
            Arc::new(TestDecoder::default()),
            "test".into(),
            invalid,
            false
        )
        .is_err()
    );
}
