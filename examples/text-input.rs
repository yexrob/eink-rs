// Ink UI examples/text-input.

use eink_rs::ui::TextInput;
use ink_rs::{Callback, Element, FlexDirection, Options, Scope, Text, View, component, rsx};

fn example(cx: &mut Scope, _: &()) -> Element {
    let value = cx.use_state(String::new);
    rsx! {
        View(padding: 2, flex_direction: FlexDirection::Column, gap: 1) {
            TextInput(
                placeholder: "Start typing...",
                on_change: Some(Callback::new(move |next| value.set(next)))
            )
            Text { {format!("Input value: \"{}\"", value.get())} }
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
