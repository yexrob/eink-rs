use eink_rs::{
    router::use_router,
    ui::{Select, SelectOption},
};
use ink_rs::{Callback, Element, Props, Scope, Text};

use crate::{
    routes::{_layout::ThemeChoice, Route, settings},
    view,
};

/// Palette selection updates the shell's context for every page.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = index)]
pub struct Index {}

fn index(cx: &mut Scope, _: &Index) -> Element {
    let choice = cx
        .use_context::<ThemeChoice>()
        .expect("shell theme provider");
    let router = use_router::<Route>(cx);
    cx.use_input(true, move |input, _| {
        if input == "a" {
            router.push(Route::Settings {
                route: settings::Route::Account,
            });
        }
    });
    view::column()
        .child(Text::new("Theme").bold(true))
        .child(
            Select::default()
                .options(
                    ["Ink UI", "Harbor", "Signal"]
                        .into_iter()
                        .enumerate()
                        .map(|(i, name)| SelectOption::new(name, i))
                        .collect(),
                )
                .default_value(Some(choice.current))
                .on_change(Some(Callback::new(move |value| choice.change.call(value)))),
        )
        .into()
}
