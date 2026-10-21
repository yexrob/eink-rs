//! Ported from Ink UI 2.0.0 test/badge.tsx.

mod support;

use eink_rs::ui::Badge;
use ink_rs::{Color, Text, rsx, testing::render};

#[test]
fn badge() {
    let instance = render(rsx! { Badge("Success", color: Some(Color::Green)) });
    let expected = Text::new(" ")
        .background_color(Color::Green)
        .child(Text::new("SUCCESS").color(Color::Black))
        .child(" ");
    assert_eq!(instance.last_frame(), support::frame(expected));
}
