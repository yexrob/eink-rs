//! Ported from Ink UI 2.0.0 test/status-message.tsx.

mod support;

use eink_rs::ui::{StatusMessage, Variant};
use ink_rs::{Color, FlexDirection, Styled, Text, View, rsx, testing::render};

#[test]
fn success() {
    // Ink UI marks this expected frame as failing; the port renders it.
    let instance = render(rsx! { StatusMessage(Variant::Success) { Text { "Success" } } });
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new(Text::new("✔").color(Color::Green)).child(" Success"))
    );
}

#[test]
fn error() {
    let instance = render(rsx! { StatusMessage(Variant::Error) { Text { "Error" } } });
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new(Text::new("✘").color(Color::Red)).child(" Error"))
    );
}

#[test]
fn warning() {
    // Ink UI marks this expected frame as failing; the port renders it.
    let instance = render(rsx! { StatusMessage(Variant::Warning) { Text { "Warning" } } });
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new(Text::new("⚠").color(Color::Yellow)).child(" Warning"))
    );
}

#[test]
fn info() {
    // Ink UI marks this expected frame as failing; the port renders it.
    let instance = render(rsx! { StatusMessage(Variant::Info) { Text { "Info" } } });
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new(Text::new("ℹ").color(Color::Blue)).child(" Info"))
    );
}

#[test]
fn multiline_message() {
    // Ink UI marks this hanging-indent frame as failing; the port renders it.
    let instance = render(rsx! {
        View(width: 8) { StatusMessage(Variant::Info) { Text { "Hello world" } } }
    });
    let expected = View::new()
        .flex_direction(FlexDirection::Column)
        .child(Text::new(Text::new("ℹ").color(Color::Blue)).child(" Hello"))
        .child(Text::new("  world"));
    assert_eq!(instance.last_frame(), support::frame(expected));
}
