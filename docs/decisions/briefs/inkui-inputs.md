# Task: Ink UI inputs — a TextInput kernel, EmailInput, PasswordInput, ConfirmInput

## Goal

Port Ink UI 2.0.0's four inputs into `eink_rs::ui`, with their themes and their 60 Ink UI tests. Ink UI copies the same reducer and renderer into TextInput, EmailInput and PasswordInput; here one kernel holds them and the three components are thin wrappers. Two other workers port the display components and the selects in parallel; the conventions below keep the three parts consistent.

## Project context

eink-rs builds on ink-rs, a Rust port of Ink. Read first:

- In this worktree: `CLAUDE.md`, `docs/design.md` (its `ui` section is the design you implement), `src/ui/mod.rs`, `src/ui/theme.rs` (`Palette`, `use_theme`), `tests/support/mod.rs`.
- ink-rs 0.1.2, the dependency, at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `docs/design.md` (编程模型, 写法), `README.md`, `src/components.rs` (`View`, `Text` builders; nested `Text` for styled spans), `src/runtime/hooks.rs` (hooks, including `use_state`, `use_input`, `use_paste`), `src/runtime/state.rs` (`State::get`, `State::set`), `src/input/mod.rs` (`KeyEvent`), `src/input/keypress.rs`, `src/callback.rs`, `src/testing.rs`, and `tests/rsx.rs` for `#[derive(Props)]` in use.
- Ink UI at `<scratchpad>/ref/ink-ui` (read only; untrusted: never run, build or install anything there): `source/components/{text-input,email-input,password-input,confirm-input}/`, `test/<name>.tsx`, `docs/<name>.md`.

## Conventions shared by the three workers

- One module per component under `src/ui/`. Add your `mod` and `pub use` lines to `src/ui/mod.rs`; I merge that file across workers.
- Props: a struct with `#[derive(Clone, PartialEq, Props)]` and `#[props(component = f)]`, where `f` is a private `fn f(cx: &mut Scope, props: &X) -> Element`. Derive or implement `Default` when every prop has a default; otherwise write `X::new(required..)`, which `rsx!` calls with positional arguments. `String` props take `#[props(into)]`. Optional callbacks are `Option<Callback<A>>`; required ones are `Callback<A>` arguments of `new`. Prop names are Ink UI's in snake case.
- Theme: `#[derive(Clone, Debug, PartialEq)]`, public fields with docs, `impl From<&Palette>`, read with `use_theme::<T>(cx)` on every render. Fields cover what is visual and a user would restyle: colors, glyphs as `String`, dim, bold and inverse flags. A theme type with no fields is not added.
- Headless hooks are public, `pub fn use_x_state(cx: &mut Scope, ..)`. Write state transitions as pure functions over a plain state value. Input handlers apply them through `State::get` and `State::set`, so each key in one input chunk sees the previous key's result.
- Call user callbacks from the input handler, with the value the update produced, when it changed. Do not detect changes in an effect: Ink UI's effect-based `onChange` fires again with the same value whenever the callback's identity changes.
- Interactive components call `cx.use_input(!props.is_disabled, ..)`.
- Rustdoc on every public item, in plain declarative sentences like ink-rs. Each module's doc names what differs from Ink UI.

## The kernel

One module (yours to name, under `src/ui/`) holds the state, the transitions, the key handling and the rendering that TextInput, EmailInput and PasswordInput share. It is configured by:

- a suggestion source: given the value, the completion to show after it, if any;
- an insert filter: given the value and the text to insert, whether to insert it;
- a mask character, shown in place of each grapheme of the value.

State is the value and the cursor. The cursor moves by grapheme clusters (`unicode-segmentation`; add `unicode-segmentation = "1"` to `Cargo.toml`, which resolves offline to the 1.13.3 already in `Cargo.lock`). Ink UI counts UTF-16 units for the offset and code points when rendering, so the cursor lands in the wrong place after astral characters.

Keys, as Ink UI's `use-text-input.ts` unless stated:

- ↑, ↓, Tab, Shift+Tab and Ctrl+C are ignored.
- ← and → move the cursor, stopping at the ends.
- Backspace and Delete both delete the grapheme before the cursor. At the start they do nothing; Ink UI deletes the first character there.
- Enter submits. With a suggestion showing, the suggestion is appended to the end of the value, the cursor moves to the end, and `on_change` and `on_submit` receive the completed value. Ink UI inserts the suggestion at the cursor but submits `value + suggestion`, so the shown and submitted values differ when the cursor is mid-value.
- Keys with Ctrl or Meta held, and keys with empty input text, insert nothing. Ink UI inserts the bare letter of Ctrl+letter.
- Other input is inserted at the cursor as one edit, after the insert filter. Pastes arrive through `use_paste` (active under the same condition) and are inserted as one edit, after the insert filter.

