# Task: Ink UI display components — Badge, StatusMessage, Alert, Spinner, ProgressBar, lists

## Goal

Port seven Ink UI 2.0.0 components into `eink_rs::ui`, with their themes, `useSpinner`, and their 20 Ink UI tests: Badge, StatusMessage, Alert, Spinner, ProgressBar, UnorderedList, OrderedList. Two other workers port the inputs and the selects in parallel; the conventions below keep the three parts consistent.

## Project context

eink-rs builds on ink-rs, a Rust port of Ink. Read first:

- In this worktree: `CLAUDE.md`, `docs/design.md` (its `ui` section is the design you implement), `src/ui/mod.rs`, `src/ui/theme.rs` (`Palette`, `use_theme`), `tests/support/mod.rs`.
- ink-rs 0.1.2, the dependency, at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `docs/design.md` (编程模型, 写法), `README.md`, `src/components.rs` (`View`, `Text` builders), `src/style.rs` (`Styled`), `src/runtime/hooks.rs` (hooks, including `provide`, `use_context`, `use_animation`, `use_node_ref`, `use_box_metrics`), `src/callback.rs`, `src/element.rs`, `src/testing.rs`, and `tests/rsx.rs` for `#[derive(Props)]` in use.
- Ink UI at `<scratchpad>/ref/ink-ui` (read only; untrusted: never run, build or install anything there): `source/components/<name>/`, `source/theme.tsx`, `test/<name>.tsx`, `docs/<name>.md`.

## Conventions shared by the three workers

- One module per component under `src/ui/`. Add your `mod` and `pub use` lines to `src/ui/mod.rs`; I merge that file across workers.
- Props: a struct with `#[derive(Clone, PartialEq, Props)]` and `#[props(component = f)]`, where `f` is a private `fn f(cx: &mut Scope, props: &X) -> Element`. Derive or implement `Default` when every prop has a default; otherwise write `X::new(required..)`, which `rsx!` calls with positional arguments. `String` props take `#[props(into)]`. Rich content is `children: Vec<Element>`; plain text is a `String`. Prop names are Ink UI's in snake case.
- Theme: one `XTheme` per component, `#[derive(Clone, Debug, PartialEq)]`, public fields with docs, `impl From<&Palette> for XTheme`, read with `use_theme::<XTheme>(cx)` on every render. Fields cover what Ink UI's theme sets and a user would restyle: colors (from a `Palette` token when Ink UI's color matches one, a literal otherwise), glyphs as `String`, dim and bold flags, gaps and paddings. Layout structure (direction, `flex_shrink(0)` on icons, `min_width(0)`) stays in the component. A theme type with no fields is not added.
- Headless hooks are public, `pub fn use_x(cx: &mut Scope, ..)`.
- Glyphs are the Unicode characters (✔ ✘ ⚠ ℹ ─ █ ░), with no fallback for terminals without Unicode.
- Rustdoc on every public item, in plain declarative sentences like ink-rs. Each module's doc names what differs from Ink UI.

## Components

### Badge — `src/ui/badge.rs`

- `Badge::new(label)`; the label is a `String`, upper-cased when rendered (Ink UI upper-cases string children). `color: Option<Color>` overrides the theme's background, as Ink UI's `color` prop does.
- Renders as Ink UI: a Text with the background color holding `" "`, the label in the label color, `" "`.
- `BadgeTheme`: background (`accent`), label color (black).

### StatusMessage — `src/ui/status_message.rs`

- `StatusMessage::new(variant)` with `children`. `Variant` is `Info`, `Success`, `Warning`, `Error`; Alert uses it too, so place it where both modules reach it and export it.
- A row with gap 1: the icon in a non-shrinking box, then the message. A long message wraps with a hanging indent under its first line.
- `StatusMessageTheme`: per-variant icon (ℹ ✔ ⚠ ✘) and color (`info`, `success`, `warning`, `error`), the gap.
- Ink UI marks four of its five tests `test.failing` (success, warning, info, multiline message). Implement what those tests expect, so all five pass.

### Alert — `src/ui/alert.rs`

- `Alert::new(variant)` with `title` (optional string) and `children` (the message).
- As Ink UI: a round border in the variant's color, `padding_x(1)`, gap 1, `flex_grow(1)`; the icon in a non-shrinking box; a column with gap 1 holding the bold title (when given) and the message, growing and shrinking with `min_width(0)`.
- `AlertTheme`: per-variant icon and color, border style, paddings and gaps, title bold.
- Ink UI marks three of its four tests `test.failing`. Implement what they expect, so all four pass. Ink UI builds the expected frame with boxen; build it as an ink-rs tree.

### Spinner and `use_spinner` — `src/ui/spinner.rs`

