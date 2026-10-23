use std::cell::RefCell;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ink_rs::{AnimationOptions, AppHandle, ColorLevel, GraphicsSupport, Scope, State};

use crate::image::{
    CellSize, ImageLimits, LOCAL_TIMEOUT, LoadError, NETWORK_TIMEOUT, is_network, size_in_cells,
};

use super::feed::FrameFeed;
use super::{FrameRequest, VideoDecoders, VideoInfo};

/// What a player plays and how large its frames get.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoOptions {
    /// A file path or URL, selecting the local or network decoder slot.
    pub src: String,
    /// Whether playback starts when ready. Defaults to true.
    pub autoplay: bool,
    /// Whether playback returns to the start at the end. Defaults to false.
    pub looping: bool,
    /// The column limit, bounded by the window width. Defaults to the window width.
    pub max_columns: Option<usize>,
    /// The row limit, bounded by 20. Defaults to 20.
    pub max_rows: Option<usize>,
    /// The frame rate limit. Defaults to 24.
    pub max_fps: u32,
    /// The pixel count limit of a frame. Defaults to 640 times 360.
    pub max_pixels: u32,
}

impl Default for VideoOptions {
    fn default() -> Self {
        Self {
            src: String::new(),
            autoplay: true,
            looping: false,
            max_columns: None,
            max_rows: None,
            max_fps: 24,
            max_pixels: 640 * 360,
        }
    }
}

/// The state of a video player.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoStatus {
    /// The terminal probe or video information is pending.
    Loading,
    /// The position advances with the playback clock.
    Playing,
    /// The position is held until playback resumes or seeks.
    Paused,
    /// Playback reached the end and retains its last frame.
    Ended,
    /// The terminal or decoder cannot play the source.
    Failed(LoadError),
}

/// A handle to a player's state, passed to one [`super::Video`].
///
/// Methods take effect on the next render. A second view of the same player is not supported.
#[derive(Clone)]
pub struct VideoPlayer {
    store: Store,
    pub(super) model: Rc<RefCell<Model>>,
    pub(super) snapshot: Snapshot,
}

impl PartialEq for VideoPlayer {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.model, &other.model) && self.snapshot == other.snapshot
    }
}

impl VideoPlayer {
    /// Starts playback, returning to zero after the end.
    pub fn play(&self) {
        self.store.update(|model| {
            let mut model = model.borrow_mut();
            if model.status == VideoStatus::Ended {
                model.seek(Duration::ZERO);
            }
            if model.status == VideoStatus::Paused {
                model.status = VideoStatus::Playing;
            }
        });
    }

    /// Holds playback at its current position.
    pub fn pause(&self) {
        self.store.update(|model| {
            let mut model = model.borrow_mut();
            if model.status == VideoStatus::Playing {
                model.status = VideoStatus::Paused;
            }
        });
    }

    /// Seeks to a position, clamped to the duration when it is known.
    /// Seeking from the end leaves the player paused.
    pub fn seek(&self, position: Duration) {
        self.store.update(|model| model.borrow_mut().seek(position));
    }

    /// Returns the player's current status.
    pub fn status(&self) -> VideoStatus {
        self.model.borrow().status.clone()
    }

    /// Returns the current position in the video.
    pub fn position(&self) -> Duration {
        self.model.borrow().position()
    }

    /// Returns the duration when the decoder reports one.
    pub fn duration(&self) -> Option<Duration> {
        self.model
            .borrow()
            .info
            .as_ref()
            .and_then(|info| info.duration)
    }
}

#[derive(Clone, PartialEq)]
pub(super) struct Snapshot {
    pub status: VideoStatus,
    pub size: Option<CellSize>,
    pub level: ColorLevel,
    pub stream: u64,
    position: Duration,
    duration: Option<Duration>,
    sequence: u64,
    tick: u64,
}

pub(super) struct Frame {
    pub sequence: u64,
    pub stream: u64,
    pub png: Vec<u8>,
}

#[derive(Clone, PartialEq)]
struct Configuration {
    options: VideoOptions,
    decoders: VideoDecoders,
    support: GraphicsSupport,
    level: ColorLevel,
    limits: ImageLimits,
}

struct InfoJob {
    app: Mutex<Option<AppHandle>>,
    started: Instant,
    timeout: Duration,
}

impl InfoJob {
    fn cancel(&self) {
        self.app.lock().expect("video info job").take();
    }
}

