use eink_rs::{
    router::{use_param, use_router},
    ui::{MultiSelect, SelectOption, Variant},
};
use ink_rs::{Callback, Element, Props, Scope, Text};

use super::_layout::{Edit, use_draft};
use crate::{
    backend::use_backend,
    routes::{Route, deploy},
    view,
};

/// Multi-region selection with validation before advancing to review.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = regions)]
pub struct Regions {}

fn regions(cx: &mut Scope, _: &Regions) -> Element {
    let service = use_param::<super::Service>(cx);
    let backend = use_backend(cx);
    let router = use_router::<Route>(cx);
    let draft = use_draft(cx);
    let error = cx.use_state(|| false);
    let options = backend
        .0
        .borrow()
        .regions()
        .into_iter()
        .map(|r| SelectOption::new(&r.name, r.name.clone()))
        .collect();
    let change = draft.edit.clone();
    view::column()
        .child(Text::new("Choose at least one region"))
        .child(
            MultiSelect::default()
                .options(options)
                .default_value(draft.regions)
                .on_change(Some(Callback::new(move |regions| {
                    change.call(Edit::Regions(regions))
                })))
                .on_submit(Some(Callback::new(move |regions: Vec<String>| {
                    if regions.is_empty() {
                        error.set(true);
                        return;
                    }
                    draft.edit.call(Edit::Regions(regions));
                    router.push(Route::Deploy {
                        route: deploy::Route::Service {
                            service: service.clone(),
                            route: super::Route::Review,
                        },
                    });
                }))),
        )
        .child(
            error
                .get()
                .then(|| view::message(Variant::Error, "Select a region with Space")),
        )
        .into()
}
