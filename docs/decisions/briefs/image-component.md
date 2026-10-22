# Task: an `Image` component with loaders and kitty placeholders

## Goal

eink-rs will show images: first as an `Image` component, then inside Markdown (a later task). Terminals with the kitty graphics protocol and its Unicode placeholders show the pixels; elsewhere the component shows `[alt]`. ink-rs gives the primitives: `Scope::use_graphics()` probes kitty support and the cell size in pixels, `OutputWriter::write_passthrough` wraps bytes for tmux, and `Scope::use_color_level()` gives the colour level. This task builds the `image` module: loaders, decoding, the kitty encoding, and the component. The Markdown work builds on the crate-private parts, so shape them for it.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/023-image.md`: every decision below comes from it.
- `<repo>/docs/decisions/020-markdown-image-research.md` and `022-gpuix-herdr-research.md`.
- `<repo>/docs/design.md`: 「ui」, 「主题」.
- `<scratchpad>/research/dl/kitty-gp.rst`: the kitty graphics protocol. Read 「Unicode placeholders」, the transmission keys (`a`, `t`, `f`, `i`, `q`, `m`), chunking, and virtual placements (`U=1`).
- `<scratchpad>/diacritics.txt`: kitty's 297 row and column diacritics (`gen/rowcolumn-diacritics.txt`) as Rust `char` literals, in order. Put the table in the crate as a constant.
- `<scratchpad>/graphics-check/src/main.rs`: a working check, run in Ghostty, tmux and herdr, that transmits an image and draws placeholders through ink-rs. It shows the commands, the placeholder cells and the foreground colour. Its code is a sketch; do not copy its shortcuts.
- ink-rs, from the path in `[patch.crates-io]` of `<repo>/Cargo.toml`: `use_graphics`, `Graphics`, `GraphicsSupport`, `use_stdout`, `OutputWriter::write_passthrough`, `use_color_level`, `use_app`, `AppHandle::update`, `use_effect`, `use_window_size`, `Scope::provide`, `use_context`.

## Decisions

**Features.**

```toml
image = ["dep:image"]                  # Image, the loader trait, the file loader; png, jpeg, gif (first frame), webp
image-http = ["image", "dep:ureq"]     # the network loader
image-svg = ["image", "dep:resvg"]     # SVG
```

The main agent added these and the dependencies to `Cargo.toml` and fetched them. Do not change the dependency versions or features; the sandbox has no network. If you need a change, say so in the report.

**Loaders.** A trait, one implementation per kind of source, replaceable per kind:

```rust
pub trait ImageLoader: Send + Sync {
    /// Runs on a loader thread; returns the encoded bytes.
    fn load(&self, src: &str) -> Result<Vec<u8>, LoadError>;
}
pub struct ImageLoaders {
    pub local: Option<Arc<dyn ImageLoader>>,
    pub network: Option<Arc<dyn ImageLoader>>,
}
```

- `ImageLoaders` reaches components through `Scope::provide`. Without a provider: the local slot is a file loader based on the current directory; the network slot is the ureq loader with `image-http`, and empty without it.
- `http://` and `https://` sources go to the network slot; everything else to the local slot. An empty slot gives `[alt]`.
- Files over 20 MB fail. Decoding uses `image`'s `Limits`.
- Timeouts: local 5 s, network 10 s. A load past its deadline shows `[alt]`.
- Loads run off the UI thread; results come back through `AppHandle::update`.

**Decoding and transmission.** One path: decode, scale down to the display size in pixels, encode as PNG (`f=100`), send inline (`t=d`) in base64 chunks of 4096 bytes (`m=1` on all but the last), with `q=2`. Each chunk is one `write_passthrough` call. SVG is rasterised with resvg at the display size. Write the base64 encoder yourself; it is a few lines.

