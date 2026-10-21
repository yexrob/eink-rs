use ink_rs::{Element, Props, Scope, Text};

/// The home page.
#[derive(Default, PartialEq, Props)]
#[props(component = home)]
pub struct Index {}

fn home(_: &mut Scope, _: &Index) -> Element {
    Text::new("Choose a market with m or open the wallet with w.").into()
}
