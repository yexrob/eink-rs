use eink_rs::{
    router::use_param,
    ui::{ConfirmInput, Select, SelectOption},
};
use ink_rs::{Callback, Element, Props, Scope, Text};

use crate::{backend::use_backend, view};

/// A release selector, confirmation, and backend-owned rollback progress.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = rollback)]
pub struct Rollback {}

fn rollback(cx: &mut Scope, _: &Rollback) -> Element {
    let service = use_param::<super::Service>(cx);
    let backend = use_backend(cx);
    let selected = cx.use_state(|| None::<String>);
    let started = cx.use_state(|| None::<Result<u64, String>>);
    if let Some(result) = started.get() {
        return match result {
            Err(error) => view::alert(error),
            Ok(id) => backend
                .0
                .borrow()
                .deployments()
                .iter()
                .find(|d| d.id == id)
                .map(view::deployment_progress)
                .unwrap_or_else(|| view::alert("Deployment not found")),
        };
    }
    if let Some(version) = selected.get() {
        return view::column()
            .child(Text::new(format!("Roll back {} to {version}?", service.0)))
            .child(ConfirmInput::new(
                Callback::new(move |()| {
                    let regions = backend
                        .0
                        .borrow()
                        .regions()
                        .into_iter()
                        .map(|r| r.name)
                        .collect::<Vec<_>>();
                    let result = backend
                        .0
                        .borrow_mut()
                        .deploy(&service.0, &version, &regions);
                    started.set(Some(result));
                }),
                Callback::new(move |()| selected.set(None)),
            ))
            .into();
    }
    let data = backend.0.borrow();
    let current = data.service(&service.0).expect("service layout validation");
    let options = data
        .releases(&service.0)
        .into_iter()
        .skip_while(|r| r.version != current.version)
        .skip(1)
        .map(|release| {
            SelectOption::new(
                format!(
                    "{} | {} ago | {}",
                    release.version, release.age, release.author
                ),
                release.version,
            )
        })
        .collect::<Vec<_>>();
    view::column()
        .child(Text::new("Earlier releases"))
        .child(
            options
                .is_empty()
                .then(|| Text::new("No earlier release available")),
        )
        .child(
            Select::default()
                .options(options)
                .on_change(Some(Callback::new(move |value| selected.set(Some(value))))),
        )
        .into()
}
