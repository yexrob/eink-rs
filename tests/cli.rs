#![cfg(feature = "cli")]

use std::process::ExitCode;

use eink_rs::cli::{
    self, Commands,
    clap::{self, Args, Parser, Subcommand},
};
use ink_rs::{Element, Props, Scope, Text, testing::render};

#[derive(Parser, Clone, PartialEq, Props)]
#[command(name = "test-cli", version, about)]
#[props(component = app)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Clone, PartialEq, Commands)]
enum Command {
    #[command(alias = "g")]
    Greet(Greet),
    #[command(subcommand)]
    Team(Team),
    Exit(Exit),
}

#[derive(Subcommand, Clone, PartialEq, Commands)]
enum Team {
    Greet(Greet),
}

#[derive(Args, Clone, PartialEq, Props)]
#[props(component = greet)]
struct Greet {
    name: String,
}

fn greet(_: &mut Scope, props: &Greet) -> Element {
    Text::new(format!("Hello, {}!", props.name)).into()
}

fn app(_: &mut Scope, props: &Cli) -> Element {
    match props.command.clone() {
        Some(command) => command.into(),
        None => Greet {
            name: "world".into(),
        }
        .into(),
    }
}

#[derive(Args, Clone, PartialEq, Props)]
#[props(component = exit)]
struct Exit {
    #[arg(long)]
    error: bool,
    #[arg(long)]
    code: Option<u8>,
}

fn exit(cx: &mut Scope, props: &Exit) -> Element {
    let app = cx.use_app();
    let props = props.clone();
    cx.use_effect((), move || {
        if props.error {
            app.exit_with_error("command failed");
        } else if let Some(code) = props.code {
            app.exit_with(ExitCode::from(code));
        }
    });
    Text::new("Exiting").into()
}

#[test]
fn dispatch() {
    for argv in [
        vec!["test-cli", "greet", "Ada"],
        vec!["test-cli", "g", "Ada"],
        vec!["test-cli", "team", "greet", "Ada"],
    ] {
        let element = cli::parse_from::<Cli>(argv).unwrap();
        assert_eq!(render(element).last_frame(), "Hello, Ada!");
    }
}

#[test]
fn default_command() {
    let element = cli::parse_from::<Cli>(["test-cli"]).unwrap();
    assert_eq!(render(element).last_frame(), "Hello, world!");
}

#[test]
fn exit_codes() {
    for (argv, code) in [
        (vec!["test-cli", "greet"], 2),
        (vec!["test-cli", "--help"], 0),
        (vec!["test-cli", "exit", "--error"], 1),
        (vec!["test-cli", "exit", "--code", "3"], 3),
        (vec!["test-cli", "greet", "Ada"], 0),
    ] {
        assert_eq!(cli::run_from::<Cli>(argv), ExitCode::from(code));
    }
}

#[cfg(feature = "completion")]
#[test]
fn completions() {
    assert_eq!(
        cli::run_from::<Cli>(["test-cli", "completions", "bash"]),
        ExitCode::SUCCESS
    );
}
