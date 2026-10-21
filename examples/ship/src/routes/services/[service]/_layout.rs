use eink_rs::router::{Outlet, use_param, use_router};
use ink_rs::{Element, Props, Scope, Styled, Text, View};

use crate::{
    backend::use_backend,
    routes::{Route, deploy, services},
    view::{self, HealthBadge},
};

/// A service header whose pin state survives tab replacements.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = layout)]
pub struct Layout {}

fn layout(cx: &mut Scope, _: &Layout) -> Element {
    let service = use_param::<super::Service>(cx);
    let backend = use_backend(cx);
    let router = use_router::<Route>(cx);
    let pinned = cx.use_state(|| false);
    let current = match router.route() {
        Route::Services {
            route: services::Route::Service { route, .. },
        } => route.clone(),
        _ => unreachable!("service layout"),
    };
    let tab = match current {
        super::Route::Index => 0,
        super::Route::Logs => 1,
        super::Route::Scale => 2,
        super::Route::Rollback => 3,
    };
    let name = service.0.clone();
    let result = backend.0.borrow().service(&name);
    cx.use_input(result.is_ok(), move |input, key| {
        if key.ctrl && input == "p" {
            pinned.set(!pinned.get());
        }
        if tab != 2 && !key.ctrl && !key.meta && input == "d" {
            router.push(Route::Deploy {
                route: deploy::Route::Service {
                    service: deploy::service::Service(name.clone()),
                    route: deploy::service::Route::Index,
                },
            });
        }
        if key.tab {
            let next = if key.shift {
                (tab + 3) % 4
            } else {
                (tab + 1) % 4
            };
            let route = [
                super::Route::Index,
                super::Route::Logs,
                super::Route::Scale,
                super::Route::Rollback,
            ][next]
                .clone();
            router.replace(Route::Services {
                route: services::Route::Service {
                    service: service.clone(),
                    route,
                },
            });
        }
    });
    match result {
        Err(error) => view::alert(error),
        Ok(service) => view::column()
            .child(
                View::new()
                    .gap(1)
                    .child(Text::new(format!(
                        "{} {} | {} replicas | pinned: {}",
                        service.name,
                        service.version,
                        service.replicas,
                        pinned.get()
                    )))
                    .child(HealthBadge {
                        health: service.health,
                    }),
            )
            .child(
                View::new().gap(2).children(
                    ["overview", "logs", "scale", "rollback"]
                        .into_iter()
                        .enumerate()
                        .map(|(i, label)| Text::new(label).inverse(i == tab)),
                ),
            )
            .child(Outlet::default())
            .into(),
    }
}
