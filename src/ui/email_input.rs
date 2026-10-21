//! Email input with domain completion. Editing uses the shared grapheme kernel
//! instead of Ink UI's UTF-16 reducer and effect-based change callbacks.

use ink_rs::{Callback, Element, Props, Scope};

use super::input::{InputConfig, completion, render_input, use_input};
use super::{InputTheme, use_input_state, use_theme};

/// An uncontrolled email input that completes the domain after the first `@`.
#[derive(Clone, PartialEq, Props)]
#[props(component = email_input)]
pub struct EmailInput {
    /// Ignores input and hides the cursor and suggestion. Defaults to false.
    pub is_disabled: bool,
    /// Text shown while the value is empty. Defaults to an empty string.
    #[props(into)]
    pub placeholder: String,
    /// Value read at mount. Defaults to an empty string.
    #[props(into)]
    pub default_value: String,
    /// Domains searched in order. Defaults to Ink UI's eight email providers.
    pub domains: Vec<String>,
    /// Receives each changed value from the input handler.
    pub on_change: Option<Callback<String>>,
    /// Receives the completed value when Enter is pressed.
    pub on_submit: Option<Callback<String>>,
}

impl Default for EmailInput {
    fn default() -> Self {
        Self {
            is_disabled: false,
            placeholder: String::new(),
            default_value: String::new(),
            domains: [
                "aol.com",
                "gmail.com",
                "yahoo.com",
                "hotmail.com",
                "live.com",
                "outlook.com",
                "icloud.com",
                "hey.com",
            ]
            .map(str::to_owned)
            .into(),
            on_change: None,
            on_submit: None,
        }
    }
}

fn suggestion(value: &str, domains: &[String]) -> Option<String> {
    completion(value.split_once('@')?.1, domains)
}

fn email_input(cx: &mut Scope, props: &EmailInput) -> Element {
    let state = use_input_state(cx, &props.default_value);
    let theme = use_theme::<InputTheme>(cx);
    let domains = props.domains.clone();
    use_input(
        cx,
        state,
        props.is_disabled,
        InputConfig {
            suggestion: move |value: &str| suggestion(value, &domains),
            insert_filter: |value: &str, text: &str| !(value.contains('@') && text.contains('@')),
            on_change: props.on_change.clone(),
            on_submit: props.on_submit.clone(),
        },
    );
    let state = state.get();
    render_input(
        &state,
        props.is_disabled,
        &props.placeholder,
        suggestion(&state.value, &props.domains).as_deref(),
        None,
        &theme,
    )
}
