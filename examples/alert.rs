// Ink UI examples/alert.
// Text wraps message strings because Alert children must be elements in eink-rs.

use eink_rs::ui::{Alert, Variant};
use ink_rs::{Element, FlexDirection, Options, Scope, Text, View, component, rsx};

fn example(_cx: &mut Scope, _: &()) -> Element {
    rsx! {
        View(flex_direction: FlexDirection::Column, padding: 2, width: 60, gap: 1) {
            Alert(Variant::Success) { Text { "A new version of this CLI is available" } }
            Alert(Variant::Error) { Text { "Your license is expired" } }
            Alert(Variant::Warning) { Text { "Current version of this CLI has been deprecated" } }
            Alert(Variant::Info) { Text { "API won't be available tomorrow night" } }
        }
    }
    .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(component(example, ()), Options::default())
        .wait_until_exit()
        .map(|_| ())
}
