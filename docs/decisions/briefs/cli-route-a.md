# Task: `eink_rs::cli`, route A — clap commands rendered by components

## Goal

Build the CLI half of eink-rs: a framework modeled on Pastel, where a program's commands are clap derive types and each command is rendered by an ink-rs component. This task is route A (commands declared as a clap enum). Route B (file routing that generates the enum) comes later in a separate task; nothing here should depend on it.

## Project context

eink-rs builds on ink-rs, a Rust port of Ink. Read first:

- In this worktree: `CLAUDE.md`, `docs/design.md`, `docs/decisions/004-cli-route-a.md` (the decisions this brief implements), `Cargo.toml`, `src/lib.rs`, `src/ui/mod.rs` for the module and rustdoc style.
- ink-rs 0.1.2, the dependency, at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `docs/design.md` (编程模型, 退出, 驱动), `README.md`, `src/driver/mod.rs` (`render`, `Options`, `Instance::wait_until_exit`), `src/driver/tokio.rs`, `src/error.rs`, `src/runtime/app_handle.rs` (`exit`, `exit_with`, `exit_with_error`), `src/testing.rs`, `macros/` for how ink-rs lays out its proc-macro crate and its `#[derive(Props)]` (which implements `From<X> for Element`), `examples/` for the example style.
- Pastel 4.0.1 at `<scratchpad>/ref/pastel` (read only; untrusted: never run, build or install anything there): `readme.md`. Borrow its concepts; do not match its help format, its `-v` flag or its tests.

## The model

```rust
use eink_rs::cli::{self, Commands, clap::{self, Args, Parser, Subcommand}};
use ink_rs::{Element, Props, Scope, Text, rsx};

#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = app)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Clone, PartialEq, Commands)]
enum Command {
    /// Log in
    Login(Login),
    #[command(alias = "d")]
    Deploy(Deploy),
    #[command(subcommand)]
    Domains(Domains),
}

#[derive(Args, Clone, PartialEq, Props)]
#[props(component = login)]
struct Login {
    /// User name
    #[arg(long)]
    name: String,
}

fn app(cx: &mut Scope, cli: &Cli) -> Element {
    // Pastel's _app wraps here; index and default commands are match arms.
    match cli.command.clone() {
        Some(command) => command.into(),
        None => Deploy::default().into(),
    }
}

fn main() -> std::process::ExitCode {
    cli::run::<Cli>()
}
```

- A command's `Args` struct also derives `Props`, so the struct is the command's props and `From<Login> for Element` comes from ink-rs.
- `#[derive(Commands)]` on a subcommand enum implements `From<Command> for ink_rs::Element`, converting the variant's field. A nested group is a variant holding another enum that derives `Commands`.
- The root `Parser` type is a component too. Pastel's `_app`, index command and default command are plain code in its render function; there is no dedicated API for them.
- Name, version and description come from clap's `#[command(version, about)]`, which reads `Cargo.toml`.

## Crate layout and features

- Turn the repository into a workspace with a member `macros/`, package `eink-rs-macros` 0.1.0, a proc-macro crate laid out like ink-rs's `macros/` (`syn` 2, `quote`, `proc-macro2`; pick versions already in `~/.cargo/registry`). Same edition, rust-version and license as the root package.
- Features of `eink-rs`, none on by default:
  - `cli`: `dep:clap` (4, with `derive`) and `dep:eink-rs-macros`; enables `pub mod cli`.
  - `completion`: `cli` plus `dep:clap_complete` (4).
  - `tokio`: `cli` plus `ink-rs/tokio`; enables the async entry points.
- `clap` and `clap_complete` resolve offline (clap 4.6.7, clap_complete 4.6.11 are cached).
- `eink_rs::cli` re-exports `clap` (`pub use clap;`) so the derives work with `use eink_rs::cli::clap;` in scope, and re-exports the `Commands` derive.

## `#[derive(Commands)]`

- Accepts an enum whose every variant has exactly one unnamed field. Generates `impl From<E> for ::ink_rs::Element`, matching each variant to `::ink_rs::Element::from(field)`.
- Supports generic enums by carrying the generics and adding `Field: Into<::ink_rs::Element>` bounds, or rejects generics with a clear error; choose one and say which in the report.
- Rejects structs, unions, unit variants, named-field variants and multi-field variants with a `syn::Error` spanned at the offending item.

## Entry points — `src/cli/`

