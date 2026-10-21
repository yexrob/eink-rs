use eink_rs::cli::clap::{self, Args};
use ink_rs::{Element, Props, Scope, Text};

/// List configured domains.
#[derive(Args, Clone, PartialEq, Props)]
#[props(component = list)]
pub struct List {}

fn list(_: &mut Scope, _: &List) -> Element {
    Text::new("example.com").into()
}
