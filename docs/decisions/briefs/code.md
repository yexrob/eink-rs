# Task: a `Code` component with line numbers and unified diffs

## Goal

bingo, a coding agent built on ink-rs and eink-rs, renders Claude Code's element vocabulary, which has `Code { source, language?, path?, startLine?, format: source | diff, wrap: wrap | truncate-end }`. Tool rows show file reads with line numbers and edits as unified diffs. eink-rs highlights code inside Markdown, but has no public component for it. The requirement from bingo is `<repo>/docs/decisions/briefs/bingo-code.md`; this brief settles its open points.

The work has two parts. **Do part 1, run the commands, and end your turn with a short report on part 1.** Part 2 comes in a follow-up message after the main agent commits part 1.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/028-code.md`: the decision and the user's choices.
- `<repo>/docs/design.md`: 「ui」 and 「高亮」.
- `<repo>/docs/decisions/019-pierre-highlight-research.md` and `021-highlight-long-lines-and-reset.md`.
- `<repo>/src/markdown/highlight.rs`: `highlight`, `Cache`, `MAX_LINE`, `Work`, `submit`, the thread, `text`.
- `<repo>/src/markdown/hook.rs`: `Wake` and how a result wakes the app.
- `<repo>/src/markdown/render.rs` and `stream.rs`: every call into `highlight`.
- `<repo>/src/markdown/theme.rs`, `<repo>/src/ui/theme.rs` (`Palette`, `use_theme`, the three-level lookup).
- `<repo>/Cargo.toml`: the features.
- ink-rs, from the path in `[patch.crates-io]` of `<repo>/Cargo.toml`: `Scope::use_color_level` (new in 0.1.7), `TextWrap`, `View`, `Text`, `use_effect`, `use_memo`, `AppHandle::update`.

## Part 1: share the highlighter

A pure refactor; behaviour does not change.

- Move `src/markdown/highlight.rs` to `src/highlight.rs`, `pub(crate)`, under `#[cfg(feature = "highlight")]`.
- Features:

  ```toml
  highlight = ["dep:syntect", "dep:two-face"]
  markdown-highlight = ["markdown", "highlight"]
  ```

  `markdown-highlight` keeps its name and meaning for users.
- `Work::Line`'s wake becomes `Option<Arc<dyn Fn() + Send + Sync>>`; Markdown's `Wake` turns itself into one.
- The language becomes `enum Lang { Token(String), File(String) }`. `File` takes a file name and looks it up with `find_syntax_by_extension` (check that it matches both extensions and whole names such as `Makefile`); on a miss it tries the extension as a token. Markdown uses `Token`.
- Block ids come from one global atomic counter, so another component's blocks never collide with a stream's.

Commands for part 1: the list below, and `cargo test --offline --features markdown-highlight --test markdown`.

## Part 2: the `Code` component

**Place.** `ui::Code`, in `src/ui/code.rs`, with the diff parser in `src/ui/diff.rs` (`pub(crate)`). Without the `highlight` feature, `Code` works in plain colours, diffs included. With it, `Code` and Markdown share the one highlighter.

**API.**

```rust
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = code)]
pub struct Code {
    #[props(into)] pub source: Rc<str>,
    #[props(into)] pub language: Option<String>,
    /// Gives the language from the file name when `language` is `None`.
    #[props(into)] pub path: Option<String>,
    /// Draws a line-number gutter starting at this number. Diffs number from their hunk headers.
    pub start_line: Option<usize>,
    pub format: CodeFormat, // Source (default) | Diff
    pub wrap: CodeWrap,     // Wrap (default) | TruncateEnd
    /// Called once per source when a diff does not parse and is drawn as plain code.
    pub on_fallback: Option<Callback<DiffError>>,
}

pub struct CodeTheme {
    pub gutter_color: Color,
    pub added: Color,
    pub removed: Color,
    pub added_background: Option<Color>,
    pub removed_background: Option<Color>,
    pub code_color: Color,
    /// A bat theme name; `ansi` by default.
    pub theme: String,
}
```

`CodeTheme` follows the theme rules in `docs/design.md`: `From<&Palette>` with `added` from `success` and `removed` from `error`; the backgrounds default to a dark green and a dark red `Rgb`.

**Colour level.** Read `use_color_level()`. Background tints are drawn at `TrueColor` and `Ansi256`. Below that, rows get no background, and the `+` and `-` markers carry the meaning in the `added` and `removed` foreground colours.

