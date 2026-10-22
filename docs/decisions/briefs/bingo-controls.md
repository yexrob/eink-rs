# Task: controlled Button, Input and Select with a shared focus ring

## Goal

bingo renders element trees that mods and plugins return, following Claude Code's `ui.render`: `Box`, `Text`, `Button`, `Input`, `Select`, `Link`, `Code`, `Markdown`. Handlers live with the mod. The TUI turns a press, an input or a pick into an event and draws whatever tree comes back. bingo's own dialogs (the permission prompt, the model picker) use the same controls.

So the controls are controlled (the value comes in through props, events go out through callbacks), and they share one focus ring per site, a site being a pane or the band above the prompt. Tab and the arrows walk the ring, Enter presses, Esc leaves the site.

eink-rs has `TextInput`, `Select` and `ConfirmInput`, all uncontrolled, and no `Button`.

## Semantics to match (Claude Code's elements)

- **Button**:
  - `key`;
  - `label`, or children of text runs;
  - `hotkey`: one digit or lowercase letter that presses it while its site holds the keys;
  - `plain`: no brackets, `1: label` beside a hotkey;
  - `variant`: primary (accent color) or secondary;
  - `dim`, `auto_focus`, `on_press`.
  - Drawn as `[ label ]`; focus inverts it.
- **Input**: `key`, `label`, `placeholder`, `value`, `submit_label`, `auto_focus`; `on_input(value)` on each change and `on_submit(value)` on Enter. Single line.
- **Select**: `key`, `label`, `options: [{value, label}]`, `value`, `auto_focus`, `on_select(value)`.
- **Focus**:
  - a ring over one site's controls, in tree order;
  - `auto_focus` takes the ring on mount;
  - the host can move the ring (`focus(key)`) and hears each move (`on_focus(key)`);
  - Esc calls the site's `on_escape`.

## Read first

- `<repo>/CLAUDE.md`, `<repo>/docs/design.md`.
- The briefs `briefs/inkui-inputs.md` and `briefs/inkui-select.md` and their decision records.
- `<repo>/src/ui/`: `select.rs`, `option_list.rs`, `text_input.rs`, `input.rs`, `confirm_input.rs`, `theme.rs`.
- ink-rs: `use_focus`, `use_focus_manager`, `FocusOptions`, `examples/use-focus-with-id.rs`.

## Change

- Add `ui::Button`.
- Add controlled `TextInput` and `Select`: new components, or a `value` prop that switches the existing ones to controlled mode. Say which and why.
- Add a `FocusRing` (name open) context that scopes focus to a subtree.
- Select:
  - number hotkeys for the first nine options;
  - an optional description per option;
  - `visible_option_count` with scrolling.
- Colors come from `Palette`.

## Tests

- Key-driven tests for each control.
- A ring of two Buttons, an Input and a Select, checking that Tab walks the ring, hotkeys press, and a value change from the host redraws.

## Report

The API, the choice on controlled mode, and the key handling.
