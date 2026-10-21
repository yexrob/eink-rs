// Ink UI examples/confirm-input.

use eink_rs::ui::ConfirmInput;
use ink_rs::{Callback, Element, Options, Scope, Text, View, component, rsx};

fn example(cx: &mut Scope, _: &()) -> Element {
    let choice = cx.use_state(|| None::<&str>);
    rsx! {
        View(gap: 1) {
            if choice.get().is_none() {
                Text(bold: true) { "Do you agree with terms of service?" }
                ConfirmInput(
                    Callback::new(move |()| choice.set(Some("agreed"))),
                    Callback::new(move |()| choice.set(Some("disagreed")))
                )
            }
            if choice.get() == Some("agreed") {
                Text { "I know you haven't read them, but ok" }
            }
            if choice.get() == Some("disagreed") {
                Text { "Ok, whatever" }
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
