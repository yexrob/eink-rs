// Ink UI examples/status-message.
// Text wraps message strings because StatusMessage children must be elements in eink-rs.

use eink_rs::ui::{StatusMessage, Variant};
use ink_rs::{Element, FlexDirection, Options, Scope, Text, View, component, rsx};

fn example(_cx: &mut Scope, _: &()) -> Element {
    rsx! {
        View(flex_direction: FlexDirection::Column, padding: 2) {
            StatusMessage(Variant::Success) { Text { "Success" } }
            StatusMessage(Variant::Error) { Text { "Error" } }
            StatusMessage(Variant::Warning) { Text { "Warning" } }
            StatusMessage(Variant::Info) { Text { "Info" } }
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
