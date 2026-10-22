# Task: images in Markdown

## Goal

The first step built `eink_rs::image` (commit `0e17f23` in this worktree): loaders, decoding, the kitty encoding and the `Image` component. This task shows images inside Markdown: the `Markdown` component for whole documents, and streamed replies (`use_markdown_stream`, `MarkdownTail`, `MarkdownPiece` and the headless `MarkdownStream`). Terminals with kitty Unicode placeholders show the pixels; everywhere else images stay `[alt]`, as today.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/023-image.md`: every decision below comes from it, including the part added before this task.
- `<repo>/docs/design.md`: the 「markdown」 section (三层, 定稿, 尾部, 发送和唤醒, 高亮).
- `<repo>/src/image/`, all of it, and `<repo>/tests/image.rs` for the `testing::mount` harness with a `TestTerminal` that answers the probe as kitty.
- `<repo>/src/markdown/`, all of it, and `<repo>/tests/markdown.rs`. Note how a highlighted code line waits in `pending` (`Pending::waiting`), how `take_pieces` releases pieces in order, and how the highlight thread wakes the model (`Wake`).
- ink-rs, from the path in `[patch.crates-io]` of `<repo>/Cargo.toml`: `use_graphics`, `Graphics`, `GraphicsSupport`, `use_color_level`, `ColorLevel`, `use_box_metrics`, `NodeRef`, `use_window_size`, `use_context`.

## Decisions

**Feature.** Add `markdown-image = ["markdown", "image"]` to `Cargo.toml`. Without it, Markdown compiles no image code and draws images as `[alt]`, as today. No new dependencies.

**Rendering.** One renderer serves whole documents and stream pieces. It receives the loaded images, by order of appearance in the source it renders, and the available columns. For each image:

- Not loaded, failed, or no image support: `[alt]`, as today.
- An image alone in its paragraph is a block. Its columns are at most the available columns minus the list and quote indentation at that point; its rows are at most 20; it is never scaled past its natural size in cells (`size_in_cells`; it handles the unknown cell size).
- An inline image whose natural height at those columns is at most one and a half rows shrinks to one row and sits in the line as placeholder cells, keeping the surrounding link. A taller inline image splits its paragraph: the text before it, the image as a block, the text after it, each wrapping on its own.
- Images in table cells stay `[alt]`: the column width decides the cell, and placeholders need their rows and columns.

**Whole documents (`Markdown`).** Find the images in the source, load them with `use_images`, render with the results, and send the placed ones with `use_image_effect`. The columns come from `use_box_metrics` on the component's root view, with the window columns until the first measurement. Nothing loads when the support is not `Kitty` or the colour level is below `Ansi256`.

**Streams: the model loads.** The user chose this: the stream loads its images the way it highlights code.

```rust
/// How a stream shows images. `Alt` by default.
#[derive(Clone)]
pub enum MarkdownImages {
    /// Images show as `[alt]`; nothing loads.
    Alt,
    /// The terminal's support is not known yet; pieces with images wait.
    Probing,
    /// Images load through these loaders and show with kitty placeholders.
    Kitty { loaders: ImageLoaders, cell_pixels: Option<(u16, u16)>, color_level: ColorLevel },
}

