use eink_rs::ui::{Button, ButtonVariant, FocusRing, Select, SelectOption, TextInput};
use ink_rs::{Callback, Element, FlexDirection, Options, Scope, Styled, Text, View, component};

fn example(cx: &mut Scope, _: &()) -> Element {
    let text = cx.use_state(|| "bingo".to_owned());
    let model = cx.use_state(|| "balanced");
    let presses = cx.use_state(|| 0);
    let app = cx.use_app();
    View::new()
        .padding(1)
        .flex_direction(FlexDirection::Column)
        .gap(1)
        .child(Text::new(
            "Tab / Shift+Tab: focus. Arrows: move or edit. Esc: exit.",
        ))
        .child(
            FocusRing::default()
                .arrows(true)
                .on_escape(Some(Callback::new(move |()| app.exit())))
                .child(
                    View::new()
                        .flex_direction(FlexDirection::Column)
                        .gap(1)
                        .child(
                            View::new()
                                .gap(2)
                                .child(
                                    Button::default()
                                        .id("apply")
                                        .label("Apply")
                                        .hotkey(Some('a'))
                                        .auto_focus(true)
                                        .on_press(Some(Callback::new(move |()| {
                                            presses.update(|count| *count += 1)
                                        }))),
                                )
                                .child(
                                    Button::default()
                                        .id("reset")
                                        .label("Reset")
                                        .hotkey(Some('r'))
                                        .variant(ButtonVariant::Secondary)
                                        .on_press(Some(Callback::new(move |()| {
                                            text.set("bingo".into());
                                            model.set("balanced");
                                        }))),
                                ),
                        )
                        .child(
                            TextInput::default()
                                .id(Some("name".into()))
                                .value(Some(text.get()))
                                .placeholder("Name")
                                .on_change(Some(Callback::new(move |value| text.set(value)))),
                        )
                        .child(
                            Select::default()
                                .id(Some("model".into()))
                                .value(Some(model.get()))
                                .number_hotkeys(true)
                                .options(vec![
                                    SelectOption::new("Fast", "fast").description("Short tasks"),
                                    SelectOption::new("Balanced", "balanced")
                                        .description("Everyday work"),
                                    SelectOption::new("Deep", "deep").description("Complex tasks"),
                                ])
                                .on_change(Some(Callback::new(move |value| model.set(value)))),
                        ),
                ),
        )
        .child(Text::new(format!(
            "Name: {} | Model: {} | Applied: {}",
            text.get(),
            model.get(),
            presses.get()
        )))
        .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
