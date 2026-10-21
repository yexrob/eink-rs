use eink_rs::cli::clap::{self, Args};
use ink_rs::{Element, Props, Scope, Text};

/// Manage deployment domains.
#[derive(Args, Clone, PartialEq, Props)]
#[command(visible_alias = "dom")]
#[props(component = index)]
pub struct Index {
    /// Include the deployment region.
    #[arg(long)]
    pub verbose: bool,
}

fn index(_: &mut Scope, props: &Index) -> Element {
    Text::new(if props.verbose {
        "Domains for website in us"
    } else {
        "Domains for website"
    })
    .into()
}
