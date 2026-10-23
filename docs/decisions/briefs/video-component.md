# Task: the Video component and its player handle

## Goal

eink-rs has a video decoder and a frame feed (`src/video/`, the first half) and plays animated images with two image ids that take turns (`src/image/protocol.rs`). This task is the second half: a silent `Video` component with play, pause, loop and seek, driven by a player handle that the application holds. Frames go to the terminal through the animation's two-id mechanism. Add an example.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/032-animation-video.md`: every decision below comes from it, the component half in particular.
- `<repo>/docs/design.md` 「image」 and 「video」.
- The code: `<repo>/src/video/` (all of it), `<repo>/src/image/` (all of it; `protocol.rs` holds the two ids, `hook.rs` the loading clock and deadlines, `mod.rs` the `Image` component), `<repo>/tests/image.rs` (a test loader and a terminal that answers the probe as kitty), `<repo>/examples/image.rs`.
- ink-rs 0.1.7 at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ink-rs-0.1.7`: `Scope::use_animation` and `AnimationOptions` (changing the options restarts the clock at zero), `use_layout_effect` and `use_effect` (`src/runtime/mod.rs`: all layout effects run before all effects, children first; a state update in a layout effect renders again before paint), `AppHandle::update`, `OutputWriter::write_passthrough` and the timing rule in `write_control`'s doc, `use_input` and `KeyEvent`, `testing::mount`, `TestApp::advance`, `TestTerminal::writes`.

Facts checked in ink-rs 0.1.7: a component that unmounts during a commit can write from its effect cleanups (only app teardown drops writes). A parent's render renders its children again; there is no bailout on equal props.

## Decisions

**Public API.** Under the `video` feature, in `eink_rs::video`. Names may change if the code reads better; say so in the report.

```rust
/// What a player plays and how large its frames get.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoOptions {
    /// A file path or URL, selecting the decoder slot as images do.
    pub src: String,
    /// Whether playback starts once the video is ready. True by default.
    pub autoplay: bool,
    /// Whether playback returns to the start at the end. False by default.
    pub looping: bool,
    /// The column limit, bounded by the window width. Defaults to the window width.
    pub max_columns: Option<usize>,
    /// The row limit, bounded by 20. Defaults to 20.
    pub max_rows: Option<usize>,
    /// The frame rate limit. Defaults to 24.
    pub max_fps: u32,
    /// The pixel count limit of a frame. Defaults to 640 × 360.
    pub max_pixels: u32,
}

pub fn use_video_player(cx: &mut Scope, options: VideoOptions) -> VideoPlayer;

/// A handle to a player's state, passed to one `Video`. Methods take effect on the next render.
#[derive(Clone)]
pub struct VideoPlayer { /* crate-private */ }

