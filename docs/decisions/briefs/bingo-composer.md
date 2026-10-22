# Task: a multi-line prompt composer

## Goal

bingo's TUI needs the input box of a coding agent, like Claude Code's. The host owns the box's text. Plugins rewrite edits (`prompt.edit`), fill drafts (`prompt.fill`), read the box (`prompt.read`), hear changes (`prompt.change`) and add typeahead rows (`prompt.autocomplete`). So the component is controlled: text and cursor come in as props, edits go out as callbacks, and the host decides what the box holds next.

`ui::TextInput` is single-line and uncontrolled. `examples/quill/src/composer.rs` builds history and completion popups on `use_input_state` and `reduce_input`, as example code.

## Read first

- `<repo>/CLAUDE.md`, `<repo>/docs/design.md`.
- `<repo>/src/ui/input.rs` (`InputState`, `InputAction`, `reduce_input`, `use_input_state`) and `<repo>/src/ui/text_input.rs`.
- `<repo>/examples/quill/src/composer.rs`.
- ink-rs: `use_input`, `use_paste`, `use_cursor` and `examples/cursor-ime`, `use_focus`, `text_width`.

## Change

A `Composer` component (name open) in `ui`, with a headless reducer beside it so a host can apply edits without the view.

- Props:
  - `value: String`;
  - `cursor: usize`, a byte offset on a grapheme boundary;
  - `placeholder`, `is_disabled`, a prompt prefix such as `> `;
  - `max_rows`: the box grows with its text up to this, then scrolls inside itself and keeps the cursor row in view.
- Callbacks:
  - `on_edit(Edit)`: the key, the text and cursor before, and the text and cursor the reducer proposes. The host may pass a different result back through the props.
  - `on_submit(String)`;
  - `on_history(Direction)` when Up or Down is pressed on the first or last visual line;
  - `on_escape`;
  - `on_interrupt` for Ctrl+C, with `exit_on_ctrl_c` off.
- Editing, safe for graphemes and wide characters:
  - Left and Right by grapheme; Alt+B, Alt+F, Ctrl+Left and Ctrl+Right by word; Ctrl+A, Ctrl+E, Home and End to the line's start and end; Up and Down by visual (wrapped) line.
  - Backspace and Delete; Ctrl+W deletes the word before the cursor, Ctrl+U to the line's start, Ctrl+K to the line's end; Ctrl+Y yanks the last kill.
  - Newline: Shift+Enter (with the kitty keyboard), Ctrl+J, and a backslash before Enter. Enter submits.
  - Bracketed paste inserts the text as one edit (`use_paste`), newlines kept.
- The terminal cursor sits at the caret through `use_cursor`, so IME candidate windows anchor there. Keep the inverse-video caret as a fallback when the cursor is hidden.
- A completion popup is out of scope. Expose the caret's row and column in the box so a host can draw one under it.

## Tests

- Reducer tests for each key group, with CJK and emoji graphemes and wrapped lines.
- A render test for growth up to `max_rows` and the inner scroll.
- A test where the host rewrites the value in `on_edit` and the rewrite wins.

## Docs

A decision record, the component docs, and an example.

## Report

The API, the key table, and what a host does for history and completion.
