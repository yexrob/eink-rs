//! Ported from Ink UI 2.0.0 test/alert.tsx.

mod support;

use eink_rs::ui::{Alert, Variant};
use ink_rs::{BorderStyle, Color, FlexDirection, Styled, Text, View, rsx, testing::render};

fn expected(icon: &str, color: Color, title: &str) -> String {
    support::frame(
        View::new()
            .width(16)
            .border_style(BorderStyle::Round)
            .border_color(color)
            .padding_x(1)
            .flex_direction(FlexDirection::Column)
            .child(
                Text::new(Text::new(icon).color(color))
                    .child(" ")
                    .child(Text::new(title).bold(true)),
            )
            .child(View::new().height(1))
            .child(Text::new("  Message")),
    )
}

#[test]
fn success() {
    // Ink UI marks this expected frame as failing; the port renders it.
    let instance = render(rsx! {
        View(width: 16) {
            Alert(Variant::Success, title: Some("Success".into())) { Text { "Message" } }
        }
    });
    assert_eq!(
        instance.last_frame(),
        expected("✔", Color::Green, "Success")
    );
}

#[test]
fn error() {
    let instance = render(rsx! {
        View(width: 16) {
            Alert(Variant::Error, title: Some("Error".into())) { Text { "Message" } }
        }
    });
    assert_eq!(instance.last_frame(), expected("✘", Color::Red, "Error"));
}

#[test]
fn warning() {
    // Ink UI marks this expected frame as failing; the port renders it.
    let instance = render(rsx! {
        View(width: 16) {
            Alert(Variant::Warning, title: Some("Warning".into())) { Text { "Message" } }
        }
    });
    assert_eq!(
        instance.last_frame(),
        expected("⚠", Color::Yellow, "Warning")
    );
}

#[test]
fn info() {
    // Ink UI marks this expected frame as failing; the port renders it.
    let instance = render(rsx! {
        View(width: 16) {
            Alert(Variant::Info, title: Some("Info".into())) { Text { "Message" } }
        }
    });
    assert_eq!(instance.last_frame(), expected("ℹ", Color::Blue, "Info"));
}
