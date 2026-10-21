// Ink UI examples/multi-select.

use eink_rs::ui::{MultiSelect, SelectOption};
use ink_rs::{Callback, Element, FlexDirection, Options, Scope, Text, View, component, rsx};

fn example(cx: &mut Scope, _: &()) -> Element {
    let value = cx.use_state(Vec::<&str>::new);
    let options = vec![
        SelectOption::new("Red", "red"),
        SelectOption::new("Green", "green"),
        SelectOption::new("Yellow", "yellow"),
        SelectOption::new("Blue", "blue"),
        SelectOption::new("Magenta", "magenta"),
        SelectOption::new("Cyan", "cyan"),
        SelectOption::new("White", "white"),
    ];
    rsx! {
        View(padding: 2, flex_direction: FlexDirection::Column, gap: 1) {
            MultiSelect(
                options: options,
                on_change: Some(Callback::new(move |next| value.set(next)))
            )
            Text { {format!("Selected values: {}", value.get().join(", "))} }
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
