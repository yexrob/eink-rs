use eink_rs::{
    router::{Outlet, use_router},
    ui::{Badge, Palette, use_theme},
};
use ink_rs::{Callback, Color, Element, Props, Scope, Styled, Text, View, component};

use crate::{
    backend::{Health, use_backend},
    routes::{Route, deploy, services, settings},
    view::{self, ConsoleTheme},
};

/// The console's shared palette and history boundary.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = layout)]
pub struct Layout {}

/// The settings page changes the shell through a callback, not a State context.
#[derive(Clone, PartialEq)]
pub(crate) struct ThemeChoice {
    pub(crate) current: usize,
    pub(crate) change: Callback<usize>,
}

fn layout(cx: &mut Scope, _: &Layout) -> Element {
    let choice = cx.use_state(|| 0);
    let change = cx.use_callback(move |value| choice.set(value));
    let palette = match choice.get() {
        1 => Palette {
            primary: Color::Cyan,
            info: Color::Cyan,
            accent: Color::Blue,
            ..Palette::default()
        },
        2 => Palette {
            primary: Color::Yellow,
            info: Color::White,
            success: Color::Cyan,
            accent: Color::Yellow,
            ..Palette::default()
        },
        _ => Palette::default(),
    };
    cx.provide(palette);
    cx.provide(ThemeChoice {
        current: choice.get(),
        change,
    });
    component(shell, ())
}

fn shell(cx: &mut Scope, _: &()) -> Element {
    let router = use_router::<Route>(cx);
    let backend = use_backend(cx);
    let theme = use_theme::<ConsoleTheme>(cx);
    let app = cx.use_app();
    let navigation = router.clone();
    cx.use_input(true, move |_, key| {
        if key.escape {
            if navigation.can_go_back() {
                navigation.back();
            } else {
                app.exit();
            }
        }
    });
    let services = backend.0.borrow().services();
    let badges = [Health::Healthy, Health::Degraded, Health::Offline]
        .into_iter()
        .map(|health| {
            let count = services.iter().filter(|s| s.health == health).count();
            Badge::new(format!("{} {count}", health.label())).color(Some(theme.health(health)))
        });
    view::column()
        .child(
            View::new()
                .gap(1)
                .child(Text::new("SHIP").bold(true).color(theme.heading))
                .child(Badge::new("simulated")),
        )
        .child(Text::new(format!("Route: {}", router.route())).color(theme.muted))
        .child(View::new().gap(1).children(badges))
        .child(Outlet::default())
        .child(Text::new(hints(router.route())).color(theme.muted))
        .child(Text::new("Esc back / exit | Ctrl-C exit").dim_color(true))
        .into()
}

fn hints(route: &Route) -> &'static str {
    match route {
        Route::Index => "Up/Down choose | Enter open",
        Route::Services {
            route: services::Route::Index,
        } => "Type to filter | Up/Down choose | Enter open",
        Route::Services {
            route: services::Route::Service { route, .. },
        } => match route {
            services::service::Route::Index => "Tab/Shift-Tab tabs | Ctrl-P pin | d deploy",
            services::service::Route::Logs => {
                "Tab/Shift-Tab tabs | Ctrl-P pin | d deploy | Space pause / resume"
            }
            services::service::Route::Scale => {
                "Tab/Shift-Tab tabs | Ctrl-P pin | 0-20 replicas | Enter submit | y/n confirm"
            }
            services::service::Route::Rollback => {
                "Tab/Shift-Tab tabs | Ctrl-P pin | d deploy | Up/Down release | Enter select | y/n confirm"
            }
        },
        Route::Deploy {
            route: deploy::Route::Index,
        } => "Up/Down service | Enter choose",
        Route::Deploy {
            route: deploy::Route::Service { route, .. },
        } => match route {
            deploy::service::Route::Index => "Up/Down version | Enter choose",
            deploy::service::Route::Regions => "Up/Down region | Space toggle | Enter review",
            deploy::service::Route::Review => "y / Enter deploy | n back",
            deploy::service::Route::Run => "Deployment continues after leaving this page",
        },
        Route::Settings {
            route: settings::Route::Index,
        } => "Up/Down palette | Enter apply | a account",
        Route::Settings {
            route: settings::Route::Account,
        } => "Tab / Shift-Tab focus | Enter next / save",
    }
}