pub(super) struct Model {
    config: Option<Configuration>,
    status: VideoStatus,
    info: Option<VideoInfo>,
    job: Option<Arc<InfoJob>>,
    loading: Duration,
    feed: Option<FrameFeed>,
    request: Option<FrameRequest>,
    generation: u64,
    stream: u64,
    position: Duration,
    size: Option<CellSize>,
    waiting: bool,
    pending_paint: bool,
    sequence: u64,
    tick: u64,
    clock: Option<(AnimationOptions, Duration)>,
    pub frame: Option<Frame>,
}

type Store = State<Rc<RefCell<Model>>>;

impl Model {
    fn position(&self) -> Duration {
        self.info
            .as_ref()
            .and_then(|info| info.duration)
            .map_or(self.position, |duration| self.position.min(duration))
    }

    fn new() -> Self {
        Self {
            config: None,
            status: VideoStatus::Loading,
            info: None,
            job: None,
            loading: Duration::ZERO,
            feed: None,
            request: None,
            generation: 0,
            stream: 0,
            position: Duration::ZERO,
            size: None,
            waiting: false,
            pending_paint: false,
            sequence: 0,
            tick: 0,
            clock: None,
            frame: None,
        }
    }

    fn stop(&mut self) {
        if let Some(job) = self.job.take() {
            job.cancel();
        }
        self.feed = None;
        self.waiting = false;
        self.pending_paint = false;
    }

    fn fail(&mut self, error: LoadError) {
        self.stop();
        self.status = VideoStatus::Failed(error);
        self.frame = None;
    }

    fn restart(&mut self, position: Duration) {
        self.stream += 1;
        self.position = position;
        self.frame = None;
        self.waiting = true;
        self.pending_paint = false;
    }

    fn seek(&mut self, position: Duration) {
        let Some(feed) = &self.feed else { return };
        let duration = self.info.as_ref().and_then(|info| info.duration);
        let position = duration.map_or(position, |duration| position.min(duration));
        self.generation = feed.seek(position);
        self.restart(position);
        if self.status == VideoStatus::Ended {
            self.status = VideoStatus::Paused;
        }
    }

    fn prepare(&mut self) -> Result<(), LoadError> {
        let (Some(info), Some(config)) = (&self.info, &self.config) else {
            return Ok(());
        };
        let (size, mut request) = frame_request(info, &config.options, config.limits)?;
        self.size = Some(size);
        if self.request.as_ref().is_some_and(|old| {
            (old.width, old.height, old.fps) == (request.width, request.height, request.fps)
        }) {
            if let Some(feed) = &self.feed {
                feed.set_looping(config.options.looping);
            }
            return Ok(());
        }
        request.start = self.position();
        self.feed = Some(FrameFeed::new(
            config.decoders.decoder(&config.options.src)?,
            config.options.src.clone(),
            request.clone(),
            config.options.looping,
        )?);
        self.request = Some(request);
        self.generation = 0;
        self.restart(self.position());
        Ok(())
    }

    fn tick(&mut self, options: AnimationOptions, time: Duration) {
        let delta = match self.clock.replace((options, time)) {
            Some((old, previous)) if old == options => time.saturating_sub(previous),
            _ => Duration::ZERO,
        };
        self.tick += 1;
        self.pending_paint = false;
        if self.status == VideoStatus::Loading {
            self.loading += delta;
            if self
                .job
                .as_ref()
                .is_some_and(|job| self.loading >= job.timeout)
            {
                self.fail(LoadError::new("video info timed out"));
            }
            return;
        }
        if self.status == VideoStatus::Playing && !self.waiting {
            self.position = self.position.saturating_add(delta);
        }
        if self.status != VideoStatus::Playing && !self.waiting {
            return;
        }
        let Some(feed) = &self.feed else { return };
        let update = feed.poll(self.position);
        if let Some(error) = update.error {
            self.fail(error);
            return;
        }
        if update.generation != self.generation {
            self.generation = update.generation;
            self.restart(update.start);
        }
        if let Some(frame) = update.frame {
            self.sequence += 1;
            self.frame = Some(Frame {
                sequence: self.sequence,
                stream: self.stream,
                png: frame.png,
            });
            self.waiting = false;
            self.pending_paint = true;
        }
        // EOF is a position boundary, not the timestamp of the last decoded frame.
        if update.ended && update.next_frame.is_none() {
            self.position = self
                .info
                .as_ref()
                .and_then(|info| info.duration)
                .or(update.end)
                .unwrap_or(self.position);
            if self.status == VideoStatus::Playing {
                self.status = VideoStatus::Ended;
            }
            self.waiting = false;
        }
    }
}

