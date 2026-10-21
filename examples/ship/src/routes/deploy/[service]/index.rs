use eink_rs::{
    router::{use_param, use_router},
    ui::{Select, SelectOption},
};
use ink_rs::{Callback, Element, Props, Scope, Text};

use super::_layout::{Edit, use_draft};
use crate::{
    backend::use_backend,
    routes::{Route, deploy},
    view,
};

/// Version choice; a saved draft remains visible when returning here.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = index)]
pub struct Index {}

fn index(cx: &mut Scope, _: &Index) -> Element {
    let service = use_param::<super::Service>(cx);
    let backend = use_backend(cx);
    let router = use_router::<Route>(cx);
    let draft = use_draft(cx);
    let options = backend
        .0
        .borrow()
        .releases(&service.0)
        .into_iter()
        .map(|release| {
            SelectOption::new(
                format!("{} | {} | {}", release.version, release.age, release.author),
                release.version,
            )
        })
        .collect();
    view::column()
        .child(Text::new(
            "Choose a version (worker v2.0.0 demonstrates failure)",
        ))
        .child(
            Select::default()
                .options(options)
                .on_change(Some(Callback::new(move |version| {
                    draft.edit.call(Edit::Version(version));
                    router.push(Route::Deploy {
                        route: deploy::Route::Service {
                            service: service.clone(),
                            route: super::Route::Regions,
                        },
                    });
                }))),
        )
        .into()
}
