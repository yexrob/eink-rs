use ink_rs::{Element, Props, Scope, Text};

/// The market selection page.
#[derive(Default, PartialEq, Props)]
#[props(component = index)]
pub struct Index {}

fn index(_: &mut Scope, _: &Index) -> Element {
    Text::new("Choose a market with m.").into()
}
