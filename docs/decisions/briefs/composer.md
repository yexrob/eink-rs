# Task: a controlled multi-line Composer

## Goal

bingo, a coding agent built on ink-rs and eink-rs, needs the prompt box of a coding agent, like Claude Code's. The host owns the text: plugins rewrite edits, fill drafts, read the box, hear changes and add typeahead rows. So the component is controlled: text and cursor come in as props, edits go out as callbacks, and the host decides what the box holds next. The requirement from bingo is `<repo>/docs/decisions/briefs/bingo-composer.md`; this brief settles its open points.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/027-composer.md`: the decision and the user's choices.
- `<repo>/docs/design.md`: 「ui」.
- `<repo>/src/ui/input.rs` (`InputState`, `reduce_input`, `use_input`, the grapheme helpers), `<repo>/src/ui/text_input.rs`, `<repo>/src/ui/theme.rs` (`InputTheme`).
- `<repo>/examples/quill/src/composer.rs` and the rest of `examples/quill/`: the app the Composer replaces code in.
- ink-rs, from the path in `[patch.crates-io]` of `<repo>/Cargo.toml`:
  - `Scope::use_node_cursor` (new in 0.1.7): the cursor anchored to a node;
  - `use_input`, `use_paste`, `Key` (the kitty event type), `use_node_ref`, `use_box_metrics`, `use_window_size`, `text_width`;
  - `examples/cursor-ime`.

## Decisions

**Place.** A new `src/ui/composer.rs`, beside `reduce_input`, not an extension of it. The cursor unit differs (bingo wants byte offsets; `InputState` counts graphemes), and changing `InputState` breaks its public API. The two share private grapheme helpers. The theme reuses `InputTheme`.

**Headless reducer.**

```rust
pub struct ComposerState { pub value: String, pub cursor: usize } // byte offset on a grapheme boundary
pub enum ComposerAction {
    Insert(String), Newline, Backspace, Delete,
    Left, Right, WordLeft, WordRight, LineStart, LineEnd, Up, Down,
    DeleteWord, KillToStart, KillToEnd, Yank,
}
pub struct Edit { pub action: ComposerAction, pub before: ComposerState, pub after: ComposerState }
pub enum Direction { Up, Down }
pub struct Reduced { pub state: ComposerState, pub kill: Option<String>, pub boundary: Option<Direction> }
pub fn reduce_composer(state: &ComposerState, action: &ComposerAction, columns: usize, kill: &str) -> Reduced;
pub struct Caret { pub row: usize, pub column: usize }
pub fn composer_caret(state: &ComposerState, columns: usize) -> (usize, Caret); // visual rows, caret
```

`columns` is the wrap width, so Up and Down move by visual line. `boundary` is set when Up or Down is pressed on the first or last visual line; the state does not change then. Names may change; say why.

**Component.**

```rust
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = composer)]
pub struct Composer {
    pub value: String,
    pub cursor: usize,
    pub placeholder: String,
    /// Drawn before the first line; later lines indent by its width.
    pub prompt: String,
    pub is_disabled: bool,
    /// The box grows to this many rows, then scrolls inside itself.
    pub max_rows: usize,
    pub on_edit: Option<Callback<Edit>>,
    pub on_submit: Option<Callback<String>>,
    pub on_history: Option<Callback<Direction>>,
    pub on_escape: Option<Callback>,
    pub on_interrupt: Option<Callback>,
    /// The caret's visual row and column in the box, after layout.
    pub on_caret: Option<Callback<Caret>>,
}
```

**Wrapping.** The component wraps its own text and draws each visual line as its own `Text`, no wider than the width, so the renderer never wraps it again. Rule: add graphemes by `text_width`; a grapheme that does not fit moves to the next line; wide graphemes never split. The width is the measured width minus the prompt's width minus 1, the cell kept for a caret at the line's end. A tab displays as 4 columns; name it as a limit. No ink-rs wrapping primitive is needed: the component owns its lines.

**Width.** The outer `View` takes a node ref; the width comes from `use_box_metrics`. Before the first measurement it is `use_window_size().columns`; ink-rs re-renders on measurement before that frame is painted.

**Cursor.** `use_node_cursor` on the text `View`, at the caret's column (plus the prompt width) and its row inside the visible window. ink-rs resolves it from the frame being painted, so the IME window follows the box when the box moves. Also draw the caret cell in inverse video, for paths that ignore the cursor (string rendering, tests). While `is_disabled`, declare `None` and draw no caret.

**Scrolling.** With `max_rows`, keep the window's first row in a `use_memo` cell and adjust it on each render so the caret row is visible.

