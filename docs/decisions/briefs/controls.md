# Task: Button, controlled TextInput and Select, and a focus ring

## Goal

bingo, a coding agent built on ink-rs and eink-rs, renders element trees that its mods and plugins return, after Claude Code's `ui.render`. A press, an input or a pick becomes an event, and the TUI draws the tree that comes back. bingo's own dialogs (the permission prompt, the model picker) use the same controls. The controls are controlled, and the controls of one site (a pane's body, or the band above the prompt) share one focus ring. The requirement from bingo is `<repo>/docs/decisions/briefs/bingo-controls.md`; this brief settles its open points.

## Read first

- `<repo>/CLAUDE.md`.
- `<repo>/docs/decisions/026-controls.md`: the decision, the user's choices and bingo's answers.
- `<repo>/docs/design.md`: 「ui」, in particular 「三层」 and 「主题」.
- `<repo>/src/ui/`: `mod.rs`, `theme.rs`, `input.rs` (`reduce_input`, `use_input`, `apply`), `text_input.rs`, `select.rs`, `option_list.rs` (the window rules that Ink UI's tests fix), `multi_select.rs`, `confirm_input.rs`.
- `<repo>/tests/select.rs`, `<repo>/tests/text_input.rs`, `<repo>/tests/support/mod.rs`.
- ink-rs 0.1.5 or later, in `~/.cargo/registry/src/*/ink-rs-0.1.*/`: `Scope::provide`, `use_context`, `use_state`, `use_layout_effect`, `use_input`, `use_node_ref`, `measure_element`, and how a `View` takes a node ref. ink-rs's own focus (`src/runtime/focus.rs`): app-wide, numbered by registration, Tab and Esc handled before handlers.

## Decisions

### Controlled mode: a `value` prop on the existing components

`value: Option<…>` switches `TextInput` and `Select` to controlled mode; `default_value` is ignored then. No new components: the kernel, theme and tests stay shared. `EmailInput`, `PasswordInput` and `MultiSelect` do not change.

**TextInput**

- The display follows `value`. The cursor stays internal.
- A key computes the proposed value with `reduce_input` and calls `on_change(proposed)`; the component does not apply it. If the host does not take it, the next frame shows the old value and `reduce_input`'s clamp pulls the cursor back.
- Keys of one input batch are dispatched without a render between them, and a handler reads the props of the last render. Keep an optimistic copy (`Rc<RefCell<…>>` or a `State`) that each key updates and each render overwrites from `value`, so the second key of a batch builds on the first.
- When `value` differs from the last proposal (the host replaced it), the cursor moves to the end.
- Add `id: Option<String>`, the ring key.

**Select**

- The selected mark follows `value`; the focused index and the window stay internal. In controlled mode the initial focus is the option holding `value`; uncontrolled stays on the first option, as Ink UI's tests fix.
- New `on_select: Option<Callback<T>>`, called on every Enter, including Enter on the current value (a permission prompt confirms the current option). `on_change` keeps its meaning.
- `number_hotkeys: bool`, false by default: digits 1–9 focus that option and act as Enter on it. The row shows a `1.` prefix, styled from `SelectTheme`. They work while the Select holds the keys.
- `SelectOption` gets `description: Option<String>` and a `.description(…)` builder. It renders dimmed and indented on the line under the label. The window counts options, not rows. `MultiSelect` shares the row rendering and gets it.
- `visible_option_count` exists; no change.
- Add `id: Option<String>`.

### Button

```rust
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = button)]
pub struct Button {
    pub id: String,
    pub label: String,
    /// Wins over `label` when not empty.
    pub children: Vec<Element>,
    /// A digit or lowercase letter that presses the button while its ring holds the keys.
    pub hotkey: Option<char>,
    /// No brackets; `1: label` beside a hotkey.
    pub plain: bool,
    pub variant: ButtonVariant,
    pub dim: bool,
    pub auto_focus: bool,
    pub is_disabled: bool,
    pub on_press: Option<Callback>,
}
pub enum ButtonVariant { #[default] Primary, Secondary }
pub struct ButtonTheme { … }
```

- Drawn as `[ label ]`; focus inverts it. `ButtonTheme` follows the theme rules in `docs/design.md`: `From<&Palette>` with primary from `accent`, secondary with no colour; the bracket strings and the hotkey separator live in the theme.
- Enter or Space presses the focused button. Its hotkey presses it while its ring holds the keys and no text-capturing member is focused.

### FocusRing

ink-rs's focus is app-wide and cannot be scoped to a subtree, so the ring is built here on `provide`, `use_context`, `use_state`, `use_layout_effect` and `use_input`, without `use_focus`. Do not call `use_focus`; say in the docs that an app that mixes the ring with ink-rs focus gets Tab handled twice.

```rust
#[derive(Clone, PartialEq, Props)]
#[props(component = focus_ring)]
pub struct FocusRing {
    pub children: Vec<Element>,
    /// The host's focus: manage it inside the ring, this key, or nothing.
    pub focused: …,
    /// Receives each proposed move.
    pub on_focus: Option<Callback<Option<String>>>,
    pub on_escape: Option<Callback>,
    /// Whether this ring holds the keys. Defaults to true.
    pub is_active: bool,
    /// Whether the arrows walk the ring. Defaults to false.
    pub arrows: bool,
}

/// Joins the nearest ring; returns whether this member holds the keys.
/// Outside a ring it returns true, so a control behaves as it does today.
pub fn use_ring_focus(cx: &mut Scope, item: RingItem) -> bool;

pub struct RingItem {
    pub key: Option<String>,
    pub node: NodeRef,
    pub auto_focus: bool,
    pub claims: Claims,
}
/// The keys a focused member uses for itself.
pub enum Claims { None, Arrows, Text }
```

The shape of `focused` is yours: it must express three states (managed inside, a key, nothing). Say what you chose. The rules, which come from bingo's answers:

- **Keys.** Members are addressed by their string key. A member without a key gets one generated per mount; the host cannot address it. The focused key survives redraws.
- **Order.** Screen position: when a move needs the order, the ring measures each member's node with `measure_element` and walks them top to bottom, then left to right. Controls that appear and go between renders land in place.
- **Every move is a proposal.** Tab, Shift+Tab, the arrows and auto-focus call `on_focus(target)`. With `focused` managed inside, the ring applies the move itself. With `focused` set by the host, the ring moves only when the host changes `focused`; the host may apply, redirect or ignore a proposal, later, after an asynchronous check.
- **A focused member that leaves the tree.** The ring proposes the next member in screen order after the removed member's last measured position, else the previous one, else nothing. Keep each member's last measured position for this.
- **Auto-focus.** When the ring becomes active (mount with `is_active`, or `is_active` turning true) and holds no focused key, it proposes the first `auto_focus` member in screen order. With no `auto_focus` member it proposes nothing: Enter does nothing until Tab, so Enter never acts on a button the author did not mark. An `auto_focus` member that mounts while the ring is active and holds nothing gets the same proposal.
- **Tab and Shift+Tab** walk the ring and wrap. **Esc** calls `on_escape`. Both only while `is_active`.
- **Arrows**: with `arrows` true, Up and Left propose the previous member and Down and Right the next, unless the focused member claims the arrows (`Arrows` for Select, `Text` for TextInput). With `arrows` false the ring leaves the arrows alone; the host scrolls with them.
- **Hotkeys**: while a `Text` member is focused, letter and digit hotkeys of other members are off, so typing `y` does not press a button.
- **Holding keys.** A member reads keys when its ring is active, it is focused, and it is not disabled. The host makes sure one site, or the prompt outside every ring, holds the keys at a time (`is_active` on rings, `is_disabled` on the prompt). Gating reads the last render's props, so the Esc that leaves a site does not reach the prompt in the same batch.
- Host components can call `use_ring_focus` to add stops that are not controls (a close mark, a pane tab); the hook's doc comment shows this.
- `Button`, `TextInput` and `Select` call `use_ring_focus` with their `id`, a node ref on their outer `View`, their `auto_focus` (add the prop to TextInput and Select), and their claims. Outside a ring they behave as today.

## Tests

Each locks a rule above:

- **Ring**: Tab and Shift+Tab wrap; `is_active` false ignores keys; Esc calls `on_escape`; host-set `focused` with `on_focus` proposals, where an ignored proposal leaves focus in place; two rings side by side do not affect each other (subtree scoping).
- **Order and removal**: a member inserted between two others lands between them in Tab order; removing the focused member proposes its neighbour.
- **Auto-focus**: no `auto_focus` member proposes nothing; an `auto_focus` member gets the proposal on activation.
- **Arrows**: with `arrows` true they walk the ring from a Button and stay with a focused Select.
- **Hotkey conflict**: with the TextInput focused, typing a Button's hotkey letter types it and does not press.
- **TextInput controlled**: a host that ignores `on_change` keeps the display; two keys in one input batch both reach `on_change` in order; a host replacement moves the cursor to the end.
- **Select controlled**: `on_select` fires on Enter on the current value; number hotkeys; a description renders under its label and the window counts options.
- **Button**: `[ label ]`, `plain` with a hotkey, the variant colour, focus inversion.
- **Integration**, from bingo's brief: one ring with two Buttons, a TextInput and a Select; Tab walks it, a hotkey presses, and a value change from the host redraws.
- The existing Select and TextInput tests do not change; they show uncontrolled behaviour is the same.

Add nothing beyond these.

## Example

`examples/focus-ring.rs`: a ring with two Buttons, a controlled TextInput and a controlled Select, `arrows` on, the values shown below. The main agent uses it for the manual check. Register it in `Cargo.toml` as the other examples are.

## Out of scope

`docs/`, README, CHANGELOG, the version number. The `Composer` (another worker, outside every ring). Mouse.

## Rules

- `CLAUDE.md` holds. No `unsafe`.
- Behaviour of uncontrolled components outside a ring does not change.
- Code reads like the existing code: naming, comment density, idiom. Public items get rustdoc in plain declarative sentences.
- No new dependencies.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- **The sandbox cannot write `.git`.** Do not commit. Do not write report files into the repository.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo clippy --offline --workspace --all-targets -- -D warnings`
- `cargo test --offline --lib`
- `cargo test --offline --test select --test text_input --test multi_select --test email_input --test password_input`, plus the test files you add.
- Run only these; the main agent runs the full suite.

## Report

Your final message is the report, in Chinese, without adverbs. It covers:

- the files you changed and added, and what each does;
- the API of each component and of the ring, and the shape of `focused`;
- how the ring stores members, measures them, and handles removal;
- the key routing: who reads which key when;
- the optimistic copy in the controlled TextInput;
- every deviation from this brief, and why, and any ink-rs gap;
- the tests you added;
- the exact commands you ran and their results.