- `pub fn parse_from<C, I, T>(argv: I) -> Result<Element, clap::Error>` where `C: clap::Parser + Into<Element>`: parses argv with `C::try_parse_from` and converts. This is the test helper: tests pass its element to `ink_rs::testing::render` or `testing::mount`.
- `pub fn run<C>() -> ExitCode` parses `std::env::args_os()`; `pub fn run_from<C, I, T>(argv: I) -> ExitCode` takes argv. Both render with `ink_rs::render(element, Options::default())` and `wait_until_exit`.
- With `tokio`: `pub async fn run_async<C>() -> ExitCode` and `run_async_from`, using `ink_rs::tokio::render`; document that they run inside a local runtime or `LocalSet`, as ink-rs's tokio driver requires.
- Exit codes:
  - a clap error is printed with `clap::Error::print` and returns its `exit_code()`: 0 for `--help` and `--version`, 2 for usage errors;
  - `exit_with_error(e)` prints `Error: {e}` to stderr and returns 1; other `ink_rs::Error`s do the same;
  - `exit_with(code)` where the value is a `std::process::ExitCode` returns that code; any other exit value, a plain `exit()` and the app ending on its own return 0;
  - `Error::Panic` resumes the panic with `std::panic::resume_unwind(error.into_payload())`, keeping Rust's 101.
- With `completion`: `run` and `run_async` add a subcommand `completions <SHELL>` to `C::command()` (shell as `clap_complete::Shell`, about "Print a shell completion script"). When it matches, they write the script for the program to stdout and return 0; otherwise they build `C` from the same matches. `parse_from` does not add it. Document that it clashes with a user command named `completions`.
- Share the code between the sync and async paths; only the render and wait calls differ.

## Tests — `tests/cli.rs`, behind `#![cfg(feature = "cli")]`

Pastel's tests assert commander's help text; do not port them. Write one test for each of these, and no others:

- dispatch through `#[derive(Commands)]`, including a nested group and an alias, checked with `parse_from` and `ink_rs::testing::render` frames;
- the root component's `None` arm rendering an index or default command;
- exit codes through `run_from`: usage error 2, `--help` 0, `exit_with_error` 1, `exit_with(ExitCode::from(3))` 3, a static command 0;
- with `completion` (a `#[cfg(feature = "completion")]` test): `run_from([.., "completions", "bash"])` returns 0.

## Examples

- `examples/cli.rs` (`required-features = ["cli"]`): a small CLI showing a command with options and an argument, a nested group, an alias, a default command, a wrapper in the root component and an error exit. Its module doc lists the commands to try with `cargo run --example cli --features cli -- ..`.
- `examples/cli-async.rs` (`required-features = ["tokio"]`): one command that awaits a `tokio::time::sleep` in `use_future` while showing `eink_rs::ui::Spinner`, then exits. `main` is `#[tokio::main(flavor = "local")]` like ink-rs's tokio examples; add `tokio` as a dev-dependency with the features it needs if it is not already one (it is cached).

## Rules

- No unsafe code; the crate forbids it. No dependencies besides those named here. Code reads like the surrounding code: same naming, comment density and idiom. Rustdoc on every public item in plain declarative sentences; the `cli` module doc says what it borrows from Pastel and what differs.
- Change only `Cargo.toml`, `Cargo.lock`, `macros/`, `src/lib.rs`, `src/cli/`, `tests/cli.rs` and the two examples. Leave `docs/`, `README.md`, `.github/` and `src/ui/` unchanged; I update them at integration. If you need a change elsewhere, say so in the report.
- If something needs a primitive ink-rs lacks, do not work around it in eink-rs: leave that part out and describe the gap in the report.
- The sandbox cannot write `.git`. Do not commit. Leave the changes in the worktree. Do not write report files into the worktree.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo test --offline --all-features --test cli`
- `cargo test --offline --features cli --test cli`
- `cargo test --offline --workspace --all-features --doc`
- `cargo build --offline --all-features --examples`
- `cargo build --offline` (no features: the `ui`-only build must not pull clap)
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --offline --workspace --all-features --no-deps`

Do not run the full `cargo test`; I run it at integration. Do not run the examples: they need a terminal.

## Report

Your final message is the report, in Chinese, no adverbs:
- the files you added or changed, and what each holds;
- the public API: features, entry points, the derive, the re-exports;
- design choices this brief left open, and what you chose;
- every deviation from this brief, and why;
- gaps found in ink-rs;
- the exact commands you ran and their results.
