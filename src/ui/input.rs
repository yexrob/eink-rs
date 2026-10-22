//! Shared text editing kernel. Unlike Ink UI, offsets count grapheme clusters,
//! deletion at the start does nothing, and completion appends at the end.
//! Input callbacks run in event handlers, not effects. Modified text keys do
//! not insert text, and bracketed pastes use a paste hook.

use std::{cell::RefCell, rc::Rc};

use ink_rs::{Callback, Element, Scope, State, Text};
use unicode_segmentation::UnicodeSegmentation;

use super::Palette;

/// Visual choices shared by text, email and password inputs.
#[derive(Clone, Debug, PartialEq)]
pub struct InputTheme {
    /// Inverts the grapheme under the cursor. Defaults to true.
    pub cursor_inverse: bool,
    /// Dims placeholder text outside the cursor. Defaults to true.
    pub placeholder_dim: bool,
    /// Dims completion text outside the cursor. Defaults to true.
    pub suggestion_dim: bool,
    /// Replaces each password grapheme. Defaults to `"*"`.
    pub mask: String,
}

impl From<&Palette> for InputTheme {
    fn from(_: &Palette) -> Self {
        Self {
            cursor_inverse: true,
            placeholder_dim: true,
            suggestion_dim: true,
            mask: "*".into(),
        }
    }
}

/// A text value and a cursor measured in extended grapheme clusters.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct InputState {
    /// The unmasked input value.
    pub value: String,
    /// The zero-based grapheme offset, including the position after the value.
    pub cursor: usize,
}

impl InputState {
    /// Creates a value with its cursor at the end.
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.graphemes(true).count();
        Self { value, cursor }
    }
}

