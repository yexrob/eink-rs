use std::collections::VecDeque;
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use image::ImageEncoder;
use image::codecs::png::{CompressionType, FilterType, PngEncoder};

use crate::image::LoadError;

use super::{FrameRequest, VideoDecoder};

pub(crate) struct TimedFrame {
    pub time: Duration,
    pub png: Vec<u8>,
}

pub(crate) struct FeedUpdate {
    pub frame: Option<TimedFrame>,
    pub next_frame: Option<Duration>,
    /// A changed generation resets the consumer's clock to `start`.
    pub generation: u64,
    pub start: Duration,
    /// The next poll deadline when EOF follows the last frame.
    pub end: Option<Duration>,
    pub ended: bool,
    pub error: Option<LoadError>,
}

struct State {
    request: FrameRequest,
    generation: u64,
    ready: VecDeque<TimedFrame>,
    /// The consumer's last polled position.
    position: Duration,
    end: Option<Duration>,
    error: Option<LoadError>,
    looping: bool,
    stopped: bool,
}

impl State {
    fn restart(&mut self, start: Duration) {
        self.generation += 1;
        self.request.start = start;
        self.position = start;
        self.ready.clear();
        self.end = None;
        self.error = None;
    }
}

struct Shared {
    state: Mutex<State>,
    changed: Condvar,
}

/// One source's decoder and PNG queue. Polling never waits for decoding.
///
/// Seek and drop take effect between decoder calls. A blocked `frames` or `next`
/// call must return before the worker can release its iterator.
pub(crate) struct FrameFeed {
    shared: Arc<Shared>,
}

impl FrameFeed {
    pub fn new(
        decoder: Arc<dyn VideoDecoder>,
        src: String,
        request: FrameRequest,
        looping: bool,
    ) -> Result<Self, LoadError> {
        request.frame_bytes()?;
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                position: request.start,
                request,
                generation: 0,
                ready: VecDeque::with_capacity(3),
                end: None,
                error: None,
                looping,
                stopped: false,
            }),
            changed: Condvar::new(),
        });
        let worker = shared.clone();
        std::thread::Builder::new()
            .name("eink-video".into())
            .spawn(move || run(worker, decoder, src))?;
        Ok(Self { shared })
    }

    /// Discards queued frames and invalidates any decoding work in flight.
    pub fn seek(&self, start: Duration) -> u64 {
        let mut state = self.shared.state.lock().expect("video feed");
        state.restart(start);
        self.shared.changed.notify_one();
        state.generation
    }

    pub fn set_looping(&self, looping: bool) {
        self.shared.state.lock().expect("video feed").looping = looping;
    }

    /// Takes the newest due frame. `position` is in the current generation's video time.
    ///
    /// Looping waits for the last frame's interval before resetting the generation.
    /// The caller resets its clock to `start` when the returned generation changes.
    pub fn poll(&self, position: Duration) -> FeedUpdate {
        let mut state = self.shared.state.lock().expect("video feed");
        state.position = position;
        let mut frame = None;
        while state
            .ready
            .front()
            .is_some_and(|frame| frame.time <= position)
        {
            frame = state.ready.pop_front();
        }
        let mut ended = state.end.is_some_and(|end| end <= position) && state.ready.is_empty();
        if ended && state.looping && state.error.is_none() {
            state.restart(Duration::ZERO);
            frame = None;
            ended = false;
        }
        self.shared.changed.notify_one();
        FeedUpdate {
            frame,
            next_frame: state.ready.front().map(|frame| frame.time),
            generation: state.generation,
            start: state.request.start,
            end: state.end,
            ended,
            error: state.error.clone(),
        }
    }
}

impl Drop for FrameFeed {
    fn drop(&mut self) {
        let mut state = self.shared.state.lock().expect("video feed");
        state.stopped = true;
        state.ready.clear();
        self.shared.changed.notify_one();
        // An arbitrary decoder can block in frames()/next(); do not join it on the UI thread.
    }
}

fn run(shared: Arc<Shared>, decoder: Arc<dyn VideoDecoder>, src: String) {
    loop {
        let (request, generation) = {
            let state = shared.state.lock().expect("video feed");
            if state.stopped {
                return;
            }
            (state.request.clone(), state.generation)
        };
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            decode(&shared, decoder.as_ref(), &src, &request, generation)
        }))
        .unwrap_or_else(|_| Err(LoadError::new("video decoder panicked")));
        let mut state = shared.state.lock().expect("video feed");
        if state.generation == generation {
            if let Err(error) = result {
                state.error = Some(error);
            }
            while !state.stopped && state.generation == generation {
                state = shared.changed.wait(state).expect("video feed");
            }
        }
        if state.stopped {
            return;
        }
    }
}

fn decode(
    shared: &Shared,
    decoder: &dyn VideoDecoder,
    src: &str,
    request: &FrameRequest,
    generation: u64,
) -> Result<(), LoadError> {
    let mut frames = decoder.frames(src, request)?;
    let mut index = 0_u64;
    loop {
        let time = frame_time(request, index)?;
        let next = index
            .checked_add(1)
            .ok_or_else(|| LoadError::new("video frame index overflows"))?;
        let late = {
            let mut state = shared.state.lock().expect("video feed");
            while !state.stopped && state.generation == generation && state.ready.len() == 3 {
                state = shared.changed.wait(state).expect("video feed");
            }
            if state.stopped || state.generation != generation {
                return Ok(());
            }
            // Polling takes the newest due frame, so a frame whose successor is due never shows.
            frame_time(request, next)? <= state.position
        };
        let frame = frames.next().transpose()?;
        if late && frame.is_some() {
            index = next;
            continue;
        }
        let frame = frame.map(|rgb| encode(&rgb, request)).transpose()?;
        let mut state = shared.state.lock().expect("video feed");
        if state.stopped || state.generation != generation {
            return Ok(());
        }
        let Some(png) = frame else {
            // An empty video must not become a process-spawning loop; a seek
            // past the last frame ends the stream.
            if index == 0 && state.looping && request.start.is_zero() {
                return Err(LoadError::new("video stream produced no frames"));
            }
            state.end = Some(time);
            return Ok(());
        };
        state.ready.push_back(TimedFrame { time, png });
        index = next;
    }
}

fn frame_time(request: &FrameRequest, index: u64) -> Result<Duration, LoadError> {
    let fps = u64::from(request.fps);
    let offset = Duration::new(index / fps, ((index % fps) * 1_000_000_000 / fps) as u32);
    request
        .start
        .checked_add(offset)
        .ok_or_else(|| LoadError::new("video frame time overflows"))
}

fn encode(rgb: &[u8], request: &FrameRequest) -> Result<Vec<u8>, LoadError> {
    if rgb.len() != request.frame_bytes()? {
        return Err(LoadError::new(
            "video decoder returned an invalid RGB frame size",
        ));
    }
    let mut png = Vec::new();
    PngEncoder::new_with_quality(&mut png, CompressionType::Fast, FilterType::Adaptive)
        .write_image(
            rgb,
            request.width,
            request.height,
            image::ExtendedColorType::Rgb8,
        )
        .map_err(|error| LoadError::new(error.to_string()))?;
    Ok(png)
}

#[cfg(test)]
mod tests;