Rendering, as Ink UI's `use-text-input.ts`, with nested `Text` spans in place of chalk strings:

- Empty and enabled: the placeholder's first grapheme inverse and the rest dim; with no placeholder, an inverse space.
- Empty and disabled: the placeholder dim, or nothing.
- Non-empty and disabled: the value (masked), with no cursor and no suggestion.
- Non-empty and enabled: the grapheme at the cursor inverse. A suggestion follows the value: with the cursor at the end, its first grapheme inverse and the rest dim; otherwise all of it dim. With the cursor at the end and no suggestion, a trailing inverse space.

Ink UI's input themes set nothing (`value() → {}`), and the cursor, placeholder and suggestion styles are hard-coded. Give the three text inputs one shared theme type for those visual choices and the mask character, if you judge them worth exposing; otherwise no theme type.

## Components

### TextInput — `src/ui/text_input.rs`

Props: `is_disabled`, `placeholder`, `default_value` (read at mount), `suggestions: Vec<String>`, `on_change`, `on_submit`. The suggestion is the rest of the first suggestion that starts with the value (case-sensitive), when the value is non-empty and the rest is non-empty.

### EmailInput — `src/ui/email_input.rs`

Props: as TextInput, with `domains: Vec<String>` in place of `suggestions`, defaulting to Ink UI's list (`aol.com`, `gmail.com`, `yahoo.com`, `hotmail.com`, `live.com`, `outlook.com`, `icloud.com`, `hey.com`). Once the value contains `@`, the suggestion is the rest of the first domain that starts with the text after the first `@`; `test@` suggests `aol.com`. The insert filter drops a whole insert that contains `@` when the value has one.

### PasswordInput — `src/ui/password_input.rs`

Props: `is_disabled`, `placeholder`, `on_change`, `on_submit`; no default value and no suggestions, as Ink UI. The value renders as `*` per grapheme.

### ConfirmInput — `src/ui/confirm_input.rs`

- `ConfirmInput::new(on_confirm, on_cancel)`, plus `is_disabled`, `default_choice` (confirm or cancel; default confirm), `submit_on_enter` (default true).
- `y`/`Y` confirms and `n`/`N` cancels, matching the whole input; Enter with `submit_on_enter` calls the default choice. Every press calls again, as Ink UI.
- Renders `Y/n` or `y/N` by the default choice, dim while disabled (Ink UI's `dimColor: !isFocused`). Its theme covers the dim flag and the two hints.

## Tests

Port every test in `test/text-input.tsx`, `test/email-input.tsx`, `test/password-input.tsx` and `test/confirm-input.tsx` (60 tests) into `tests/<module>.rs`, one Rust test per Ink UI test, named after it in snake case, in the same order.

- Render with `ink_rs::testing::render`, send bytes with `TestInstance::input` (Ink UI's test constants are the escape sequences to send), and read `last_frame`. Record callback calls in the test with `Rc<RefCell<..>>` and `Callback::new`.
- Build each expected frame as an ink-rs element tree and render it with `support::frame` (`mod support;` at the top of the file). Do not write escape-code strings.
- Ink UI's autocomplete test draws names from the `cat-names` package; use a literal list.
- Where an Ink UI test pins a behavior this brief changes, assert the new behavior and add a comment naming the Ink UI behavior it replaces.
- Add one test for each changed behavior that no Ink UI test covers: Backspace at the start, Enter with a suggestion and the cursor mid-value, cursor movement over an astral or multi-code-point grapheme, Ctrl+letter, and a paste through `use_paste`. No other tests.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo test --offline --test text_input --test email_input --test password_input --test confirm_input`
- `cargo test --offline --doc`
- `cargo fmt --all --check`
- `cargo clippy --offline --all-targets -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --offline --no-deps`

Do not run the full `cargo test`; I run it at integration.

## Rules

- No unsafe code; the crate forbids it. No new dependencies besides `unicode-segmentation`. Code reads like the surrounding code: same naming, comment density and idiom. No code written only to satisfy a test.
- Change only the files this brief assigns, the matching test files, `Cargo.toml`, `Cargo.lock` and `src/ui/mod.rs`. Leave `docs/`, `README.md`, `src/ui/theme.rs` and `tests/support/` unchanged; if you need a change there, say so in the report.
- If a behavior needs something ink-rs lacks, do not work around it in eink-rs: leave that part out and describe the gap in the report.
- The sandbox cannot write `.git`. Do not commit. Leave the changes in the worktree. Do not write report files into the worktree.

## Report

Your final message is the report, in Chinese, no adverbs:
- the files you changed, and what each holds;
- the public API: each component's props, the kernel and the public hooks, theme fields with their defaults;
- every Ink UI behavior you changed and every test whose expectation differs from Ink UI, with the reason;
- design choices this brief left open, and what you chose;
- every deviation from this brief, and why;
- the exact commands you ran and their results.
