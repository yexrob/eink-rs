//! Commands rendered by components, with a root wrapper and default command.
//!
//! ```sh
//! cargo run --example cli --features cli -- --help
//! cargo run --example cli --features cli --
//! cargo run --example cli --features cli -- deploy website --region eu
//! cargo run --example cli --features cli -- d website
//! cargo run --example cli --features cli -- domains list
//! cargo run --example cli --features cli -- fail
//! ```

use std::process::ExitCode;

use eink_rs::cli::{
    self, Commands,
    clap::{self, Args, Parser, Subcommand},
};
use ink_rs::{Element, FlexDirection, Props, Scope, Text, View, rsx};

#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = app)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Clone, PartialEq, Commands)]
enum Command {
    /// Describe a deployment
    #[command(alias = "d")]
    Deploy(Deploy),
    /// Manage domains
    #[command(subcommand)]
    Domains(Domains),
    /// Demonstrate an error exit
    Fail(Fail),
}

#[derive(Args, Clone, PartialEq, Props)]
#[props(component = deploy)]
struct Deploy {
    /// Project to deploy
    #[arg(default_value = "website")]
    project: String,
    /// Deployment region
    #[arg(long, default_value = "us")]
    region: String,
}

impl Default for Deploy {
    fn default() -> Self {
        Self {
            project: "website".into(),
            region: "us".into(),
        }
    }
}

fn deploy(_: &mut Scope, props: &Deploy) -> Element {
    Text::new(format!("Deploy {} to {}", props.project, props.region)).into()
}

#[derive(Subcommand, Clone, PartialEq, Commands)]
enum Domains {
    /// List configured domains
    List(List),
}

#[derive(Args, Clone, PartialEq, Props)]
#[props(component = list)]
struct List {}

fn list(_: &mut Scope, _: &List) -> Element {
    Text::new("example.com").into()
}

#[derive(Args, Clone, PartialEq, Props)]
#[props(component = fail)]
struct Fail {}

fn fail(cx: &mut Scope, _: &Fail) -> Element {
    let app = cx.use_app();
    cx.use_effect((), move || app.exit_with_error("deployment failed"));
    Text::new("Checking deployment").into()
}

fn app(_: &mut Scope, props: &Cli) -> Element {
    let command: Element = match props.command.clone() {
        Some(command) => command.into(),
        None => Deploy::default().into(),
    };
    rsx! {
        View(flex_direction: FlexDirection::Column) {
            Text(bold: true) { "Deployment console" }
            {command}
        }
    }
    .into()
}

fn main() -> ExitCode {
    cli::run::<Cli>()
}
