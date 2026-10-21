use eink_rs::{router::use_param, ui::OrderedList};
use ink_rs::{Element, Props, Scope, Text};

use crate::{backend::use_backend, view};

/// Instance inventory and resource meters.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = index)]
pub struct Index {}

fn index(cx: &mut Scope, _: &Index) -> Element {
    let service = use_param::<super::Service>(cx);
    let backend = use_backend(cx);
    let data = backend.0.borrow();
    let Ok(service) = data.service(&service.0) else {
        return Element::empty();
    };
    view::column()
        .child(
            OrderedList::default().children(data.instances(&service.name).iter().map(|instance| {
                Text::new(format!(
                    "{} | {} | {} | uptime {}m",
                    instance.id,
                    instance.region,
                    instance.health.label(),
                    instance.uptime.as_secs() / 60
                ))
            })),
        )
        .child(view::meter("CPU", service.cpu))
        .child(view::meter("Memory", service.memory))
        .into()
}
