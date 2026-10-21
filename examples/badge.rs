// Ink UI examples/badge.
// Labels are constructor arguments because Badge accepts a string instead of children.

use eink_rs::ui::Badge;
use ink_rs::{Color, Element, Options, Scope, View, component, rsx};

fn example(_cx: &mut Scope, _: &()) -> Element {
    rsx! {
        View(gap: 2, padding: 2) {
            Badge("Pass", color: Some(Color::Green))
            Badge("Fail", color: Some(Color::Red))
            Badge("Warn", color: Some(Color::Yellow))
            Badge("Todo", color: Some(Color::Blue))
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
