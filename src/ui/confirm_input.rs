//! Stateless confirmation input. Unlike Ink UI, its hints are theme fields.

use ink_rs::{Callback, Element, Props, Scope, Text};

use super::{Palette, use_theme};

/// The choice submitted by Enter in a confirmation input.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ConfirmChoice {
    /// Calls the confirmation callback.
    #[default]
    Confirm,
    /// Calls the cancellation callback.
    Cancel,
}

/// Visual choices for a confirmation input.
#[derive(Clone, Debug, PartialEq)]
pub struct ConfirmInputTheme {
    /// Dims the hint while disabled. Defaults to true.
    pub disabled_dim: bool,
    /// Hint when Enter confirms. Defaults to `"Y/n"`.
    pub confirm_hint: String,
    /// Hint when Enter cancels. Defaults to `"y/N"`.
    pub cancel_hint: String,
}

impl From<&Palette> for ConfirmInputTheme {
    fn from(_: &Palette) -> Self {
        Self {
            disabled_dim: true,
            confirm_hint: "Y/n".into(),
            cancel_hint: "y/N".into(),
        }
    }
}

/// A yes/no input that invokes a callback for each matching input event.
#[derive(Clone, PartialEq, Props)]
#[props(component = confirm_input)]
pub struct ConfirmInput {
    /// Ignores input while disabled. Defaults to false.
    pub is_disabled: bool,
    /// The choice for Enter and the displayed hint. Defaults to confirm.
    pub default_choice: ConfirmChoice,
    /// Enables Enter to submit the default choice. Defaults to true.
    pub submit_on_enter: bool,
    /// Runs for `y`, `Y`, or Enter with the confirm default.
    pub on_confirm: Callback,
    /// Runs for `n`, `N`, or Enter with the cancel default.
    pub on_cancel: Callback,
}

impl ConfirmInput {
    /// Creates a confirmation input with the required callbacks.
    pub fn new(on_confirm: Callback, on_cancel: Callback) -> Self {
        Self {
            is_disabled: false,
            default_choice: ConfirmChoice::Confirm,
            submit_on_enter: true,
            on_confirm,
            on_cancel,
        }
    }
}

fn confirm_input(cx: &mut Scope, props: &ConfirmInput) -> Element {
    let theme = use_theme::<ConfirmInputTheme>(cx);
    let props = props.clone();
    let hint = match props.default_choice {
        ConfirmChoice::Confirm => theme.confirm_hint,
        ConfirmChoice::Cancel => theme.cancel_hint,
    };
    let dim = props.is_disabled && theme.disabled_dim;
    cx.use_input(!props.is_disabled, move |input, key| {
        let choice = match input {
            "y" | "Y" => Some(ConfirmChoice::Confirm),
            "n" | "N" => Some(ConfirmChoice::Cancel),
            _ if key.enter && props.submit_on_enter => Some(props.default_choice),
            _ => None,
        };
        match choice {
            Some(ConfirmChoice::Confirm) => props.on_confirm.call(()),
            Some(ConfirmChoice::Cancel) => props.on_cancel.call(()),
            None => {}
        }
    });
    Text::new(hint).dim_color(dim).into()
}
