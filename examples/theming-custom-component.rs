// Ink UI examples/theming/custom-component.
// Scope::provide and use_theme replace the named component theme because eink-rs keys themes by type.
// From<&Palette> supplies the green default through the success token to follow eink-rs's theme contract.

use eink_rs::ui::{Palette, use_theme};
use ink_rs::{Color, Element, Options, Scope, Text, component, rsx};

#[derive(Clone, PartialEq)]
struct CustomLabelTheme {
    color: Color,
}

impl From<&Palette> for CustomLabelTheme {
    fn from(palette: &Palette) -> Self {
        Self {
            color: palette.success,
        }
    }
}

fn custom_label(cx: &mut Scope, _: &()) -> Element {
    let theme = use_theme::<CustomLabelTheme>(cx);
    rsx! {
        Text(color: theme.color) { "Hello world" }
    }
    .into()
}

fn example(cx: &mut Scope, _: &()) -> Element {
    cx.provide(CustomLabelTheme::from(&Palette::default()));
    component(custom_label, ())
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
