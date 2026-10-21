//! Ported from Ink UI 2.0.0 test/progress-bar.tsx.

mod support;

use eink_rs::ui::ProgressBar;
use ink_rs::{Color, Text, View, rsx, testing::render};

#[test]
fn progress_bar() {
    let mut instance = render(rsx! { View(width: 20) { ProgressBar(value: 0.0) } });
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new("░".repeat(20)).dim_color(true))
    );
    instance.rerender(rsx! { View(width: 20) { ProgressBar(value: 50.0) } });
    let expected = Text::new(Text::new("█".repeat(10)).color(Color::Magenta))
        .child(Text::new("░".repeat(10)).dim_color(true));
    assert_eq!(instance.last_frame(), support::frame(expected));
}
