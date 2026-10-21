// Ink UI examples/theming/spinner.
// Scope::provide supplies a typed theme because eink-rs uses theme data instead of extendTheme.

use eink_rs::ui::{Palette, Spinner, SpinnerTheme};
use ink_rs::{Color, Element, Options, Scope, View, component, rsx};

fn example(cx: &mut Scope, _: &()) -> Element {
    cx.provide(SpinnerTheme {
        frame_color: Color::Magenta,
        ..SpinnerTheme::from(&Palette::default())
    });
    rsx! {
        View(padding: 2) {
            Spinner(label: "Loading")
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