struct StopPlayer(Rc<RefCell<Model>>);

impl Drop for StopPlayer {
    fn drop(&mut self) {
        self.0.borrow_mut().stop();
    }
}

/// Creates a player whose model belongs to the calling component.
///
/// The calling component renders at the frame rate while playing. Put the player
/// in a small component to bound that work. One player drives one [`super::Video`];
/// a second view of the same player is not supported.
pub fn use_video_player(cx: &mut Scope, options: VideoOptions) -> VideoPlayer {
    let decoders = cx.use_context::<VideoDecoders>().unwrap_or_default();
    let graphics = cx.use_graphics();
    let level = cx.use_color_level();
    let window = cx.use_window_size();
    let limits = ImageLimits {
        columns: options
            .max_columns
            .unwrap_or(usize::from(window.columns))
            .min(usize::from(window.columns)),
        rows: options.max_rows.unwrap_or(20),
        cell_pixels: graphics.cell_pixels,
    };
    let config = Configuration {
        options,
        decoders,
        support: graphics.support,
        level,
        limits,
    };
    let store = cx.use_state(|| Rc::new(RefCell::new(Model::new())));
    let model = store.get();
    let app = cx.use_app();
    cx.use_layout_effect(config.clone(), move || {
        store.update(|model| configure(&mut model.borrow_mut(), config, store, app));
    });
    let animation = {
        let model = model.borrow();
        AnimationOptions {
            active: matches!(model.status, VideoStatus::Loading | VideoStatus::Playing)
                || model.waiting
                || model.pending_paint,
            interval: Duration::from_secs_f64(
                1.0 / f64::from(model.request.as_ref().map_or(24, |request| request.fps)),
            )
            .max(Duration::from_millis(1)),
        }
    };
    let clock = cx.use_animation(animation);
    cx.use_layout_effect((animation, clock.time), move || {
        store.update(|model| model.borrow_mut().tick(animation, clock.time));
    });
    let cleanup = model.clone();
    cx.use_effect((), move || StopPlayer(cleanup));
    let snapshot = {
        let model = model.borrow();
        Snapshot {
            status: model.status.clone(),
            size: model.size,
            level,
            stream: model.stream,
            position: model.position(),
            duration: model.info.as_ref().and_then(|info| info.duration),
            sequence: model.sequence,
            tick: model.tick,
        }
    };
    VideoPlayer {
        store,
        model,
        snapshot,
    }
}

fn configure(model: &mut Model, config: Configuration, store: Store, app: AppHandle) {
    let reload = model.config.as_ref().is_none_or(|old| {
        old.options.src != config.options.src
            || old.decoders != config.decoders
            || old.support != config.support
            || old.level != config.level
    });
    model.config = Some(config.clone());
    if reload {
        model.stop();
        model.restart(Duration::ZERO);
        model.waiting = false;
        model.info = None;
        model.request = None;
        model.size = None;
        model.loading = Duration::ZERO;
        model.clock = None;
        model.status = VideoStatus::Loading;
        if config.support == GraphicsSupport::Probing {
            return;
        }
        if config.support != GraphicsSupport::Kitty {
            model.fail(LoadError::new("video needs kitty graphics"));
            return;
        }
        if !matches!(config.level, ColorLevel::TrueColor | ColorLevel::Ansi256) {
            model.fail(LoadError::new("video placeholders need 256 colors"));
            return;
        }
        let decoder = match config.decoders.decoder(&config.options.src) {
            Ok(decoder) => decoder,
            Err(error) => {
                model.fail(error);
                return;
            }
        };
        let job = Arc::new(InfoJob {
            app: Mutex::new(Some(app)),
            started: Instant::now(),
            timeout: if is_network(&config.options.src) {
                NETWORK_TIMEOUT
            } else {
                LOCAL_TIMEOUT
            },
        });
        model.job = Some(job.clone());
        let worker = job.clone();
        let result = std::thread::Builder::new()
            .name("eink-video-info".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    decoder.info(&config.options.src)
                }))
                .unwrap_or_else(|_| Err(LoadError::new("video decoder panicked")));
                let app = worker.app.lock().expect("video info job").take();
                if let Some(app) = app {
                    app.update(move || {
                        store.update(|model| {
                            let mut model = model.borrow_mut();
                            if !model
                                .job
                                .as_ref()
                                .is_some_and(|job| Arc::ptr_eq(job, &worker))
                                || model.status != VideoStatus::Loading
                            {
                                return;
                            }
                            model.job = None;
                            if worker.started.elapsed() >= worker.timeout {
                                model.fail(LoadError::new("video info timed out"));
                                return;
                            }
                            match result {
                                Ok(info) => {
                                    model.info = Some(info);
                                    model.status = if model
                                        .config
                                        .as_ref()
                                        .expect("configuration")
                                        .options
                                        .autoplay
                                    {
                                        VideoStatus::Playing
                                    } else {
                                        VideoStatus::Paused
                                    };
                                    if let Err(error) = model.prepare() {
                                        model.fail(error);
                                    }
                                }
                                Err(error) => model.fail(error),
                            }
                        })
                    });
                }
            });
        if let Err(error) = result {
            model.fail(error.into());
        }
    } else if !matches!(model.status, VideoStatus::Failed(_))
        && let Err(error) = model.prepare()
    {
        model.fail(error);
    }
}

