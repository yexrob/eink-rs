// Ink UI examples/theming/unordered-list.
// Scope::provide supplies a typed theme because eink-rs uses theme data instead of extendTheme.
// A one-element markers vector replaces marker because eink-rs supports markers by depth.
// Children replace Item wrappers because eink-rs treats each child as an item.
// A column View groups Green and its nested list because one item holds both.

use eink_rs::ui::{Palette, UnorderedList, UnorderedListTheme};
use ink_rs::{Element, FlexDirection, Options, Scope, Text, View, component, rsx};

fn example(cx: &mut Scope, _: &()) -> Element {
    cx.provide(UnorderedListTheme {
        markers: vec!["+".into()],
        ..UnorderedListTheme::from(&Palette::default())
    });
    rsx! {
        View(padding: 2) {
            UnorderedList {
                Text { "Red" }
                View(flex_direction: FlexDirection::Column) {
                    Text { "Green" }
                    UnorderedList {
                        Text { "Light" }
                        Text { "Dark" }
                    }
                }
                Text { "Blue" }
            }
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