**Keys.**

| Key | Action |
|---|---|
| Enter | `\` before the cursor: replace it with a newline. Otherwise `on_submit(value)` |
| Shift+Enter (kitty keyboard), Alt+Enter, Ctrl+J | Newline |
| Left, Right | One grapheme |
| Alt+B, Alt+F, Ctrl+Left, Ctrl+Right | One word: UAX #29 word segments, skipping whitespace and punctuation |
| Ctrl+A, Ctrl+E, Home, End | Logical line start and end |
| Up, Down | Visual line; on the first or last visual line, `on_history` |
| Backspace, Delete | One grapheme |
| Ctrl+W, Ctrl+U, Ctrl+K | Delete the word before the cursor; delete to the logical line start; delete to the logical line end, or the newline when at the end. Each fills the kill buffer |
| Ctrl+Y | Insert the kill buffer |
| Esc | `on_escape` |
| Ctrl+C | `on_interrupt`. Ignore kitty Release events, or a press and its release fire twice |

Ctrl+J arrives as `input == "\n"` with `enter == false`; Enter as `enter == true`. Check both in ink-rs's key parser.

**Paste.** `use_paste` while enabled: the pasted text, with `\r\n` and `\r` turned into `\n`, is one `Insert` edit.

**Controlled contract.**

- Every change, moves included, calls `on_edit` with the before and after states. No call when after equals before.
- The component does not apply the edit. A host that does not take it leaves the props, and the box shows the old text. A host that rewrites `after` gets the rewrite.
- Keys of one input batch are dispatched with no render between them, and a handler reads the props of the last render. Keep an optimistic state in an `Rc<RefCell<…>>` that each key updates and each render overwrites from the props, so the second key of a batch builds on the first and a host rewrite wins at the next render.
- The kill buffer belongs to the component, held across renders; a headless user passes its own to `reduce_composer`.

**Focus.** The Composer is not a focus-ring member and does not call `use_focus`. The host gives it the keys with `is_disabled = false` when no site holds them.

**Host duties**, for the rustdoc: history (`on_history` plus the host's index), completion popups (intercept Up and Down in `on_edit` by `action` while a popup is open, and draw the popup from `on_caret`), Tab (the host's own `use_input`), double Esc.

**quill.** Replace the hand-built editing in `examples/quill/src/composer.rs` with the Composer: its `prefix`, mention workaround, hand-drawn caret and `\n` handling go. Keep quill's behaviour and its tests green.

## Tests

Each covers the place where byte offsets, graphemes and wrapping go wrong, or the controlled contract:

- **Reducer**: one case per key group, with CJK, a ZWJ emoji, Up and Down across wrapped lines, `\` plus Enter, and the history boundary.
- **Render**: growth up to `max_rows`, then the inner scroll keeping the caret row; a wide grapheme at the line's end moving down; `on_caret` row and column.
- **Controlled**: a host rewrite in `on_edit` wins; two keys in one input batch both arrive in order; a host that ignores edits keeps the text.
- **Paste**: multi-line text with `\r\n` gives one `on_edit` with `\n`.
- **Cursor**: through `testing::mount`, the cursor lands on the caret cell and follows when a line above the box grows.

Add nothing beyond these.

## Example

`examples/composer.rs`: a Composer with a prompt, `max_rows` 5, history from submitted lines, and the submitted lines printed above through `Static`. The main agent uses it for the manual check in tmux, including IME. Register it in `Cargo.toml`.

## Out of scope

`docs/`, README, CHANGELOG, the version number. The completion popup. Mouse. The focus ring.

## Rules

- `CLAUDE.md` holds. No `unsafe`.
- `TextInput` and the other components do not change behaviour.
- Code reads like the existing code: naming, comment density, idiom. Public items get rustdoc in plain declarative sentences.
- No new dependencies. `unicode-segmentation` is a dependency already.
- Leave the `[patch.crates-io]` section in `Cargo.toml` as it is; the main agent removes it after the ink-rs release.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into the repository.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --workspace --all-targets -- -D warnings`
- `cargo test --offline --lib`
- `cargo test --offline --test text_input`, plus the test files you add.
- `cargo test --offline -p quill`
- Run only these; the main agent runs the full suite.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you changed and added, and what each does;
- the API, and the key table as built;
- the wrapping rule, the width, the cursor and the scroll;
- the optimistic state and the kill buffer;
- what changed in quill;
- every deviation from this brief, and why, and any ink-rs gap;
- the tests you added;
- the exact commands you ran and their results.
