# Task: video decoding behind a trait, with ffmpeg

## Goal

eink-rs will get a silent `Video` component with play, pause, loop and seek. It comes in two halves. This task is the first half: a decoder trait, its ffmpeg implementation, the two decoder slots, and a crate-private frame feed that turns a decoder's raw frames into timed PNG frames on a background thread. It has no component and writes nothing to the terminal. The second half, dispatched after a parallel task on animated images lands, builds the `Video` component, a player handle and the sending on top of this. Shape the feed for that use.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/032-animation-video.md`: every decision below comes from it.
- `<repo>/docs/decisions/023-image.md` and `<repo>/docs/design.md` 「image」: the image loaders and slots that the video slots mirror.
- `<scratchpad>/research/animation/decoding.md` section 2.1 (the ffmpeg pipe: commands, timing, seeking, throughput) and 3.2 (PNG `Fast` with the `Adaptive` filter).
- The code: `<repo>/src/image/loaders.rs` (`ImageLoaders`, `is_network`, `LoadError`), `<repo>/src/image/load.rs` (the loader thread), `<repo>/src/image/decode.rs` (PNG encoding).

## Decisions

**Feature.** `video = ["image"]`; it uses `image` for PNG encoding. Add no dependencies. Add the module as `pub mod video` behind the feature.

**Public API.** Names may change if the code reads better; say so in the report.

```rust
/// A video's display dimensions and length.
pub struct VideoInfo {
    pub width: u32,
    pub height: u32,
    /// `None` when the source does not report one, such as a live stream.
    pub duration: Option<Duration>,
}

/// Frames wanted from a decoder.
pub struct FrameRequest {
    /// Where the first frame comes from.
    pub start: Duration,
    pub width: u32,
    pub height: u32,
    /// Frame `n` shows at `start + n / fps`.
    pub fps: u32,
}

/// Decodes one kind of video source. Methods run on worker threads.
pub trait VideoDecoder: Send + Sync {
    fn info(&self, src: &str) -> Result<VideoInfo, LoadError>;
    /// Each item is `width * height * 3` bytes of RGB. The iterator ends with the video;
    /// dropping it stops decoding.
    fn frames(&self, src: &str, request: &FrameRequest)
        -> Result<Box<dyn Iterator<Item = Result<Vec<u8>, LoadError>> + Send>, LoadError>;
}

/// Decoders selected by source kind and provided through `Scope::provide`.
pub struct VideoDecoders {
    pub local: Option<Arc<dyn VideoDecoder>>,
    pub network: Option<Arc<dyn VideoDecoder>>,
}
```

- `LoadError` is `crate::image::LoadError`.
- `VideoDecoders` compares by decoder identity, as `ImageLoaders` does. HTTP and HTTPS sources use `network`, others `local` (`is_network`); an empty slot means the source fails.
- Without a provider, both slots hold an `FfmpegDecoder`; local paths resolve against the current directory. ffmpeg reads URLs itself, in ranges, without downloading the whole file. An application turns network video off by providing `network: None`.

**`FfmpegDecoder`.** Runs the `ffprobe` and `ffmpeg` programs with `std::process`; `ffmpeg` is not linked.

- Construction: a base directory for local paths, and the two program paths, `ffmpeg` and `ffprobe` on `PATH` by default.
- Input: a local source becomes `file:<base joined with src>`, with `-protocol_whitelist file`, so a source name can neither start an option nor pick another ffmpeg protocol (`concat:`, `subfile:`, playlists pointing elsewhere). A network source must start with `http://` or `https://`; it gets `-protocol_whitelist http,https,tcp,tls,crypto` and `-rw_timeout` of 10 seconds, the image network timeout.
- `info`: `ffprobe -v error -select_streams v:0` with the stream's width, height and rotation and the format's duration, in a key=value output that you parse. A rotation of ±90 or 270 degrees swaps width and height (ffmpeg rotates frames by default). A missing or `N/A` duration is `None`. No video stream is an error.
- `frames`: `ffmpeg -nostdin -v error <whitelist> -ss <start> -i <input> -map 0:v:0 -an -sn -dn -vf fps=<fps>,scale=<width>:<height> -pix_fmt rgb24 -f rawvideo pipe:1`. Read exact frames from stdout; a short read at the end ends the stream. Drain stderr on its own thread into a bounded buffer so the child cannot block on it; a non-zero exit becomes a `LoadError` that carries its last lines. Dropping the iterator kills and reaps the child.
- A missing program gives a `LoadError` that names it.

**Frame feed (crate-private).** One feed plays one source:

- A thread asks the decoder for frames and encodes each one as PNG (`CompressionType::Fast`, `FilterType::Adaptive`). Each frame carries its time in the video. It keeps up to three frames ready and then blocks, so ffmpeg blocks on the pipe; pausing costs nothing more than not taking frames.
- Seeking drops the current stream and the ready frames, and starts a new stream at the target time. Each stream has a generation number; frames of an older generation are never handed out.
- At the end of the video, a looping feed starts a new stream at zero (a new generation); a feed that does not loop reports that it ended.
- The consumer asks for the frame due at a time without blocking: it gets the newest ready frame whose time has come and drops older ones (late frames are skipped), and can see the time of the next ready frame. The consumer also sees an ended state and the decoder's error.
- Dropping the feed stops the thread and drops the decoder's iterator.
- The second half will call `info` first to choose the frame size, then start the feed. Do not choose sizes or frame rates here.

**Local implementation.** The decoder slots take any `VideoDecoder`. Tests define their own decoder, as the image tests define their own loader; the second half's example defines a test-pattern decoder for machines without ffmpeg. Add no public mock.

## Tests

Each covers new logic and nothing more:

- the ffmpeg arguments for a local and a network source: the `file:` prefix, the whitelists, `-rw_timeout`, `-ss`, the filter; a local source named `-x` stays an input;
- parsing the ffprobe output: dimensions, a 90-degree rotation, a missing duration, no video stream;
- the feed with a test decoder: frame times; at most three ready frames; seeking discards older frames and restarts at the target; looping restarts at zero; late frames are skipped; dropping the feed drops the decoder's iterator;
- real ffmpeg, one test that returns early when `ffmpeg` or `ffprobe` is not on `PATH`: make a one-second `testsrc2` clip of 64×36 in a temporary directory with ffmpeg, then check `info` and that `frames` from 0.5 s at 32×18 and 10 fps gives five frames of 32×18×3 bytes.

## Out of scope

The `Video` component, the player handle, frame sizes, the clock, anything written to the terminal, examples. Audio. `docs/`, README, CHANGELOG, the version number. A parallel task changes `src/image/` and `src/markdown/`: outside `src/video/`, change only what you need to reach existing items (for example the `cfg` on the `is_network` re-export), and list each such change in the report.

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
- `cargo test --offline --lib --features video` and `--all-features`
- any test file you add, with `--features video`
- Run only these; the main agent runs the full suite.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you changed and added, and what each does;
- the public API;
- the ffmpeg and ffprobe commands, and how errors, missing programs and child processes are handled;
- the feed: threads, the queue, generations, seeking, looping, ending, dropping, and the calls the second half will use;
- every change outside `src/video/`;
- every deviation from this brief, and why, and any ink-rs gap;
- the tests you added;
- the exact commands you ran and their results.