**Layout.**

- One row `View` per source line: a gutter `Text` (its width from the last number's digits, `flex_shrink(0)`) and a content `View` (`flex_grow(1)`, `min_width(0)`).
- Continuation rows of a wrapped line have no number; the gutter stays blank.
- A diff row `View` carries the background, so continuation rows keep the tint.
- `Wrap` breaks long lines and keeps them readable; check whether `TextWrap::Hard` or `TextWrap::Wrap` keeps leading indentation better and say which you chose. `TruncateEnd` uses `TextWrap::TruncateEnd`.

**Highlighting.**

- A source of 64 lines or fewer is highlighted on the render thread with the line `Cache`, so a host that puts the row into `Static` gets colours in the first frame.
- A larger source draws in plain colour first. Its lines go to the background thread, and results come back through `State` and a redraw.
- The component holds an `Arc<()>` in `use_memo`; after unmount the thread skips the remaining lines, as Markdown streams do.
- `MAX_LINE` and the long-line rule apply as in Markdown.

**Diff.**

- Own parser, about 120 lines. Off-the-shelf crates expect whole patches and fail on cut ones.
- Recognised: `@@ -a,b +c,d @@` headers; lines starting with ` `, `-`, `+`; `\ No newline at end of file`, drawn dim; `diff --git`, `---` and `+++` headers before the first hunk, drawn dim.
- The gutter has an old and a new number column. Removed lines show the old number, added lines the new one, and context lines both.
- Context lines are dimmed; removed and added lines get their marker, foreground and tint.
- **Highlighting across lines**: rebuild each hunk into its old text (context and removed lines) and its new text (context and added lines), highlight each as one block, and map the results back: removed lines from the old block, added and context lines from the new one. Multi-line comments inside a hunk stay right; the state before a hunk is unknown.
- **Fallback**: no `@@` header, a hunk whose line counts differ from its header (a cut hunk included), or an unknown line prefix. The source is drawn as plain code, numbered from `start_line`. `on_fallback` runs once per source, from `use_effect` keyed by the source, never during render.

**Markdown fences do not use `Code`.** Stacked stream pieces must equal the whole-document render, and fences have their own label, padding and per-line pieces. They share the highlighter, no more.

## Tests

Each covers new logic:

- a source with a gutter: number width and alignment;
- a two-hunk diff: numbers from the headers, markers, dim context;
- `Wrap` and `TruncateEnd` at a narrow width: continuation rows have no number, truncation ends with the ellipsis;
- broken diffs: no `@@`, and a cut hunk; each draws as plain code and calls `on_fallback` once;
- with `highlight`: the hunk rebuild keeps a multi-line comment's colour on old and new lines, tested on the rebuild function, not through the thread;
- the colour level: `Basic` draws no backgrounds.

Add nothing beyond these. `pieces_equal_document` covers the shared highlighter for Markdown.

## Example

`examples/code.rs`: a Rust source with a gutter, then a diff of it, at the window width. Register it in `Cargo.toml` with `required-features = ["highlight"]`.

## Out of scope

`docs/`, README, CHANGELOG, the version number. Side-by-side diffs. Word-level diff highlighting.

## Rules

- `CLAUDE.md` holds. No `unsafe`.
- Markdown's behaviour does not change; `tests/markdown.rs` is the check.
- Code reads like the existing code: naming, comment density, idiom. Public items get rustdoc in plain declarative sentences.
- No new dependencies.
- Leave the `[patch.crates-io]` section in `Cargo.toml` as it is; the main agent removes it after the ink-rs release.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into the repository.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --workspace --all-targets -- -D warnings`
- `cargo clippy --offline --workspace --all-targets --features highlight -- -D warnings`
- `cargo test --offline --lib --all-features`
- `cargo test --offline --features markdown-highlight --test markdown`, plus the test files you add, with and without `highlight`.
- Run only these; the main agent runs the full suite and measures a 2,000-line source.

## Report

Your final message for each part is its report, in Chinese, without adverbs. Part 2's covers:

- the files you changed and added, and what each does;
- the API and the theme;
- the layout, the wrap choice, and the colour-level rule;
- the sync threshold, the background path and unmount;
- the diff parser, the numbering, the hunk rebuild, and the fallback rules;
- every deviation from this brief, and why, and any ink-rs gap;
- the tests you added;
- the exact commands you ran and their results.
