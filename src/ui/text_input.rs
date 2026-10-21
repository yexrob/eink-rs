//! Text input with prefix completion. Editing uses the shared grapheme kernel
//! instead of Ink UI's UTF-16 reducer and effect-based change callbacks.

use ink_rs::{Callback, Element, Props, Scope};

use super::input::{InputConfig, completion, render_input, use_input};
use super::{InputTheme, use_input_state, use_theme};

/// An uncontrolled text input with case-sensitive prefix suggestions.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = text_input)]
pub struct TextInput {
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
    let state = use_input_state(cx, &props.default_value);
    let theme = use_theme::<InputTheme>(cx);
    let suggestions = props.suggestions.clone();
    use_input(
        cx,
        state,
        props.is_disabled,
        InputConfig {
            suggestion: move |value: &str| suggestion(value, &suggestions),
            insert_filter: |_: &str, _: &str| true,
            on_change: props.on_change.clone(),
            on_submit: props.on_submit.clone(),
        },
    );
    let state = state.get();
    render_input(
        &state,
        props.is_disabled,
        &props.placeholder,
        suggestion(&state.value, &props.suggestions).as_deref(),
        None,
        &theme,
    )
}
