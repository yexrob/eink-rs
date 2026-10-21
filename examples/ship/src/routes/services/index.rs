use eink_rs::{
    router::use_router,
    ui::{SelectOption, TextInput, use_select_state, use_theme},
};
use ink_rs::{Callback, Element, Props, Scope, Styled, Text, View};

use crate::{
    backend::use_backend,
    routes::{Route, services},
    view::{self, ConsoleTheme, HealthBadge},
};

/// A text filter with headless selection and service-specific rows.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = index)]
pub struct Index {}

fn index(cx: &mut Scope, _: &Index) -> Element {
    let backend = use_backend(cx);
    let router = use_router::<Route>(cx);
    let theme = use_theme::<ConsoleTheme>(cx);
    let filter = cx.use_state(String::new);
    let services = backend.0.borrow().services();
    let options: Vec<_> = services
        .iter()
        .filter(|s| s.name.contains(&filter.get()))
        .map(|s| SelectOption::new(&s.name, s.name.clone()))
        .collect();
    let selection = use_select_state(cx, &options, None, 5);
    cx.use_input(true, move |_, key| {
        let mut next = selection.get();
        if key.up_arrow {
            next = next.focus_previous_option();
        }
        if key.down_arrow {
            next = next.focus_next_option();
        }
        if key.enter {
            next = next.select_focused_option();
            if let Some(name) = &next.value {
                router.push(Route::Services {
                    route: services::Route::Service {
                        service: services::service::Service(name.clone()),
                        route: services::service::Route::Index,
                    },
                });
            }
        }
        selection.set(next);
    });
    let selected = selection.get();
    view::column()
        .child(Text::new("Services / filter").bold(true))
        .child(
            TextInput::default()
                .placeholder("Type a service name")
                .on_change(Some(Callback::new(move |value| filter.set(value)))),
        )
        .children(
            selected
                .visible_options()
                .iter()
                .enumerate()
                .filter_map(|(offset, option)| {
                    let service = services.iter().find(|s| s.name == option.value)?;
                    let focused =
                        selected.focused_index() == Some(selected.visible_range().start + offset);
                    Some(
                        View::new()
                            .gap(1)
                            .child(Text::new(if focused { ">" } else { " " }).color(theme.heading))
                            .child(
                                Text::new(format!(
                                    "{} {} | {} replicas",
                                    service.name, service.version, service.replicas
                                ))
                                .bold(focused),
                            )
                            .child(HealthBadge {
                                health: service.health,
                            }),
                    )
                }),
        )
        .child(
            options
                .is_empty()
                .then(|| Text::new("No matching services")),
        )
        .into()
}
