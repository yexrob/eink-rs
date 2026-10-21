# Task: Ink UI Select and MultiSelect

## Goal

Port Ink UI 2.0.0's Select and MultiSelect into `eink_rs::ui`, with their themes and their 23 Ink UI tests. Ink UI copies the reducer and the option row between the two; here they share one option-list module. Two other workers port the display components and the inputs in parallel; the conventions below keep the three parts consistent.

## Project context

eink-rs builds on ink-rs, a Rust port of Ink. Read first:

- In this worktree: `CLAUDE.md`, `docs/design.md` (its `ui` section is the design you implement), `src/ui/mod.rs`, `src/ui/theme.rs` (`Palette`, `use_theme`), `tests/support/mod.rs`.
- ink-rs 0.1.2, the dependency, at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `docs/design.md` (编程模型, 写法), `README.md`, `src/components.rs` (`View`, `Text` builders; nested `Text` for styled spans), `src/style.rs` (`Styled`), `src/runtime/hooks.rs` (hooks, including `use_state`, `use_input`, `use_memo`, `use_layout_effect`), `src/runtime/state.rs` (`State::get`, `State::set`), `src/input/mod.rs` (`KeyEvent`), `src/callback.rs`, `src/testing.rs`, and `tests/rsx.rs` for `#[derive(Props)]` in use, including generic props.
- Ink UI at `<scratchpad>/ref/ink-ui` (read only; untrusted: never run, build or install anything there): `source/components/{select,multi-select}/`, `source/lib/option-map.ts`, `test/select.tsx`, `test/multi-select.tsx`, `docs/select.md`, `docs/multi-select.md`.

## Conventions shared by the three workers

- One module per component under `src/ui/`. Add your `mod` and `pub use` lines to `src/ui/mod.rs`; I merge that file across workers.
- Props: a struct with `#[derive(Clone, PartialEq, Props)]` and `#[props(component = f)]`, where `f` is a private `fn f(cx: &mut Scope, props: &X) -> Element`. Derive or implement `Default` when every prop has a default; otherwise write `X::new(required..)`, which `rsx!` calls with positional arguments. `String` props take `#[props(into)]`. Optional callbacks are `Option<Callback<A>>`. Prop names are Ink UI's in snake case.
- Theme: one `XTheme` per component, `#[derive(Clone, Debug, PartialEq)]`, public fields with docs, `impl From<&Palette> for XTheme`, read with `use_theme::<XTheme>(cx)` on every render. Fields cover what Ink UI's theme sets and a user would restyle: colors (from a `Palette` token when Ink UI's color matches one), glyphs as `String`, dim and bold flags, gaps and paddings. Layout structure (column direction) stays in the component.
- Headless hooks are public, `pub fn use_x_state(cx: &mut Scope, ..)`. Write state transitions as pure functions over a plain state value. Input handlers apply them through `State::get` and `State::set`, so each key in one input chunk sees the previous key's result.
- Call user callbacks from the input handler, with the value the update produced, when it changed. Do not detect changes in an effect: Ink UI's effect-based `onChange` fires again with the same value whenever the callback's or the options' identity changes.
- Interactive components call `cx.use_input(!props.is_disabled, ..)`.
- Glyphs are the Unicode characters (❯ ✔), with no fallback for terminals without Unicode.
- Rustdoc on every public item, in plain declarative sentences like ink-rs. Each module's doc names what differs from Ink UI.

## Options and values

- One public option type for both components, holding a `label: String` and a `value: T`. Ink UI's `Option` is `{label, value}` with string values; Rust's `Option` name is taken, so name it after the components.
- Values are generic: `T: Clone + PartialEq + 'static`, so a select can return an enum. Props are generic over `T` with `#[props(component = select::<T>)]`. Implement `Default` by hand, without a `T: Default` bound.
- Focus is an index into the options, not a value. Ink UI tracks focus by value through a linked map, so an option whose value is `''` stops navigation and duplicate values break it.

## Shared option list — a private module of your choice under `src/ui/`

