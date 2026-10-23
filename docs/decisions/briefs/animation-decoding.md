# Research: decoding animated images and video in Rust

## Context

ink-rs is a Rust port of Ink, the React renderer for terminal UIs; eink-rs is a component library on top of it. eink-rs 0.3.1 decodes still images with the `image` crate 0.25 (png, jpeg, gif first frame, webp; `default-features = false`), rasterizes SVG with resvg, scales to the display size with `resize_exact` (Lanczos3), encodes PNG, and sends it with the kitty graphics protocol (`a=t,t=d,f=100`, base64 in 4096-byte chunks). Loading runs on a background thread with a 20 MiB file limit and `image::Limits`.

The next step is animated images (GIF, APNG, animated WebP) and video. This is research only: change no files in any repository, write only the report named below. You may build and run scratch programs under `<scratchpad>/research/animation/` to measure things.

Constraints of the project: the eink-rs crate forbids `unsafe` in its own code (dependencies may use it); `rust-version` is 1.88; CI builds on Linux, macOS and Windows; features are opt-in, and a feature that is off must not compile its dependencies. External services and programs sit behind a trait with a local implementation for development and tests. eink-rs source, read only: `~/Episodes/Projects/bingo-inc/eink-rs` (see `docs/design.md`, section "image", and `src/image/decode.rs`).

## Questions

1. **Animated images in `image` 0.25.** The APIs for GIF (`GifDecoder` and `AnimationDecoder::into_frames`), APNG (`PngDecoder::apng`) and animated WebP: frame delays, whether disposal and blending are applied for you (are frames full canvases?), whether frames can be decoded one at a time (streaming) or only all at once, how `Limits` apply per frame, and which crate features each needs.
2. **Video decoding options.** Compare: running the `ffmpeg` command line and reading raw RGBA frames from a pipe (with `ffprobe` for metadata); `ffmpeg-next`/`ffmpeg-sys-next`; GStreamer bindings; pure-Rust or bundled decoders (`openh264`, `rav1d`/`dav1d`, `video-rs`, `re_mp4` and similar). For each: codecs and containers covered, how it builds on the three platforms, licence (LGPL/GPL implications for a library crate), MSRV against 1.88, binary size, whether our own code would need `unsafe`, and maintenance status. Note how seeking, frame timing and end of stream are exposed.
3. **Per-frame cost.** Measure, with scratch programs, for a 640×360 and a 1280×720 frame: scaling (`image::imageops` vs `fast_image_resize`), PNG encoding (default and fast compression) versus raw RGBA (`f=32`) with zlib (`o=z`), the resulting byte counts after base64, and so the bandwidth for 24 and 30 fps. Report the machine and numbers.
4. **Timing and audio in terminal players.** How terminal video players (`mpv --vo=kitty`, timg, and others you find) pace frames, drop late frames and handle audio. List Rust audio playback options (cpal, rodio) with platform notes; audio may be out of scope, so keep this short.
5. **Memory.** Memory for a GIF of 200 frames at 480×270 decoded up front versus streamed; what limits or caps players use.

## Report

Write `<scratchpad>/research/animation/decoding.md`: one section per question, each claim with its source (URL, docs.rs page, or measurement), uncertainty marked. End with a short list of the facts that most constrain a design. Do not propose an eink-rs design; the main agent does that with the user.
