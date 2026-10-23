//! Silent video playback with a player handle and source-selected decoders.

use std::sync::{Arc, OnceLock};
use std::time::Duration;

use crate::image::{LoadError, is_network};

pub(crate) mod feed;
mod ffmpeg;
mod player;
mod view;

pub use player::{VideoOptions, VideoPlayer, VideoStatus, use_video_player};
pub use view::Video;

pub use ffmpeg::FfmpegDecoder;

/// A video's display dimensions and length.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VideoInfo {
    /// Display width in pixels, after rotation.
    pub width: u32,
    /// Display height in pixels, after rotation.
    pub height: u32,
    /// The duration, or `None` when the source does not report one.
    pub duration: Option<Duration>,
    /// The average frame rate rounded up, or `None` when unknown.
    pub fps: Option<u32>,
}

/// Frames wanted from a decoder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrameRequest {
    /// Where the first frame comes from.
    pub start: Duration,
    /// The output width in pixels, greater than zero.
    pub width: u32,
    /// The output height in pixels, greater than zero.
    pub height: u32,
    /// Frame `n` shows at `start + n / fps`. This value must be greater than zero.
    pub fps: u32,
}

impl FrameRequest {
    fn frame_bytes(&self) -> Result<usize, LoadError> {
        if self.width == 0 || self.height == 0 || self.fps == 0 {
            return Err(LoadError::new("video dimensions and fps must be positive"));
        }
        usize::try_from(self.width)
            .ok()
            .and_then(|width| width.checked_mul(self.height as usize))
            .and_then(|pixels| pixels.checked_mul(3))
            .filter(|bytes| *bytes <= isize::MAX as usize)
            .ok_or_else(|| LoadError::new("video frame size overflows"))
    }
}

/// An owned stream of RGB frames. Dropping it stops decoding.
pub type VideoFrames = Box<dyn Iterator<Item = Result<Vec<u8>, LoadError>> + Send>;

/// Decodes one kind of video source. Methods run on worker threads.
pub trait VideoDecoder: Send + Sync {
    /// Reads the display dimensions and duration of a source.
    fn info(&self, src: &str) -> Result<VideoInfo, LoadError>;

    /// Returns RGB frames with `width * height * 3` bytes per item.
    ///
    /// The iterator ends with the video. Dropping it stops decoding.
    fn frames(&self, src: &str, request: &FrameRequest) -> Result<VideoFrames, LoadError>;
}

/// Decoders provided through [`ink_rs::Scope::provide`].
///
/// HTTP and HTTPS sources use `network`; other sources use `local`.
/// An empty slot disables that kind of source. Equality compares decoder identity.
#[derive(Clone)]
pub struct VideoDecoders {
    /// The decoder for paths and other local source names.
    pub local: Option<Arc<dyn VideoDecoder>>,
    /// The decoder for HTTP and HTTPS URLs.
    pub network: Option<Arc<dyn VideoDecoder>>,
}

impl VideoDecoders {
    /// Selects the source's decoder, or reports that its slot is disabled.
    pub(crate) fn decoder(&self, src: &str) -> Result<Arc<dyn VideoDecoder>, LoadError> {
        let slot = if is_network(src) {
            &self.network
        } else {
            &self.local
        };
        slot.clone()
            .ok_or_else(|| LoadError::new("video decoder slot is disabled"))
    }
}

impl PartialEq for VideoDecoders {
    fn eq(&self, other: &Self) -> bool {
        fn same(
            left: &Option<Arc<dyn VideoDecoder>>,
            right: &Option<Arc<dyn VideoDecoder>>,
        ) -> bool {
            match (left, right) {
                (Some(left), Some(right)) => Arc::ptr_eq(left, right),
                (None, None) => true,
                _ => false,
            }
        }
        same(&self.local, &other.local) && same(&self.network, &other.network)
    }
}

impl Default for VideoDecoders {
    /// Uses ffmpeg for both slots, with local paths relative to the current directory.
    fn default() -> Self {
        static DEFAULT: OnceLock<VideoDecoders> = OnceLock::new();
        DEFAULT
            .get_or_init(|| {
                let decoder: Arc<dyn VideoDecoder> = Arc::new(FfmpegDecoder::default());
                Self {
                    local: Some(decoder.clone()),
                    network: Some(decoder),
                }
            })
            .clone()
    }
}
