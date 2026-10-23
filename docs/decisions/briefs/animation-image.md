# Task: play animated GIF, APNG and WebP images

## Goal

The `image` module shows the first frame of an animated GIF. After this task, animated GIF, APNG and WebP images play in the `Image` component and in the `Markdown` component, on every terminal where still images work today: kitty, Ghostty 1.3.1 (which has no kitty animation commands), herdr, and kitty-protocol terminals behind tmux. The method is per-frame retransmission into two image ids that take turns: the placeholder cells switch their foreground colour to the id that holds the frame due now, and the next frame goes into the other id. A later task builds video on the same mechanism, so keep it usable for frames that arrive from a decoder thread rather than from a list prepared at load time.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/032-animation-video.md`: every decision below comes from it.
- `<repo>/docs/decisions/023-image.md` and `<repo>/docs/design.md` 「image」: how still images work.
- `<scratchpad>/research/animation/terminal.md` sections 1, 2 and 5 (retransmission deletes placements; flicker; quotas), and `<scratchpad>/research/animation/decoding.md` sections 1 and 3.2 (`image` animation APIs, limits, loop counts; PNG `Fast` with the `Adaptive` filter).
- The code: `<repo>/src/image/` (all of it), `<repo>/src/markdown/mod.rs`, `images.rs`, `piece.rs`, `render.rs` (the placeholder rows and `render::Cache`), `stream.rs` (where the stream loads images), `<repo>/tests/image.rs`, `<repo>/tests/markdown_image.rs`.
- ink-rs 0.1.7 at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ink-rs-0.1.7`: `Scope::use_animation` and `AnimationOptions`, `OutputWriter::write_passthrough` and the timing rule in `write_control`'s doc, `use_effect` and its cleanup, `testing::mount`, `TestTerminal::writes`, `TestApp::advance`.

## Decisions

**Decoding.** In `image` 0.25, `GifDecoder`, `PngDecoder::apng()` and `WebPDecoder` stream full-canvas RGBA frames with disposal and blending applied (`AnimationDecoder::into_frames`).

- Detect animation with `image::guess_format` (GIF), `PngDecoder::is_apng` and `WebPDecoder::has_animation`. A file with one frame takes today's still path.
- Apply today's limits (32,768 pixels per side, 128 MiB `max_alloc`) on the concrete decoder (`set_limits`, `PngDecoder::with_limits`); `ImageReader` gives still images only. `Limits` bound one frame; WebP ignores them.
- Scale each frame to the pixel size a still image of the same natural size gets, and encode it as PNG with `CompressionType::Fast` and `FilterType::Adaptive`. Keep the encoded bytes and the delay of each frame; drop the pixels.
- Caps: when the encoded frames pass 32 MiB in total or 1,000 frames, stop decoding and show the first frame as a still image.
- A delay of 10 ms or less counts as 100 ms, as browsers do. The loop count comes from the decoder; `LoopCount::Infinite` loops forever. A GIF without a loop block reports `Infinite`; accept that.
- The load deadline covers the whole preparation, as today. Stop decoding when the job has expired.
- SVG does not change.

**Where animation plays.** The `Image` component and the `Markdown` component play animations. Pieces of a `MarkdownStream` go into `Static`, and placeholders in the scrollback cannot be redrawn, so the stream loads the first frame and sends one id, as today. Give `ImageRequest` a field that says which of the two is wanted. The props of `Image` do not change.

**Two ids.**

- An animated image allocates two ids at the colour level, through today's allocator. If the second id cannot be allocated, show the first frame as a still image.
- The render draws the placeholder cells in the colour of one of the two ids. When the id that is not on screen holds a frame whose time has come, the render switches to that id; otherwise it keeps the id on screen.
- After each commit, an effect sends the frame that follows the one due at the render's time into the id that the render did not draw, then that id's virtual placement (`a=p,U=1,i=<id>,p=1,c=<columns>,r=<rows>,q=2`): retransmitting an id deletes its placements. It never writes into the id the render drew. `write_passthrough` bytes go out after the frame that includes the earlier state changes, so the terminal sees the colour switch before the upload.
- When the image first becomes ready, the effect sends the first frame into the id drawn and the second frame into the other id.
- A size change sends each id's placement again with the new columns and rows, as for still images.
- When the component unmounts, delete the data of the id that is not on screen (`a=d,d=I,i=<id>,q=2`). The id on screen stays for the placeholders the terminal keeps. If writes from an effect cleanup are dropped by ink-rs, leave the deletion out and report it.
- After the last loop of a finite animation, the image stays on its last frame and sends nothing more.

**Clock.** A component with playing animations drives them with `use_animation`. The interval is at most the shortest delay among the playing frames and at least 20 ms; the clock is inactive when nothing plays. An image's time starts when it becomes ready. Keep the hook count fixed across renders; `use_images` runs a clock for load deadlines, and you may merge the two.

**Shape.** Keep the playback crate-private in `image` and shared by `Image` and `Markdown`: the placements a render uses say which id to draw, and `use_image_effect` (or its successor) sends what they need. Frames that a decoder thread delivers later (video) must fit the same sending path. Name things as you see fit and describe them in the report.

**Markdown.** `render::Cache` must not freeze the placeholder colour of a playing image. Make sure the rows of a playing image change with the id they draw.

## Tests

Each covers new logic and nothing more:

- the schedule: from the frame delays, the loop count and a time, the frame due; the end of a finite animation on its last frame;
- the decoding: a three-frame GIF built in the test with `image::codecs::gif::GifEncoder`, one frame with delay 0, gives three PNG frames at the display size, with the 0 delay counted as 100 ms;
- the protocol, through `mount`, a test loader and a `TestTerminal` that answers the probe as kitty (see `tests/image.rs`): the first frame goes into the drawn id and the second into the other; after `advance` past the first delay, the placeholder cells carry the other id's colour, the third frame goes into the first id, and that upload comes after the write that switched the colour; no upload targets the id drawn at that moment; after unmount, the idle id is deleted.

Use images built in the tests; add no binary fixtures.

## Out of scope

Video. The `t=f`, `t=t` and `t=s` media; the `a=f`, `a=a` and `a=c` commands. Deleting still images. New props on `Image`. `docs/`, README, CHANGELOG, the version number. New examples: `examples/image.rs` plays animated files as they are.

## Rules

- `CLAUDE.md` holds. No `unsafe`.
- Code reads like the existing code: naming, comment density, idiom. Public items get rustdoc in plain declarative sentences.
- Add no dependencies; the `image` crate has `gif`, `png` and `webp` on. The sandbox has no network.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into the repository.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --workspace --all-targets -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features image -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features markdown-image -- -D warnings`
- `cargo test --offline --lib --all-features`
- `cargo test --offline --test image --features image` and `--all-features`
- `cargo test --offline --test markdown_image --all-features`
- Run only these; the main agent runs the full suite and the manual checks.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you changed and added, and what each does;
- the decoding path: detection, limits, caps, delays, loop counts, the deadline;
- the playback: the clock, the schedule, how a render picks the id, what the effect sends and when, the unmount;
- how `Markdown` and the stream use it, and the `render::Cache` change;
- the shape a decoder thread would use to feed frames later;
- every deviation from this brief, and why, and any ink-rs gap;
- the tests you added;
- the exact commands you ran and their results.
