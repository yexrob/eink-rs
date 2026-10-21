# Task: `eink_rs::cli::commands!`, route B — commands from a directory

## Goal

Route A (done) declares commands as a clap `Subcommand` enum that derives `Commands`. Route B generates those enums from a directory, as Pastel turns files into commands: each `.rs` file under `src/commands/` is a command, each subdirectory a command group. Implement the `commands!` macro, an example package that uses it, and the tests below.

## Project context

Read first:

- In this worktree: `CLAUDE.md`; `docs/design.md` (its `cli` section); `docs/decisions/004-cli-route-a.md`, `005-cli-route-b-research.md` and `007-cli-route-b.md` (the decisions this brief implements); `Cargo.toml`; `macros/` (the `Commands` derive); `src/cli/mod.rs`; `examples/cli.rs`; `tests/cli.rs`.
- The research report `<scratchpad>/research/route-b-report.md`, sections 2, 4 and 5; its candidate design A is the chosen one. Its scratch crates under `<scratchpad>/research/route-b/`: `pm` shows a proc macro emitting bare `pub mod` lines and the one-line build.rs; `flat` shows the shape of a group with an index.
- ink-rs 0.1.3, the dependency, at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `Props`, `testing::render`.

## What the user writes

```text
examples/cli-routes/
  Cargo.toml
  build.rs                        fn main() { println!("cargo::rerun-if-changed=src/commands"); }
  src/main.rs                     fn main() -> ExitCode { cli::run::<cli_routes::Cli>() }
  src/lib.rs                      the root Parser, its component, and the commands module
  src/commands/deploy.rs          pub struct Deploy    -> `deploy`
  src/commands/domains/index.rs   pub struct Index     -> `domains` with no subcommand
  src/commands/domains/list.rs    pub struct List      -> `domains list`
```

A command file is a route A command:

```rust
use eink_rs::cli::clap::{self, Args};
use ink_rs::{Element, Props, Scope, Text, rsx};

/// Describe a deployment
#[derive(Args, Clone, PartialEq, Props)]
#[command(visible_alias = "d")]
#[props(component = deploy)]
pub struct Deploy {
    /// Project to deploy
    #[arg(default_value = "website")]
    pub project: String,
}

fn deploy(cx: &mut Scope, props: &Deploy) -> Element { .. }
```

The root is written by hand, as in route A:

```rust
pub mod commands {
    eink_rs::cli::commands!("src/commands");
}

#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = app)]
pub struct Cli {
    #[command(subcommand)]
    command: Option<commands::Command>,
}

fn app(cx: &mut Scope, cli: &Cli) -> Element {
    // The wrapper goes here; the default command is the `None` arm.
    match cli.command.clone() {
        Some(command) => command.into(),
        None => commands::deploy::Deploy::default().into(),
    }
}
```

## The macro

`commands!("<dir>")`, with `<dir>` relative to `CARGO_MANIFEST_DIR`. It is called inside an inline module whose files live in `<dir>`, so rustc resolves the bare `pub mod` lines it emits to those files (verified in the research for rustc and rust-analyzer).

For each directory level it emits:

- `pub mod <stem>;` for each `.rs` file, and `pub mod <subdir> { .. }` for each subdirectory, recursively.
- `pub enum Command` with one variant per command at that level, deriving `Subcommand`, `Clone`, `PartialEq` and route A's `Commands`:
  - a file `deploy.rs` gives `Deploy(deploy::Deploy)`;
  - a subdirectory without `index.rs` gives `#[command(subcommand)] Domains(domains::Command)`; called with no subcommand, clap prints its help or usage error (confirm what clap does and report it);
  - a subdirectory with `index.rs` gives `Domains(domains::Group)`;
  - `index.rs` is never a variant.
- In a subdirectory with `index.rs`: `pub struct Group { #[command(flatten)] pub index: index::Index, #[command(subcommand)] pub command: Option<Command> }` with `#[command(args_conflicts_with_subcommands = true)]`, deriving `Args`, `Clone` and `PartialEq`, and `impl From<Group> for ::ink_rs::Element` that renders the subcommand when present and the index otherwise. The index's doc comment and aliases become the group's.

Names: the module is the file stem; the type is its PascalCase (`super_deploy.rs` holds `SuperDeploy`, which clap names `super-deploy`). A stem that is a Rust keyword uses a raw identifier (`type.rs` gives `pub mod r#type;` holding `Type`).

