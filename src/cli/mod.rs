//! A CLI framework borrowing command components, wrappers, and default commands
//! from [Pastel](https://github.com/vadimdemedes/pastel).
//!
//! Commands are clap derive types, declared by hand or wired from files with
//! [`commands!`]. Each command's arguments
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

/// Generates command modules and dispatch types from a directory.
///
/// The path is relative to `CARGO_MANIFEST_DIR`. The macro belongs in an inline
/// module whose Rust module directory matches that path, such as this module in
/// `src/lib.rs` or `src/main.rs`:
///
/// ```ignore
/// pub mod commands {
///     eink_rs::cli::commands!("src/commands");
/// }
/// ```
///
/// Each `.rs` file declares a public clap `Args` type that converts into
/// [`Element`], for example through `ink_rs::Props`. File stems are lowercase
/// snake_case; their types use PascalCase. `deploy.rs` holds `Deploy`, and
/// `super_deploy.rs` holds `SuperDeploy`, exposed as `super-deploy` by clap.
/// Rust keywords use raw module identifiers: `type.rs` becomes `r#type::Type`.
/// Directories follow the same naming rule and become command groups.
/// Hidden entries and files without the `.rs` extension are skipped.
/// Entries are sorted by name before expansion.
///
/// Each level exposes a `Command` enum deriving [`clap::Subcommand`], `Clone`,
/// `PartialEq`, and [`Commands`]. A group's `index.rs` holds `index::Index` and
/// adds a `Group` deriving [`clap::Args`], `Clone`, and `PartialEq`. Its public
/// fields are `index: index::Index` and `command: Option<Command>`. Converting
/// `Group` into an [`Element`] renders the subcommand or, when absent, the index.
/// The index's doc comment and clap aliases supply the group's metadata.
/// Index arguments conflict with subcommands. A group without an index requires
/// a subcommand; a bare invocation produces a clap usage error with help.
/// The root parser and component remain handwritten, including its wrapper and
/// `None` arm for the default command.
///
/// The calling package requires a `build.rs` that tracks the directory:
///
/// ```ignore
/// fn main() {
///     println!("cargo::rerun-if-changed=src/commands");
/// }
/// ```
///
/// This lets Cargo detect added and removed files. Expansion checks `OUT_DIR`
/// to detect a missing build script; it cannot check the script's output.
/// Generated modules import the [`clap`] re-export, so the package does not
/// need a direct clap dependency.
///
/// # Errors
///
/// Expansion reports the offending path for unreadable or missing directories,
/// a path that is not a directory, invalid names (including uppercase letters,
/// hyphens, leading underscores or digits), the reserved stems `self`, `super`,
/// `crate` and `clap` (generated modules import `clap`), `mod.rs`, a file and directory sharing a stem, a directory with
/// no commands (`index.rs` does not count), or an `index.rs` at the root.
/// An unset `OUT_DIR` reports the required build script line.
pub use eink_rs_macros::commands;

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
    match prepare::<C>(argv) {
        Ok(element) => finish(ink_rs::render(element, Options::default()).wait_until_exit()),
        Err(code) => code,
    }
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
        Ok(element) => finish(
            ink_rs::tokio::render(element, Options::default())
                .wait_until_exit()
                .await,
        ),
        Err(code) => code,
    }
}

fn prepare<C>(
    argv: impl IntoIterator<Item = impl Into<OsString> + Clone>,
) -> Result<Element, ExitCode>
where
    C: clap::Parser + Into<Element>,
{
    #[cfg(not(feature = "completion"))]
    {
        parse_from::<C>(argv).map_err(clap_exit)
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
        C::from_arg_matches(&matches)
            .map(Into::into)
            .map_err(|error| clap_exit(error.format(&mut command)))
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