**Ids.** 24-bit image ids. The process picks a random start once (from `std`'s `RandomState`) and counts up. Panes inside tmux share the outer terminal's id space; the random start lowers collisions. eink-rs never deletes images; the terminal evicts old ones by its quota.

**Placement and placeholders.**

- After the data, a virtual placement: `a=p,U=1,i=<id>,c=<columns>,r=<rows>,q=2`. It is sent again when the image's columns or rows change. The data is sent once per image per process.
- The cells are ordinary `Text`: each cell is `U+10EEEE`, the row diacritic and the column diacritic, with the image id as the foreground colour.
- **The colour level decides the id's encoding.** At `TrueColor`, the foreground is `Rgb` of the id's three bytes. At `Ansi256`, the id must be below 256 and the foreground is `Ansi256(id)`; keep a separate counter for these ids. Below that, the image shows `[alt]`: a degraded colour would point at another image.
- Images are sent from an effect after the commit. A `Static` item runs its effect after it is written, so placeholders reach the terminal before the data; kitty and Ghostty draw when the data arrives.

**Size.** The width is at most the available columns and at most 297, the length of the diacritic table; the height follows the aspect ratio, at most 20 rows. The component never scales up past the image's natural size in cells. When the cell size in pixels is unknown, assume cells are twice as tall as wide.

**States.** While the probe runs or the image loads, and on any failure, the component shows `[alt]` (the theme's style). When the terminal does not support kitty placeholders, it shows `[alt]` and loads nothing.

**The component.**

```rust
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = image)]
pub struct Image {
    #[props(into)] pub src: String,
    #[props(into)] pub alt: String,
    /// Defaults to the window width.
    pub max_columns: Option<usize>,
    /// Defaults to 20.
    pub max_rows: Option<usize>,
}
```

Plus an `ImageTheme` for the `[alt]` style, following the theme rules.

**For the Markdown task**, keep these crate-private and usable on their own: loading one source to a status (loading, ready with the natural size, failed); the size in cells for given limits; the placeholder rows as styled text runs, so a renderer can put a one-row image inside a line of text; and one effect that sends the data and placements for a set of images a render used. Say in the report what each is called and how Markdown calls it.

## Tests

Each covers new logic:

- the base64 encoder against known vectors, and the chunking (`m=1` and `m=0`);
- the placement and transmission commands, through `testing::mount` with a test loader and a `TestTerminal` that answers the probe as kitty: the bytes in `writes()`, sent once; the placement sent again on a resize that changes the columns;
- the placeholder cells: diacritics by row and column, the `Rgb` id at `TrueColor`, the `Ansi256` id at `Ansi256`, `[alt]` at `Basic`;
- the size: the width cap, the 20-row cap, no upscaling, the 1:2 fallback;
- `[alt]` while loading, after a failure, after a timeout (through `advance`), and on a terminal without kitty;
- the slots: an `http` source with an empty network slot shows `[alt]`; a provided loader replaces the default;
- with `image-svg`: an SVG rasterises to the expected pixel size.

Use small images built in the tests; add no binary fixtures. Add nothing beyond these.

## Example

`examples/image.rs`: shows the image files given as arguments, each with its `alt`, at the window width. Register it with `required-features = ["image"]`. The main agent checks it in Ghostty, tmux and herdr.

## Out of scope

Markdown. `docs/`, README, CHANGELOG, the version number. Video, animation, half blocks, Sixel, iTerm2, file and shared-memory transmission, deleting images.

## Rules

- `CLAUDE.md` holds. No `unsafe`.
- Code reads like the existing code: naming, comment density, idiom. Public items get rustdoc in plain declarative sentences.
- Loaders and network access stay behind the trait; tests use a test loader and never the network.
- Leave the `[patch.crates-io]` section in `Cargo.toml` as it is; the main agent removes it after the ink-rs release.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into the repository.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --workspace --all-targets -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features image -- -D warnings`
- `cargo test --offline --lib --all-features`
- the test files you add, with `--features image` and with `--all-features`.
- Run only these; the main agent runs the full suite and the manual checks.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you changed and added, and what each does;
- the public API, and the crate-private parts the Markdown task will use;
- the loading path: threads, deadlines, limits, and how results reach the component;
- the id allocation and the colour-level rule;
- the commands sent and when;
- every deviation from this brief, and why, and any ink-rs gap;
- the tests you added;
- the exact commands you ran and their results.
