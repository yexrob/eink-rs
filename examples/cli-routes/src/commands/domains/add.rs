use eink_rs::cli::clap::{self, Args};
use ink_rs::{Element, Props, Scope, Text};

/// Describe a domain attachment.
#[derive(Args, Clone, PartialEq, Props)]
#[props(component = add)]
pub struct Add {
    /// Domain to attach.
    pub domain: String,
}

fn add(_: &mut Scope, props: &Add) -> Element {
    Text::new(format!("Attach {} to website", props.domain)).into()
}