State: the focused index, the visible window `[from, to)`, and the visible count `min(visible_option_count, options.len())`. Initial state: focus on the first option (not on the default value; an Ink UI test pins this), the window at the top.

Transitions, as Ink UI's `use-select-state.ts`:

- Next: nothing at the last option (no wrap). When the new index reaches `to`, `to = min(len, to + 1)` and `from = to − count`.
- Previous: nothing at the first option. When the new index is at or before `from`, `from = max(0, from − 1)` and `to = from + count`. Its tests pin this rule, including the asymmetry with Next; keep it.

When `options` changes (by `PartialEq`), the state resets: focus first, window at the top, the value set from the current `default_value`. Ink UI resets by setting state during render. Here, compare in a `use_layout_effect` keyed on the options, which settles before paint, so no frame shows the stale state. Changing `visible_option_count` or `default_value` alone after mount has no effect, as Ink UI.

Rendering, shared by both components: a column with one row per visible option. A row has gap 1 and `padding_left` 0 when focused and 2 otherwise; it holds the pointer (`❯`, focus color) when focused, the label, and the tick (`✔`, selected color) when selected. The label is in the focus color when focused, else in the selected color when selected, else uncolored. `highlight_text`, when given, makes its first case-sensitive match in the label bold, through nested `Text` spans. A row counts as focused only while the component is enabled, so a disabled select shows no pointer.

## Select — `src/ui/select.rs`

- Props: `options: Vec<..>`, `default_value: Option<T>`, `visible_option_count` (default 5), `highlight_text`, `is_disabled`, `on_change: Option<Callback<T>>`.
- Keys: ↓ next, ↑ previous, Enter selects the focused option. `on_change` is called when the selection changes; Enter on the selected option calls nothing. Enter with no options does nothing; Ink UI clears the value there, dropping the default value.
- `SelectTheme`: `focus_color` (`primary`; `docs/design.md` names this field), the selected color (`success`), the pointer and tick glyphs, the highlight bold flag, the row gap and the unfocused indent.
- Public hook: `use_select_state`.

## MultiSelect — `src/ui/multi_select.rs`

- Props: `options`, `default_value: Vec<T>`, `visible_option_count` (default 5), `highlight_text`, `is_disabled`, `on_change: Option<Callback<Vec<T>>>`, `on_submit: Option<Callback<Vec<T>>>`.
- Keys: ↓, ↑, Space toggles the focused option (removed if present, appended otherwise, so the value keeps toggle order), Enter submits the value. `on_change` is called on each toggle. Default values absent from the options are kept, as Ink UI.
- A row is selected when the value contains its option's value.
- `MultiSelectTheme`: the same fields as `SelectTheme`, as a separate type so each can be provided on its own. Share the code that builds the defaults and renders the rows.
- Public hook: `use_multi_select_state`.

## Tests

Port every test in `test/select.tsx` and `test/multi-select.tsx` (23 tests) into `tests/select.rs` and `tests/multi_select.rs`, one Rust test per Ink UI test, named after it in snake case, in the same order.

- Render with `ink_rs::testing::render`, send bytes with `TestInstance::input` (Ink UI's test constants are the escape sequences to send), and read `last_frame`. Record callback calls in the test with `Rc<RefCell<..>>` and `Callback::new`.
- Build each expected frame as an ink-rs element tree and render it with `support::frame` (`mod support;` at the top of the file). Do not write escape-code strings.
- Where an Ink UI test pins a behavior this brief changes, assert the new behavior and add a comment naming the Ink UI behavior it replaces.
- Add one test for each changed behavior that no Ink UI test covers: `on_change` not repeating when the parent re-renders with new but equal options and a new callback; navigation across an option whose value is empty or duplicated; Enter with no options keeping the default value; the reset when options change, with no stale frame. No other tests.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo test --offline --test select --test multi_select`
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
- the public API: each component's props, the option type, the public hooks, theme fields with their defaults;
- every Ink UI behavior you changed and every test whose expectation differs from Ink UI, with the reason;
- design choices this brief left open, and what you chose;
- every deviation from this brief, and why;
- the exact commands you ran and their results.