/// An edit applied by [`reduce_input`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InputAction<'a> {
    /// Moves one grapheme toward the start.
    MoveLeft,
    /// Moves one grapheme toward the end.
    MoveRight,
    /// Inserts text at the cursor.
    Insert(&'a str),
    /// Deletes the grapheme before the cursor.
    Delete,
    /// Appends a completion and moves the cursor to the end.
    Complete(&'a str),
}

/// Returns the state after an edit, without effects or input filtering.
/// Out-of-range cursors are clamped to the end before applying the edit.
pub fn reduce_input(state: &InputState, action: InputAction<'_>) -> InputState {
    let mut next = state.clone();
    let offsets = grapheme_offsets(&state.value);
    next.cursor = next.cursor.min(offsets.len() - 1);
    match action {
        InputAction::MoveLeft => next.cursor = next.cursor.saturating_sub(1),
        InputAction::MoveRight => next.cursor = (next.cursor + 1).min(offsets.len() - 1),
        InputAction::Insert(text) => {
            let end = offsets[next.cursor] + text.len();
            next.value.insert_str(offsets[next.cursor], text);
            // An insert can merge with either neighboring grapheme. Place the
            // cursor at the first grapheme boundary after the inserted bytes.
            next.cursor = next
                .value
                .grapheme_indices(true)
                .take_while(|(offset, _)| *offset < end)
                .count();
        }
        InputAction::Delete if next.cursor > 0 => {
            let start = offsets[next.cursor - 1];
            next.value.replace_range(start..offsets[next.cursor], "");
            next.cursor = next
                .value
                .grapheme_indices(true)
                .take_while(|(offset, _)| *offset < start)
                .count();
        }
        InputAction::Delete => {}
        InputAction::Complete(text) => {
            next.value.push_str(text);
            next.cursor = next.value.graphemes(true).count();
        }
    }
    next
}

pub(super) fn grapheme_offsets(value: &str) -> Vec<usize> {
    value
        .grapheme_indices(true)
        .map(|(offset, _)| offset)
        .chain(std::iter::once(value.len()))
        .collect()
}

/// Stores an input value initialized at mount, without binding keys or rendering.
pub fn use_input_state(cx: &mut Scope, default_value: &str) -> State<InputState> {
    cx.use_state(|| InputState::new(default_value))
}

pub(super) struct InputConfig<S, F> {
    pub suggestion: S,
    pub insert_filter: F,
    pub on_change: Option<Callback<String>>,
    pub on_submit: Option<Callback<String>>,
}

#[derive(Clone)]
pub(super) struct InputBuffer {
    state: State<InputState>,
    optimistic: Option<Rc<RefCell<InputState>>>,
}

impl From<State<InputState>> for InputBuffer {
    fn from(state: State<InputState>) -> Self {
        Self {
            state,
            optimistic: None,
        }
    }
}

impl InputBuffer {
    pub fn controlled(state: State<InputState>, optimistic: Rc<RefCell<InputState>>) -> Self {
        Self {
            state,
            optimistic: Some(optimistic),
        }
    }

    fn get(&self) -> InputState {
        self.optimistic
            .as_ref()
            .map_or_else(|| self.state.get(), |value| value.borrow().clone())
    }

    fn set(&self, next: InputState) {
        if let Some(value) = &self.optimistic {
            *value.borrow_mut() = next.clone();
            // Render after a repeated rejected proposal to reset the batch buffer.
            self.state.update(|state| *state = next);
        } else {
            self.state.set(next);
        }
    }
}

fn apply(
    state: &InputBuffer,
    action: InputAction<'_>,
    on_change: &Option<Callback<String>>,
) -> String {
    let previous = state.get();
    let next = reduce_input(&previous, action);
    let changed = previous.value != next.value;
    let value = next.value.clone();
    state.set(next);
    if changed && let Some(callback) = on_change {
        callback.call(value.clone());
    }
    value
}

pub(super) fn use_input<S, F>(
    cx: &mut Scope,
    state: impl Into<InputBuffer>,
    is_disabled: bool,
    config: InputConfig<S, F>,
) where
    S: Fn(&str) -> Option<String> + 'static,
    F: Fn(&str, &str) -> bool + Clone + 'static,
{
    let state = state.into();
    let paste_state = state.clone();
    let paste_filter = config.insert_filter.clone();
    let paste_change = config.on_change.clone();
    cx.use_input(!is_disabled, move |input, key| {
        if key.up_arrow || key.down_arrow || key.tab || (key.ctrl && input == "c") {
            return;
        }
        if key.enter {
            let value = state.get().value;
            let value = if let Some(suggestion) = (config.suggestion)(&value)
                && !suggestion.is_empty()
            {
                apply(
                    &state,
                    InputAction::Complete(&suggestion),
                    &config.on_change,
                )
            } else {
                value
            };
            if let Some(callback) = &config.on_submit {
                callback.call(value);
            }
        } else if key.left_arrow {
            apply(&state, InputAction::MoveLeft, &config.on_change);
        } else if key.right_arrow {
            apply(&state, InputAction::MoveRight, &config.on_change);
        } else if key.backspace || key.delete {
            apply(&state, InputAction::Delete, &config.on_change);
        } else if !key.ctrl
            && !key.meta
            && !input.is_empty()
            && (config.insert_filter)(&state.get().value, input)
        {
            apply(&state, InputAction::Insert(input), &config.on_change);
        }
    });
    cx.use_paste(!is_disabled, move |text| {
        if !text.is_empty() && paste_filter(&paste_state.get().value, text) {
            apply(&paste_state, InputAction::Insert(text), &paste_change);
        }
    });
}

pub(super) fn render_input(
    state: &InputState,
    is_disabled: bool,
    placeholder: &str,
    suggestion: Option<&str>,
    mask: Option<&str>,
    theme: &InputTheme,
) -> Element {
    let cursor = |text: &str| Text::new(text).inverse(theme.cursor_inverse);
    if state.value.is_empty() {
        if is_disabled {
            return Text::new(placeholder)
                .dim_color(theme.placeholder_dim)
                .into();
        }
        let mut graphemes = placeholder.graphemes(true);
        return Text::default()
            .child(cursor(graphemes.next().unwrap_or(" ")))
            .child(Text::new(graphemes.as_str()).dim_color(theme.placeholder_dim))
            .into();
    }
    let mut text = Text::default();
    let mut count = 0;
    for (index, grapheme) in state.value.graphemes(true).enumerate() {
        let value = mask.unwrap_or(grapheme);
        text = text.child(if !is_disabled && index == state.cursor {
            cursor(value)
        } else {
            Text::new(value)
        });
        count += 1;
    }
    if !is_disabled {
        let mut suggestion = suggestion.unwrap_or_default().graphemes(true);
        if state.cursor >= count {
            text = text.child(cursor(suggestion.next().unwrap_or(" ")));
        }
        text = text.child(Text::new(suggestion.as_str()).dim_color(theme.suggestion_dim));
    }
    text.into()
}

pub(super) fn completion(value: &str, suggestions: &[String]) -> Option<String> {
    suggestions
        .iter()
        .find_map(|suggestion| suggestion.strip_prefix(value))
        .filter(|rest| !rest.is_empty())
        .map(str::to_owned)
}
