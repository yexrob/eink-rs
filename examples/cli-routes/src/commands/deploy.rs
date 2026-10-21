use eink_rs::cli::clap::{self, Args};
use ink_rs::{Element, Props, Scope, Text};

/// Describe a deployment.
#[derive(Args, Clone, PartialEq, Props)]
#[command(visible_alias = "d")]
#[props(component = deploy)]
pub struct Deploy {
    /// Project to deploy.
    #[arg(default_value = "website")]
    pub project: String,
    /// Deployment region.
    #[arg(long, default_value = "us")]
    pub region: String,
    /// Select a preview deployment.
    #[arg(long)]
    pub preview: bool,
}

impl Default for Deploy {
    fn default() -> Self {
        Self {
            project: "website".into(),
            region: "us".into(),
            preview: false,
        }
    }
}

fn deploy(_: &mut Scope, props: &Deploy) -> Element {
    let mode = if props.preview { " (preview)" } else { "" };
    Text::new(format!(
        "Deploy {} to {}{mode}",
        props.project, props.region
    ))
    .into()
}
