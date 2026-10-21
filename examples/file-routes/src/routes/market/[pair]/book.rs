use eink_rs::router::use_param;
use ink_rs::{Element, Props, Scope, Text};

use super::Pair;

/// The order book for the selected pair.
#[derive(Default, PartialEq, Props)]
#[props(component = book)]
pub struct Book {}

fn book(cx: &mut Scope, _: &Book) -> Element {
    Text::new(format!("{} order book: bids | asks", use_param::<Pair>(cx))).into()
}
