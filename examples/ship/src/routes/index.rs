use eink_rs::{
    router::use_router,
    ui::{Alert, Select, SelectOption, Spinner, UnorderedList, Variant},
};
use ink_rs::{Callback, Element, Props, Scope, Text};

use crate::{backend::use_backend, routes::Route, view};

/// Fleet health and navigation through path-valued menu entries.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = index)]
pub struct Index {}

fn index(cx: &mut Scope, _: &Index) -> Element {
    let backend = use_backend(cx);
    let router = use_router::<Route>(cx);
    let data = backend.0.borrow();
    view::column()
        .children(
            data.regions()
                .iter()
                .map(|region| view::meter(&region.name, region.health)),
        )
        .child(if data.syncing() {
            Element::from(Spinner::default().label("Syncing simulated fleet"))
        } else {
            Text::new("Fleet synced").into()
        })
        .child(data.incident().map(|incident| {
            Alert::new(Variant::Warning)
                .title(Some(incident.title))
                .children(vec![Text::new(incident.message)])
        }))
        .child(Text::new("Recent deployments").bold(true))
        .child(
            UnorderedList::default().children(
                data.deployments()
                    .iter()
                    .take(3)
                    .map(|d| Text::new(view::deployment_label(d))),
            ),
        )
        .child(
            Select::default()
                .options(vec![
                    SelectOption::new("Services", "/services".to_owned()),
                    SelectOption::new("Deploy", "/deploy".to_owned()),
                    SelectOption::new("Settings", "/settings".to_owned()),
                ])
                .on_change(Some(Callback::new(move |path: String| {
                    router.push_path(&path).expect("menu route");
                }))),
        )
        .into()
}
