//! Password input with one mask per grapheme, rather than Ink UI's UTF-16 unit.
//! Editing and event-based callbacks use the shared text input kernel.

use ink_rs::{Callback, Element, Props, Scope};

use super::input::{InputConfig, render_input, use_input};
use super::{InputTheme, use_input_state, use_theme};

/// An uncontrolled password input whose callbacks receive the unmasked value.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = password_input)]
pub struct PasswordInput {
    /// Ignores input and hides the cursor. Defaults to false.
    pub is_disabled: bool,
    /// Text shown while the value is empty. Defaults to an empty string.
    #[props(into)]
    pub placeholder: String,
    /// Receives each changed value from the input handler.
    pub on_change: Option<Callback<String>>,
    /// Receives the value when Enter is pressed.
    pub on_submit: Option<Callback<String>>,
}

fn password_input(cx: &mut Scope, props: &PasswordInput) -> Element {
    let state = use_input_state(cx, "");
    let theme = use_theme::<InputTheme>(cx);
    use_input(
        cx,
        state,
        props.is_disabled,
        InputConfig {
            suggestion: |_: &str| None,
            insert_filter: |_: &str, _: &str| true,
            on_change: props.on_change.clone(),
            on_submit: props.on_submit.clone(),
        },
    );
    render_input(
        &state.get(),
        props.is_disabled,
        &props.placeholder,
        None,
        Some(&theme.mask),
        &theme,
    )
}