impl MarkdownStream {
    pub fn set_images(&mut self, images: MarkdownImages);
    /// The columns available to images in later pieces; 80 by default.
    pub fn set_columns(&mut self, columns: usize);
    /// Waits for one image result; false when no image is pending.
    pub fn wait_for_images(&mut self) -> bool;
}
```

- When a block becomes a document piece and holds images, the stream starts one load per image with the environment and columns of that moment, and the piece waits in `pending` the way a highlighted line does. Pieces after it wait behind it. When each of its images is ready or failed, the piece is rebuilt with them and `take_pieces` releases it.
- Under `Probing`, a piece with images waits and nothing loads. When the setting changes to `Alt`, waiting pieces are released with `[alt]`; when it changes to `Kitty`, their loads start.
- Loads run off the UI thread. Move the load thread, the job and its deadline out of `image/hook.rs` into a function that does not need a `Scope`; `use_images` and the stream both call it. The stream's completion sends on a channel and calls the wake, as `markdown-highlight` does; `take_pieces` polls the channel.
- Deadlines are 5 s local and 10 s network, as in the first step, measured on the stream's clock: the time given to `tick`. An overdue image fails and shows `[alt]`. `use_markdown_stream` keeps its animation clock running while a piece waits on an image, so deadlines pass without new text.
- `wait_for_images` blocks for one result, at most until the earliest deadline by wall time; overdue images then fail. It mirrors `wait_for_highlights` for hosts without a UI.
- A piece carries its loaded images and their sizes; rendering it loads nothing. The piece component sends its images with `use_image_effect`. A `Static` item runs its effect after it is written; placeholders reach the terminal before the data, and kitty and Ghostty draw when the data arrives.
- `use_markdown_stream` reads `use_graphics`, `use_color_level` and the `ImageLoaders` context (the default when none is provided) and calls `set_images` when they change: `Probing` while the probe runs; `Kitty` when the support is `Kitty` and the level is `TrueColor` or `Ansi256`; `Alt` otherwise.
- `MarkdownTail` measures its own width with `use_box_metrics` and gives it to the stream with `set_columns`; until the first measurement it gives the window columns. The tail draws images in the unfinished block and in waiting pieces as `[alt]`.
- With the same images and columns, the streamed pieces render the same as the whole document. `pieces_equal_document` keeps passing unchanged: there, images stay `[alt]`.
- Each image occurrence loads on its own, as in the first step.

**Example.** With `markdown-image`, `examples/markdown.rs --file PATH` provides `ImageLoaders` whose local loader is `FileLoader::new` of the file's directory, so a README's relative images load. No new example.

## Tests

Each covers new logic. Put them in one new file, `tests/markdown_image.rs`, compiled only with `markdown-image`:

- the renderer: a block image; a small inline image as one row inside its text; a tall inline image splitting its paragraph; an image in a list item with the item's indentation taken off the columns; an image in a table as `[alt]`;
- the stream with a test loader under `Kitty`: a text piece after an image piece is not released before it; after the load, the pieces render the same as the whole document with the same images;
- a loader that never returns: the piece is released with `[alt]` after 5 s of `tick`;
- `Probing` then `Alt`: the waiting piece is released with `[alt]` and nothing loads;
- the hook path through `testing::mount` with a kitty `TestTerminal`: a streamed image piece in `Static` transmits its data once.

Use small images built in the tests; add no binary fixtures. Add nothing beyond these.

## Out of scope

`docs/`, README, CHANGELOG, the version number. Video, animation, half blocks, deleting images. Changes to the first step's behaviour beyond moving the load function.

## Rules

- `CLAUDE.md` holds. No `unsafe`.
- Code reads like the existing code: naming, comment density, idiom. Public items get rustdoc in plain declarative sentences without adverbs.
- Tests use a test loader and never the network.
- Leave the `[patch.crates-io]` section in `Cargo.toml` as it is.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into the repository.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox. If cargo cannot write its target directory, set `CARGO_TARGET_DIR` to a directory under `/private/tmp` and remove it at the end.
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --workspace --all-targets -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features markdown -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features markdown-image -- -D warnings`
- `cargo test --offline --lib --all-features`
- `cargo test --offline --features markdown-image --test markdown_image`
- `cargo test --offline --features markdown --test markdown`
- `cargo test --offline --features image --test image`
- Run only these; the main agent runs the full suite and the manual checks.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you changed and added, and what each does;
- the public API added;
- the rendering rules as built: block, inline, split, indentation, tables;
- the stream path: when loads start, how pieces wait and are released, deadlines, the wake, `wait_for_images`;
- what moved out of `image/hook.rs`;
- every deviation from this brief, and why, and any ink-rs gap;
- the tests you added;
- the exact commands you ran and their results.
