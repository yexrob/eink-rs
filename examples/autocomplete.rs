// Ink UI examples/autocomplete.
// visible_option_count replaces defaultLimit because eink-rs names the visible row count.

use eink_rs::ui::{Select, SelectOption, TextInput};
use ink_rs::{Callback, Element, FlexDirection, Options, Scope, Text, View, component, rsx};

fn example(cx: &mut Scope, _: &()) -> Element {
    let filter_text = cx.use_state(String::new);
    let value = cx.use_state(|| None::<&str>);
    let filter = filter_text.get();
    let options = cx.use_memo(filter.clone(), || {
        [
            SelectOption::new("Red", "red"),
            SelectOption::new("Green", "green"),
            SelectOption::new("Yellow", "yellow"),
            SelectOption::new("Blue", "blue"),
            SelectOption::new("Magenta", "magenta"),
            SelectOption::new("Cyan", "cyan"),
            SelectOption::new("White", "white"),
        ]
        .into_iter()
        .filter(|option| option.label.contains(&filter))
        .collect::<Vec<_>>()
    });
    rsx! {
        View(flex_direction: FlexDirection::Column, gap: 1) {
            if let Some(selected) = value.get() {
                Text { "You've selected {selected}" }
            } else {
                TextInput(on_change: Some(Callback::new(move |next| filter_text.set(next))))
                Select(
                    visible_option_count: 5,
                    highlight_text: filter,
                    options: options,
                    on_change: Some(Callback::new(move |next| value.set(Some(next))))
                )
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