fn frame_request(
    info: &VideoInfo,
    options: &VideoOptions,
    limits: ImageLimits,
) -> Result<(CellSize, FrameRequest), LoadError> {
    let size = size_in_cells((info.width, info.height), limits)
        .ok_or_else(|| LoadError::new("video has no display area"))?;
    let width = f64::from(info.width);
    let height = f64::from(info.height);
    let mut scale = (f64::from(options.max_pixels.max(1)) / (width * height))
        .sqrt()
        .min(1.0);
    if let Some((cell_width, cell_height)) = limits.cell_pixels {
        scale = scale
            .min(size.columns as f64 * f64::from(cell_width.max(1)) / width)
            .min(size.rows as f64 * f64::from(cell_height.max(1)) / height);
    }
    let pixels = options.max_pixels.max(1);
    let width = (width * scale).floor().max(1.0) as u32;
    let height = (height * scale).floor().max(1.0) as u32;
    // At subpixel aspect ratios the minimum one-pixel edge still obeys the area cap.
    let width = width.min(pixels / height.max(1)).max(1);
    let height = height.min(pixels / width).max(1);
    Ok((
        size,
        FrameRequest {
            start: Duration::ZERO,
            width,
            height,
            fps: options
                .max_fps
                .min(info.fps.unwrap_or(options.max_fps))
                .max(1),
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_fit_placement_source_and_pixel_budget_and_choose_fps() {
        let info = VideoInfo {
            width: 1920,
            height: 1080,
            duration: None,
            fps: Some(30),
        };
        let limits = ImageLimits {
            columns: 80,
            rows: 20,
            cell_pixels: Some((10, 20)),
        };
        let options = VideoOptions::default();
        let (size, request) = frame_request(&info, &options, limits).unwrap();
        assert_eq!((request.width, request.height, request.fps), (640, 360, 24));
        assert!(request.width as usize <= size.columns * 10);
        assert!(request.height as usize <= size.rows * 20);
        for (width, height, budget, cells) in [
            (1920, 1080, 5000, Some((10, 20))),
            (1080, 1920, 5000, Some((10, 20))),
            (16, 9, 230400, Some((10, 20))),
            (1920, 1080, 230400, None),
            (100000, 1, 1, None),
            (1, 100000, 1, None),
        ] {
            let info = VideoInfo {
                width,
                height,
                ..info.clone()
            };
            let (size, request) = frame_request(
                &info,
                &VideoOptions {
                    max_pixels: budget,
                    ..options.clone()
                },
                ImageLimits {
                    cell_pixels: cells,
                    ..limits
                },
            )
            .unwrap();
            assert!(request.width <= width && request.height <= height);
            assert!(request.width >= 1 && request.height >= 1);
            assert!(request.width * request.height <= budget);
            if let Some((w, h)) = cells {
                assert!(request.width as usize <= size.columns * usize::from(w));
                assert!(request.height as usize <= size.rows * usize::from(h));
            }
            if request.width > 1 && request.height > 1 {
                let scale = f64::from(request.width) / f64::from(width);
                assert!((f64::from(request.height) - f64::from(height) * scale).abs() <= 2.0);
            }
        }
        for (source, limit, expected) in [
            (None, 24, 24),
            (Some(10), 24, 10),
            (Some(30), 0, 1),
            (Some(0), 24, 1),
        ] {
            let (_, request) = frame_request(
                &VideoInfo {
                    fps: source,
                    ..info.clone()
                },
                &VideoOptions {
                    max_fps: limit,
                    ..options.clone()
                },
                limits,
            )
            .unwrap();
            assert_eq!(request.fps, expected);
        }
    }
}
