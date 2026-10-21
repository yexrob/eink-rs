use ink_rs::{Element, Props, Scope, Text};

/// The wallet page.
#[derive(Default, PartialEq, Props)]
#[props(component = wallet)]
pub struct Wallet {}

fn wallet(_: &mut Scope, _: &Wallet) -> Element {
    Text::new("Wallet: no account connected.").into()
}
