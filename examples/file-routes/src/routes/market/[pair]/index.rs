use eink_rs::router::use_param;
use ink_rs::{Element, Props, Scope, Text};

use super::Pair;

/// The chart for the selected pair.
#[derive(Default, PartialEq, Props)]
#[props(component = chart)]
pub struct Index {}

fn chart(cx: &mut Scope, _: &Index) -> Element {
    Text::new(format!("{} chart: _/\\__/\\_", use_param::<Pair>(cx))).into()
}
