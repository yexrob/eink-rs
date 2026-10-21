use eink_rs::{
    router::use_param,
    ui::{ConfirmInput, TextInput, Variant},
};
use ink_rs::{Callback, Element, Props, Scope, Text};

use crate::{backend::use_backend, view};

/// Validated replica input followed by an explicit confirmation.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = scale)]
pub struct Scale {}

fn scale(cx: &mut Scope, _: &Scale) -> Element {
    let service = use_param::<super::Service>(cx);
    let backend = use_backend(cx);
    let pending = cx.use_state(|| None::<u8>);
    let result = cx.use_state(|| None::<Result<(), String>>);
    let error = cx.use_state(|| None::<String>);
    if let Some(result) = result.get() {
        return view::result_message(&result, "Replica count saved");
    }
    if let Some(replicas) = pending.get() {
        return view::column()
            .child(Text::new(format!(
                "Scale {} to {replicas} replicas?",
                service.0
            )))
            .child(ConfirmInput::new(
                Callback::new(move |()| {
                    result.set(Some(backend.0.borrow_mut().scale(&service.0, replicas)));
                }),
                Callback::new(move |()| pending.set(None)),
            ))
            .into();
    }
    view::column()
        .child(Text::new("Replica count (0-20)"))
        .child(
            TextInput::default()
                .placeholder("3")
                .on_submit(Some(Callback::new(move |value: String| {
                    match value.parse::<u8>() {
                        Ok(value) if value <= 20 => {
                            pending.set(Some(value));
                            error.set(None);
                        }
                        _ => error.set(Some("Enter a whole number from 0 to 20".into())),
                    }
                }))),
        )
        .child(
            error
                .get()
                .map(|error| view::message(Variant::Error, error)),
        )
        .into()
}
