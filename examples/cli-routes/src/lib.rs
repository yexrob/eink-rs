//! Deployment commands generated from a directory, with a root wrapper.
//!
//! ```sh
//! cargo run -p cli-routes -- --help
//! cargo run -p cli-routes --
//! cargo run -p cli-routes -- deploy website --region eu --preview
//! cargo run -p cli-routes -- d website
//! cargo run -p cli-routes -- domains
//! cargo run -p cli-routes -- dom --verbose
//! cargo run -p cli-routes -- domains list
//! cargo run -p cli-routes -- domains add example.org
//! cargo run -p cli-routes -- team
//! cargo run -p cli-routes -- team list
//! cargo run -p cli-routes -- team projects list
//! ```

use eink_rs::cli::clap::{self, Parser};
use ink_rs::{Element, FlexDirection, Props, Scope, Text, View, rsx};

/// The commands discovered under `src/commands`.
pub mod commands {
    eink_rs::cli::commands!("src/commands");
}

/// A deployment console with directory-based routing.
#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = app)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<commands::Command>,
}

fn app(_: &mut Scope, props: &Cli) -> Element {
    let command: Element = match props.command.clone() {
        Some(command) => command.into(),
        None => commands::deploy::Deploy::default().into(),
    };
    rsx! {
        View(flex_direction: FlexDirection::Column) {
            Text { "Deployment console" }
            {command}
        }
    }
    .into()
}
