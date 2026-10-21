//! A CLI framework borrowing command components, wrappers, and default commands
//! from [Pastel](https://github.com/vadimdemedes/pastel).
//!
//! Commands are clap derive types. Each command's arguments
//! are its component props, with [`ink_rs::Props`] providing the conversion to
//! [`Element`]. [`Commands`] dispatches a subcommand enum to those components.
//! The root [`clap::Parser`] is a component too: its render function supplies
//! the wrapper and chooses an index or default command. Clap owns help,
//! validation, aliases, and package metadata through `#[command(version, about)]`.
//! Pastel's help format and `-v` flag are not reproduced.
//!
//! The `completion` feature adds `completions <SHELL>` to the run entry points.
//! This name clashes with a user command named `completions` and must be reserved.
//! [`parse_from`] uses the user's parser without this additional subcommand.
//! The `tokio` feature adds async entry points for a local runtime or `LocalSet`.
//!
//! Run entry points print clap errors and return their exit codes: help and
//! version return 0, and usage errors return 2. App and terminal errors print
//! `Error: {error}` to stderr and return 1. An exit value of type [`ExitCode`]
//! becomes the return code; other values, `exit()`, and the natural end return 0.
//! Component panics resume unwinding with their original payload.

use std::any::Any;
use std::ffi::OsString;
use std::io::Write;
use std::process::ExitCode;

use ink_rs::{Element, Error, Options};

pub use clap;
pub use eink_rs_macros::Commands;

/// Parses arguments into a component element without mounting it.
///
/// The first argument is the program name, as with [`clap::Parser::try_parse_from`].
/// The element can be passed to [`ink_rs::testing::render`] or
/// [`ink_rs::testing::mount`]. This does not add the `completions` subcommand.
pub fn parse_from<C>(
    argv: impl IntoIterator<Item = impl Into<OsString> + Clone>,
) -> Result<Element, clap::Error>
where
    C: clap::Parser + Into<Element>,
{
    C::try_parse_from(argv).map(Into::into)
}

/// Parses process arguments and renders the root component until exit.
///
/// Uses [`Options::default`]. With `completion`, reserves `completions <SHELL>`
/// to print a shell completion script. Exit handling follows the module docs.
pub fn run<C>() -> ExitCode
where
    C: clap::Parser + Into<Element>,
{
    run_from::<C>(std::env::args_os())
}

/// Parses arguments, including the program name, and renders until exit.
///
/// Rendering, completion, and exit handling match [`run`].
pub fn run_from<C>(argv: impl IntoIterator<Item = impl Into<OsString> + Clone>) -> ExitCode
where
    C: clap::Parser + Into<Element>,
{
    launch::<C>(argv, |_| Options::default())
}

/// Parses process arguments and renders with the options `options` returns.
///
/// `options` receives the parsed root before it becomes an element, so a
/// command can choose its own render options, such as the alternate screen.
/// Completion and exit handling match [`run`].
pub fn run_with<C>(options: impl FnOnce(&C) -> Options) -> ExitCode
where
    C: clap::Parser + Into<Element>,
{
    launch::<C>(std::env::args_os(), options)
}

/// Parses process arguments and renders with the Tokio driver until exit.
///
/// Runs inside a Tokio local runtime or `tokio::task::LocalSet`, as required
/// by [`ink_rs::tokio::render`] and [`ink_rs::Scope::use_future`].
/// Options, completion, and exit handling match [`run`].
#[cfg(feature = "tokio")]
pub async fn run_async<C>() -> ExitCode
where
    C: clap::Parser + Into<Element>,
{
    run_async_from::<C>(std::env::args_os()).await
}

/// Parses arguments, including the program name, and renders with Tokio.
///
/// Runs inside a Tokio local runtime or `tokio::task::LocalSet`. Rendering,
/// completion, and exit handling match [`run_async`].
#[cfg(feature = "tokio")]
pub async fn run_async_from<C>(
    argv: impl IntoIterator<Item = impl Into<OsString> + Clone>,
) -> ExitCode
where
    C: clap::Parser + Into<Element>,
{
    match prepare::<C>(argv) {
        Ok(root) => finish(
            ink_rs::tokio::render(root.into(), Options::default())
                .wait_until_exit()
                .await,
        ),
        Err(code) => code,
    }
}

fn launch<C>(
    argv: impl IntoIterator<Item = impl Into<OsString> + Clone>,
    options: impl FnOnce(&C) -> Options,
) -> ExitCode
where
    C: clap::Parser + Into<Element>,
{
    match prepare::<C>(argv) {
        Ok(root) => {
            let options = options(&root);
            finish(ink_rs::render(root.into(), options).wait_until_exit())
        }
        Err(code) => code,
    }
}

fn prepare<C>(argv: impl IntoIterator<Item = impl Into<OsString> + Clone>) -> Result<C, ExitCode>
where
    C: clap::Parser,
{
    #[cfg(not(feature = "completion"))]
    {
        C::try_parse_from(argv).map_err(clap_exit)
    }
    #[cfg(feature = "completion")]
    {
        let mut command = C::command().subcommand(
            clap::Command::new("completions")
                .about("Print a shell completion script")
                .arg(
                    clap::Arg::new("shell")
                        .value_name("SHELL")
                        .required(true)
                        .value_parser(clap::value_parser!(clap_complete::Shell)),
                ),
        );
        let matches = command.try_get_matches_from_mut(argv).map_err(clap_exit)?;
        if let Some(shell) = matches.subcommand_matches("completions") {
            let shell = *shell
                .get_one::<clap_complete::Shell>("shell")
                .expect("required shell");
            let name = command
                .get_bin_name()
                .unwrap_or(command.get_name())
                .to_owned();
            let mut script = Vec::new();
            clap_complete::generate(shell, &mut command, name, &mut script);
            let code = match std::io::stdout().lock().write_all(&script) {
                Ok(()) => ExitCode::SUCCESS,
                Err(error) => finish(Err(error.into())),
            };
            return Err(code);
        }
        C::from_arg_matches(&matches).map_err(|error| clap_exit(error.format(&mut command)))
    }
}

fn clap_exit(error: clap::Error) -> ExitCode {
    let code = ExitCode::from(error.exit_code() as u8);
    let _ = error.print();
    code
}

fn finish(result: Result<Option<Box<dyn Any + Send>>, Error>) -> ExitCode {
    match result {
        Ok(value) => value
            .and_then(|value| value.downcast::<ExitCode>().ok())
            .map_or(ExitCode::SUCCESS, |code| *code),
        Err(Error::Panic(error)) => std::panic::resume_unwind(error.into_payload()),
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "Error: {error}");
            ExitCode::FAILURE
        }
    }
}
