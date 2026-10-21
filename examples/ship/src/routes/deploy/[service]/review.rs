use eink_rs::{
    router::{use_param, use_router},
    ui::{ConfirmInput, Variant},
};
use ink_rs::{Callback, Element, Props, Scope, Text};

use super::_layout::{Edit, use_draft};
use crate::{
    backend::use_backend,
    routes::{Route, deploy},
    view,
};

/// Confirmation starts work in the backend and replaces review with run.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = review)]
pub struct Review {}

fn review(cx: &mut Scope, _: &Review) -> Element {
    let service = use_param::<super::Service>(cx);
    let backend = use_backend(cx);
    let router = use_router::<Route>(cx);
    let draft = use_draft(cx);
    let error = cx.use_state(|| None::<String>);
    let cancel = router.clone();
    let summary = format!(
        "{} / {} / {}",
        service.0,
        draft.version.as_deref().unwrap_or("no version"),
        draft.regions.join(", ")
    );
    view::column()
        .child(Text::new(summary))
        .child(Text::new("Start deployment?"))
        .child(ConfirmInput::new(
            Callback::new(move |()| {
                let result = backend.0.borrow_mut().deploy(
                    &service.0,
                    draft.version.as_deref().unwrap_or(""),
                    &draft.regions,
                );
                match result {
                    Ok(id) => {
                        draft.edit.call(Edit::Started(id));
                        router.replace(Route::Deploy {
                            route: deploy::Route::Service {
                                service: service.clone(),
                                route: super::Route::Run,
                            },
                        });
                    }
                    Err(message) => error.set(Some(message)),
                }
            }),
            Callback::new(move |()| cancel.back()),
        ))
        .child(
            error
                .get()
                .map(|error| view::message(Variant::Error, error)),
        )
        .into()
}
