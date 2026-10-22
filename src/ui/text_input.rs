//! Text input with prefix completion. Editing uses the shared grapheme kernel
//! instead of Ink UI's UTF-16 reducer and effect-based change callbacks.

use std::{cell::RefCell, rc::Rc};

use ink_rs::{Callback, Element, Props, Scope, View};

use super::input::{InputBuffer, InputConfig, completion, render_input, use_input};
use super::{Claims, InputState, InputTheme, RingItem, use_input_state, use_ring_focus, use_theme};

/// A text input with case-sensitive prefix suggestions and optional host control.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = text_input)]
pub struct TextInput {
    /// The member key in the nearest ring. Defaults to a generated per-mount key.
    pub id: Option<String>,
    /// Requests focus when its active ring holds no key. Defaults to false.
    pub auto_focus: bool,
    /// Host-owned value. When present, ignores `default_value`.
    pub value: Option<String>,
    /// Ignores input and hides the cursor and suggestion. Defaults to false.
    pub is_disabled: bool,
    /// Text shown while the value is empty. Defaults to an empty string.
    #[props(into)]
    pub placeholder: String,
    /// Value read at mount. Defaults to an empty string.
    #[props(into)]
    pub default_value: String,
    /// Candidates searched in order. Defaults to an empty list.
    pub suggestions: Vec<String>,
    /// Receives each changed value from the input handler.
    pub on_change: Option<Callback<String>>,
    /// Receives the completed value when Enter is pressed.
    pub on_submit: Option<Callback<String>>,
}

fn suggestion(value: &str, suggestions: &[String]) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        completion(value, suggestions)
    }
}

fn text_input(cx: &mut Scope, props: &TextInput) -> Element {
    let node = cx.use_node_ref();
    let focused = use_ring_focus(
        cx,
        RingItem {
            key: props.id.clone(),
            node,
            auto_focus: props.auto_focus,
            claims: Claims::Text,
        },
    );
    let is_disabled = props.is_disabled || !focused;
    let state = use_input_state(cx, props.value.as_deref().unwrap_or(&props.default_value));
    // The last proposal, and the state the last render showed.
    let optimistic = cx.use_state(|| Rc::new(RefCell::new(state.get()))).get();
    let shown = cx.use_state(|| Rc::new(RefCell::new(state.get()))).get();
    let buffer = if let Some(value) = &props.value {
        let mut proposed = optimistic.borrow_mut();
        let mut shown = shown.borrow_mut();
        if proposed.value == *value {
            *shown = proposed.clone();
        } else if shown.value == *value {
            // The host ignored the proposal; the cursor stays where it was.
            *proposed = shown.clone();
        } else {
            *shown = InputState::new(value);
            *proposed = shown.clone();
        }
        drop(proposed);
        InputBuffer::controlled(state, optimistic.clone())
    } else {
        state.into()
    };
    let theme = use_theme::<InputTheme>(cx);
    let suggestions = props.suggestions.clone();
    use_input(
        cx,
        buffer,
        is_disabled,
        InputConfig {
            suggestion: move |value: &str| suggestion(value, &suggestions),
            insert_filter: |_: &str, _: &str| true,
            on_change: props.on_change.clone(),
            on_submit: props.on_submit.clone(),
        },
    );
    let state = if props.value.is_some() {
        optimistic.borrow().clone()
    } else {
        state.get()
    };
    View::new()
        .node_ref(node)
        .child(render_input(
            &state,
            is_disabled,
            &props.placeholder,
            suggestion(&state.value, &props.suggestions).as_deref(),
            None,
            &theme,
        ))
        .into()
}
