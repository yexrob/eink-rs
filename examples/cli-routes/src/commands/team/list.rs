use eink_rs::cli::clap::{self, Args};
use ink_rs::{Element, Props, Scope, Text};

/// List deployment team members.
#[derive(Args, Clone, PartialEq, Props)]
#[props(component = list)]
pub struct List {}

fn list(_: &mut Scope, _: &List) -> Element {
    Text::new("Deployment team: Ada, Lin").into()
}