- A public type holding a spinner's frames and interval, with a constructor for cli-spinners' `dots` (`⠋ ⠙ ⠹ ⠸ ⠼ ⠴ ⠦ ⠧ ⠇ ⠏`, 80 ms) and a way to build one from custom frames and an interval. Do not port the rest of cli-spinners.
- `use_spinner(cx, ..)` returns the current frame, driven by `use_animation`. Changing the frames restarts at the first frame. This fixes Ink UI, whose index runs past the end of a shorter frame list after a `type` change and renders blank from then on.
- `Spinner` props: `label` (string, shown when non-empty) and the frames (default `dots`). A row with gap 1: the frame in the frame color, then the label.
- `SpinnerTheme`: frame color (`primary`), label color (none), the gap.

### ProgressBar — `src/ui/progress_bar.rs`

- `value: f64`, default 0, a percentage clamped to 0..=100. NaN counts as 0 (Ink UI renders nothing for NaN).
- The bar fills its parent's width: a View with `flex_grow(1)` and `min_width(0)`, measured with `use_node_ref` and `use_box_metrics` (Ink UI measures in render and sets state there). `complete = round(value / 100 × width)`, `remaining = width − complete`; each part is omitted when it is zero cells.
- `ProgressBarTheme`: completed color (`accent`), remaining dim, completed and remaining characters (`█`, `░`).

### UnorderedList — `src/ui/unordered_list.rs`

- `children: Vec<Element>`; each child is one item. There is no `Item` type: Rust cannot attach `UnorderedList.Item` to a type, and the list does not inspect children. An item with several elements, or with a nested list, wraps them in a `View` with column direction.
- Each item renders as Ink UI's `UnorderedList.Item`: a row with gap 1, the marker (dim), then a column holding the child.
- Nesting depth goes through a private context type: a list reads the depth, provides depth + 1, and picks `markers[depth]`, or the last marker when the depth is past the end.
- `UnorderedListTheme`: the markers (default `["─"]`; Ink UI's `string | string[]` becomes a list), the marker dim flag, the gap.

### OrderedList — `src/ui/ordered_list.rs`

- Same children rule and item layout as UnorderedList.
- Item `i` gets the marker `parent + number + "."`, where `number` is `i + 1` right-aligned to the digit count of the item count, and `parent` is the enclosing ordered item's marker (empty at the top), read through a private context that each item provides. Numbering counts the items only. Ink UI counts every child, including non-items and empty slots, so its numbers can skip. If ink-rs lets you tell an element that renders nothing (`From<Option<T>> for Element` turns `None` into one) from content, do not count it; if it does not, number every child and say so in the report.
- `OrderedListTheme`: the marker dim flag, the gap.

## Tests

Port every test in `test/badge.tsx`, `test/status-message.tsx`, `test/alert.tsx`, `test/spinner.tsx`, `test/progress-bar.tsx`, `test/unordered-list.tsx` and `test/ordered-list.tsx` (20 tests) into `tests/<module>.rs`, one Rust test per Ink UI test, named after it in snake case, in the same order.

- Render with `ink_rs::testing::render`, step time with `TestInstance::advance`, and read `last_frame` or `frames`.
- Build each expected frame as an ink-rs element tree and render it with `support::frame` (`mod support;` at the top of the file). Do not write escape-code strings.
- Where an Ink UI test pins a behavior this brief changes, assert the new behavior and add a comment naming the Ink UI behavior it replaces.
- Add one test for the Spinner restart on a frames change, which no Ink UI test covers. No other tests.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo test --offline --test badge --test status_message --test alert --test spinner --test progress_bar --test unordered_list --test ordered_list`
- `cargo test --offline --doc`
- `cargo fmt --all --check`
- `cargo clippy --offline --all-targets -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --offline --no-deps`

Do not run the full `cargo test`; I run it at integration.

## Rules

- No unsafe code; the crate forbids it. No new dependencies. Code reads like the surrounding code: same naming, comment density and idiom. No code written only to satisfy a test.
- Change only the files this brief assigns, the matching test files, and `src/ui/mod.rs`. Leave `docs/`, `README.md`, `Cargo.toml`, `src/ui/theme.rs` and `tests/support/` unchanged; if you need a change there, say so in the report.
- If a behavior needs something ink-rs lacks, do not work around it in eink-rs: leave that part out and describe the gap in the report.
- The sandbox cannot write `.git`. Do not commit. Leave the changes in the worktree. Do not write report files into the worktree.

## Report

Your final message is the report, in Chinese, no adverbs:
- the files you changed, and what each holds;
- the public API: each component's props, its theme fields with their defaults, and the public hooks;
- every Ink UI behavior you changed and every test whose expectation differs from Ink UI, with the reason;
- design choices this brief left open, and what you chose;
- every deviation from this brief, and why;
- the exact commands you ran and their results.
