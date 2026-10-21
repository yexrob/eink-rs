use eink_rs::ui::{EmailInput, PasswordInput};
use ink_rs::{Callback, Element, FocusOptions, Props, Scope, Text};

use crate::{backend::use_backend, view};

/// Two focus-managed fields that save through the backend boundary.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = account)]
pub struct Account {}

fn account(cx: &mut Scope, _: &Account) -> Element {
    let backend = use_backend(cx);
    let values = cx.use_state(|| {
        let mut account = backend.0.borrow().account();
        // PasswordInput starts empty; require a new token on each visit.
        account.token.clear();
        account
    });
    let result = cx.use_state(|| None::<Result<(), String>>);
    let email_focus = cx.use_focus(FocusOptions {
        id: Some("ship-email".into()),
        auto_focus: true,
        ..FocusOptions::default()
    });
    let token_focus = cx.use_focus(FocusOptions {
        id: Some("ship-token".into()),
        ..FocusOptions::default()
    });
    let focus = cx.use_focus_manager();
    let save = cx.use_callback(move |()| {
        result.set(Some(backend.0.borrow_mut().save_account(values.get())))
    });
    view::column()
        .child(Text::new("Account / simulated storage").bold(true))
        .child(Text::new("Email"))
        .child(
            EmailInput::default()
                .default_value(values.get().email)
                .domains(vec![
                    "ship.dev".into(),
                    "example.com".into(),
                    "gmail.com".into(),
                ])
                .is_disabled(!email_focus)
                .on_change(Some(Callback::new(move |email| {
                    values.update(|values| values.email = email)
                })))
                // Enter accepts the domain suggestion, then moves on to the token.
                .on_submit(Some(Callback::new(move |email| {
                    values.update(|values| values.email = email);
                    focus.focus("ship-token");
                }))),
        )
        .child(Text::new("API token"))
        .child(
            PasswordInput::default()
                .placeholder("Token (masked)")
                .is_disabled(!token_focus)
                .on_change(Some(Callback::new(move |token| {
                    values.update(|values| values.token = token)
                })))
                .on_submit(Some(Callback::new(move |token| {
                    values.update(|values| values.token = token);
                    save.call(());
                }))),
        )
        .child(
            result
                .get()
                .map(|result| view::result_message(&result, "Account saved for this session")),
        )
        .into()
}