The generated code compiles when the user's crate has no direct `clap` dependency: the clap derives need `clap` in scope, so bring `::eink_rs::cli::clap` into scope in each generated module.

Errors, as `compile_error!` at the macro call, each message naming the offending path:

- a `.rs` stem that is not a lowercase snake_case identifier: uppercase, a hyphen, a leading `_` or digit; the stems `self`, `super`, `crate`; `mod.rs`;
- `foo.rs` next to a directory `foo/`;
- a directory with no commands (`index.rs` alone does not count);
- `index.rs` in `<dir>` itself (the root's index is the root component's `None` arm);
- `<dir>` missing or not a directory;
- `OUT_DIR` unset at expansion: the package has no build script, so an added file would not trigger a rebuild. The message tells the user to add a build.rs printing `cargo::rerun-if-changed=<dir>`. Confirm that a proc macro sees `OUT_DIR` at expansion.

Skip files that do not end in `.rs` and entries starting with `.`. Sort entries by name so the output is deterministic.

## Architecture

- A new module in `macros/src/` for the macro. Split it into reading the directory into a tree (I/O), planning (pure: names, rules, errors) and emitting tokens; the unit tests target planning.
- `eink_rs::cli` re-exports `commands` under the `cli` feature. Its rustdoc covers the layout, naming, index groups, the build.rs line and the errors.
- `Commands` and `cli::run` stay unchanged. If you find they must change, say so in the report instead of changing them.

## Example package — `examples/cli-routes/`

- A workspace member: package `cli-routes`, `publish = false`, same edition, rust-version and license as the root package. Dependencies: `eink-rs` by path with feature `cli`; `ink-rs = { version = "0.1.3", default-features = false, features = ["macros"] }` as eink-rs has it.
- It shows: a command with an alias, options and an argument; the default command and a wrapper in the root component; a group with `index.rs` and two commands; a group without `index.rs`; a group nested two levels deep. Keep the deployment theme of `examples/cli.rs`.
- `src/lib.rs` holds `commands` and `Cli`; `src/main.rs` calls `cli::run::<Cli>()`. The crate doc lists the commands to try with `cargo run -p cli-routes -- ..`.
- The root package must not pick up `examples/cli-routes/` as an example target, and `cargo package -p eink-rs --list` must not include it.

## Tests

Write only these:

- In `macros`: unit tests on planning — name conversion (snake_case to PascalCase, raw keywords) and each error rule above except `OUT_DIR` and the missing directory. One table-driven test for the errors is fine.
- `examples/cli-routes/tests/routes.rs`: dispatch through `cli::parse_from::<Cli>` and `ink_rs::testing::render` frames, as `tests/cli.rs` does, for a leaf, an alias, a group with an index called with and without a subcommand, a nested group, and a group without an index called bare (a clap error); plus the index group's about and alias read from `Cli::command()`.

Check the `OUT_DIR` error by hand: remove the example's build.rs, build, put the error in the report, restore the file. Commit no test for it.

## Rules

- No unsafe code. No dependencies besides those `macros/` already has (`syn`, `quote`, `proc-macro2`) and std. Code reads like the surrounding code: same naming, comment density and idiom. Rustdoc on every public item in plain declarative sentences.
- Change only the root `Cargo.toml` (workspace members), `Cargo.lock`, `macros/`, `src/cli/mod.rs` and `examples/cli-routes/`. Leave `docs/`, `README.md`, `.github/`, `src/ui/`, `tests/` and `examples/*.rs` unchanged; I update docs at integration.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- The sandbox cannot write `.git`. Do not commit. Leave the changes in the worktree. Do not write report files into the worktree.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo test --offline -p eink-rs-macros`
- `cargo test --offline -p cli-routes`
- `cargo test --offline --all-features --test cli` (route A still passes)
- `cargo run --offline -p cli-routes -- --help`, `-- domains --help` and `-- team` (or your group without an index): these print clap output and need no terminal. Do not run commands that render components.
- `cargo build --offline` (no features)
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --offline --workspace --all-features --no-deps`
- `cargo package --offline -p eink-rs --list --allow-dirty`

Do not run the full `cargo test`; I run it at integration.

## Report

Your final message is the report, in Chinese, no adverbs:

- the files you added or changed, and what each holds;
- the public API;
- design choices this brief left open, and what you chose;
- every deviation from this brief, and why;
- what clap does for a bare group without an index, and the `OUT_DIR` error you observed;
- gaps found in ink-rs;
- the exact commands you ran and their results.