impl VideoPlayer {
    pub fn play(&self);
    pub fn pause(&self);
    /// Clamped to the duration when it is known.
    pub fn seek(&self, position: Duration);
    pub fn status(&self) -> VideoStatus;
    pub fn position(&self) -> Duration;
    pub fn duration(&self) -> Option<Duration>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum VideoStatus { Loading, Playing, Paused, Ended, Failed(LoadError) }

/// A video drawn with kitty placeholders, or `[alt]` while loading or when unavailable.
#[derive(Clone, PartialEq, Props)]
#[props(component = video)]
pub struct Video {
    pub player: VideoPlayer,
    #[props(into)]
    pub alt: String,
}
```

- `VideoInfo` gets `pub fps: Option<u32>`: the average frame rate rounded up, `None` when unknown. `FfmpegDecoder` reads `avg_frame_rate` (`30000/1001` gives 30; `0/0` gives `None`).
- `VideoPlayer` equality covers the shared state's identity and every value the view reads, so it stays correct if ink-rs gains a props bailout.
- The rustdoc of `use_video_player` says that the calling component renders at the frame rate while playing, so the player belongs in a small component.
- Looping comes from `VideoOptions::looping`; the handle has no method for it.

**Model and view.** The hook owns the model; `Video` is the view.

- The hook reads `VideoDecoders` from context (default when none is provided), the graphics support, the color level and the window size. While the probe runs, the status is `Loading`. Without kitty graphics or with fewer than 256 colors, the status is `Failed` with an error that says so, and nothing loads.
- `info` runs on a worker thread and comes back through `AppHandle::update`. The deadline is the image deadline: `LOCAL_TIMEOUT` or `NETWORK_TIMEOUT` from `src/image/loaders.rs`. Past it the status is `Failed`, and a late answer is ignored. An empty slot, an `info` error or a feed error is `Failed`.
- Sizes: the cells come from `size_in_cells` with the video's display size and the limits built as `Image` builds them. A frame keeps the video's aspect ratio and is no larger than the placement's pixels (cells times cell pixels, when the terminal reports cell pixels), the video itself, and `max_pixels`; at least 1 × 1. The frame rate is `min(max_fps, info.fps)`, or `max_fps` when the video has none; at least 1.
- The hook owns the `FrameFeed`. A change of source starts over at loading. A change of frame size or frame rate starts a new feed at the current position. A change of `looping` calls `set_looping`.
- One `use_animation` clock with the frame interval. It runs while playing, while loading (for the deadline), and while a stream waits for its first frame. Each new stream (start, seek, loop, new feed) holds the position at its start until its first frame arrives, so process start-up does not skip frames. Later in a stream, late frames are skipped as the feed does.
- After load, `autoplay` decides between `Playing` and `Paused`; a paused player shows the frame at its position. Seeking keeps playing or paused; seeking from `Ended` gives `Paused`. At the end without looping, the status is `Ended`, the position stays at the end and the last frame stays on screen. `play` from `Ended` starts at zero.

**Frames to the terminal.**

- The hook takes frames from the feed in a layout effect on each clock tick, at the player's position, and gives each frame a sequence number that never repeats for that player. The view uploads the frame in an effect of the same commit, so a frame shows on the next tick, one interval after its time.
- The view allocates two ids through `ImageId::allocate`; if it cannot, it shows `[alt]`. It draws the placeholder rows in the color of the id that holds the newest frame of the current stream, and otherwise keeps the id it drew. A frame uploaded before a seek or a loop never shows after it. Uploads never target the id the render drew, except while that id holds no frame; each upload is followed by the id's virtual placement, and a size change sends the placements again.
- `Loading` and `Failed` show `[alt]` styled with `ImageTheme`, as `Image` does.
- On unmount the view deletes the idle id's data (`a=d,d=I,i=<id>,q=2`).
- One player drives one `Video`; say in the rustdoc that a second view of the same player is not supported.

**Shared two-id type.** Extract from `Playback` in `protocol.rs` one crate-private type that the animation and the video use: the id pair, the drawn slot, the upload with its placement, the placement resend on a size change, and the idle delete. The rule that picks the slot to draw stays with each user: the animation by show time, the video by sequence number. Do not copy code between `image` and `video`; make the items `video` needs `pub(crate)` (placeholder rows from a size and a color, `ImageLimits`, `size_in_cells`, `ImageId`, the timeouts).

**Cleanup.** Remove the `#[cfg_attr(not(test), allow(dead_code))]` on `feed` and `VideoDecoders::decoder` and their comments.

**Example.** `examples/video.rs`, `required-features = ["video"]`:

- `cargo run --example video --features video -- <src>` plays a file or URL; `-- --pattern` plays a test pattern from a decoder defined in the example (moving colored bars, 640 × 360, 10 seconds, 24 fps), for machines without ffmpeg.
- Space plays and pauses, ← and → seek 5 seconds, `l` toggles looping, `q` quits.
- A status line under the video shows the status, the position and the duration, and whether it loops.

## Tests

Each covers new logic and nothing more. Group them into as few tests as read well.

- The sizes: the frame keeps the aspect ratio within the placement's pixels, the video's size and `max_pixels`; the frame rate choice.
- The ffprobe parse: `avg_frame_rate` rounded up, and `0/0` as `None` (extend the existing test).
- Through `mount`, a test decoder defined in the test, and a terminal that answers the probe as kitty (see `tests/image.rs`), in `tests/video.rs`:
  - playback: the first frame goes into the drawn id; after one interval the placeholder cells carry the other id's color, and the next upload targets the first id and comes after the write that switched the color; no upload targets the id drawn at that moment;
  - pause stops uploads and the position; a seek while paused shows the frame at the target, and a frame decoded before the seek never shows;
  - the end: without looping the status is `Ended` and uploads stop; with looping the position returns to zero;
  - unmount deletes the idle id;
  - a failed `info` shows `[alt]` and the status `Failed`.

The feed and the decoder run on real threads; wait for them by advancing the app until the expected write appears, with a bounded number of steps. Add no binary fixtures.

## Out of scope

Audio. Video in `Markdown` or the stream. The `t=f`, `t=t`, `t=s` media and the `a=f`, `a=a`, `a=c` commands. Adaptive quality. `docs/`, README, CHANGELOG, the version number. Changes to the feed or the ffmpeg commands beyond `avg_frame_rate`; if the component needs one, make the smallest change and list it in the report.

## Rules

- `CLAUDE.md` holds. No `unsafe`.
- Code reads like the existing code: naming, comment density, idiom. Public items get rustdoc in plain declarative sentences.
- Add no dependencies. The sandbox has no network.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into the repository.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --workspace --all-targets -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features video -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features image -- -D warnings`
- `cargo test --offline --lib --features video` and `--all-features`
- `cargo test --offline --test video --features video`
- `cargo test --offline --test image --features image` and `cargo test --offline --test markdown_image --all-features`, since the two-id type changes
- `cargo build --offline --example video --features video`
- Run only these; the main agent runs the full suite and the manual checks.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you changed and added, and what each does;
- the public API, and any name you changed;
- the model: loading, the deadline, sizes, the feed's lifetime, the clock, the position, the status changes;
- the frames: where they are taken, the sequence numbers, how the view picks the id, what the effect sends and when, the unmount;
- the shared two-id type and how the animation uses it now;
- the example;
- every deviation from this brief, and why, and any ink-rs gap;
- the tests you added;
- the exact commands you ran and their results.
