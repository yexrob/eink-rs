use eink_rs::{
    router::use_router,
    ui::{Select, SelectOption},
};
use ink_rs::{Callback, Element, Props, Scope, Text};

use crate::{
    backend::use_backend,
    routes::{Route, deploy},
    view,
};

/// Entry to a service-scoped deployment draft.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = index)]
pub struct Index {}

fn index(cx: &mut Scope, _: &Index) -> Element {
    let backend = use_backend(cx);
    let router = use_router::<Route>(cx);
    let options = backend
        .0
        .borrow()
        .services()
        .into_iter()
        .map(|s| SelectOption::new(&s.name, s.name.clone()))
        .collect();
    view::column()
        .child(Text::new("Deploy / choose service").bold(true))
        .child(
            Select::default()
                .options(options)
                .on_change(Some(Callback::new(move |name| {
                    router.push(Route::Deploy {
                        route: deploy::Route::Service {
                            service: deploy::service::Service(name),
                            route: deploy::service::Route::Index,
                        },
                    });
                }))),
        )
        .into()
}
