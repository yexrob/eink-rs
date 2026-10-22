mod support;

use eink_rs::ui::{Button, ButtonTheme, ButtonVariant, FocusRing, Palette, RingFocus};
use ink_rs::{Callback, Color, Element, Scope, Text, component, testing::render};
use std::{cell::RefCell, rc::Rc};

#[test]
fn brackets_plain_hotkey_variants_and_focus_inversion() {
    let tree = |button, focused| FocusRing::default().focused(focused).child(button);
    let mut app = render(tree(Button::default().id("a").label("OK"), RingFocus::None));
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("[ OK ]").color(Color::Magenta))
    );
    app.rerender(tree(
        Button::default().id("a").label("OK"),
        RingFocus::Key("a".into()),
    ));
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("[ OK ]").color(Color::Magenta).inverse(true))
    );
    app.rerender(tree(
        Button::default()
            .id("a")
            .label("OK")
            .plain(true)
            .hotkey(Some('1'))
            .variant(ButtonVariant::Secondary),
        RingFocus::None,
    ));
    assert_eq!(app.last_frame(), support::frame(Text::new("1: OK")));
}

#[test]
fn children_theme_and_palette_control_appearance() {
    fn themed(cx: &mut Scope, custom: &bool) -> Element {
        let palette = Palette {
            accent: Color::Cyan,
            ..Palette::default()
        };
        cx.provide(palette);
        cx.provide(if *custom {
            ButtonTheme {
                opening: "<".into(),
                closing: ">".into(),
                hotkey_separator: "=".into(),
                ..ButtonTheme::from(&palette)
            }
        } else {
            ButtonTheme::from(&palette)
        });
        FocusRing::default()
            .focused(RingFocus::None)
            .child(
                Button::default()
                    .id("a")
                    .label("ignored")
                    .hotkey(Some('y'))
                    .child(Text::new("Yes").bold(true)),
            )
            .into()
    }
    let mut app = render(component(themed, false));
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("[ y: ")
                .color(Color::Cyan)
                .child(Text::new("Yes").bold(true))
                .child(" ]")
        )
    );
    app.rerender(component(themed, true));
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("<y=")
                .color(Color::Cyan)
                .child(Text::new("Yes").bold(true))
                .child(">")
        )
    );
}

#[test]
fn enter_space_hotkey_and_disabled_gating() {
    let presses = Rc::new(RefCell::new(0));
    let count = presses.clone();
    let callback = Callback::new(move |()| *count.borrow_mut() += 1);
    let tree = |disabled| {
        Button::default()
            .id("a")
            .label("OK")
            .hotkey(Some('y'))
            .is_disabled(disabled)
            .on_press(Some(callback.clone()))
    };
    let mut app = render(tree(false));
    for key in ["\r", " ", "y"] {
        app.input(key);
    }
    assert_eq!(*presses.borrow(), 3);
    app.rerender(tree(true));
    for key in ["\r", " ", "y"] {
        app.input(key);
    }
    assert_eq!(*presses.borrow(), 3);
}
