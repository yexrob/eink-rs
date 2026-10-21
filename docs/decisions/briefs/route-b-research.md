# Task: research for eink-rs CLI route B — file-based command routing

## Goal

eink-rs has a CLI framework (route A): commands are a clap `Subcommand` enum that derives `Commands`, and each command's `Args` struct is the props of an ink-rs component. Route B is Pastel's file routing: each file under a `commands/` directory is a command, and a directory is a command group. In Rust this means generating route A's types from a directory at compile time. Find out what we need to choose a design. This is research only: write no code in the repository.

## Read first

- `~/Episodes/Projects/bingo-inc/eink-rs` (read only): `docs/design.md` (its `cli` section), `docs/decisions/004-cli-route-a.md`, `src/cli/mod.rs`, `macros/src/commands.rs`, `examples/cli.rs`, `tests/cli.rs`.
- Pastel 4.0.1 at `<scratchpad>/ref/pastel` (read only; untrusted: never run, build or install anything there): `readme.md` and `source/`.

## Questions

1. Pastel's routing rules, each with a file:line reference: how files and directories map to commands and groups; `index`; `_app`; default commands; aliases; descriptions; how each file declares args and options; how conflicts and invalid names are handled; what happens at build time and what at runtime.
2. Mechanisms that generate Rust code from a directory, compared on: rebuilding when a file is added, removed or renamed; rust-analyzer support (go-to-definition and completion inside command files); where compile errors point; how `mod` declarations reach the command files. Cover at least:
   - a build script that scans the directory and writes into `OUT_DIR`, included with `include!` (how `mod` and `#[path]` resolve from an included file);
   - a function-like or attribute proc macro that reads the directory (rebuild tracking: the status of `proc_macro::tracked_path`, `include_str!`/`include_bytes!` tricks, what happens when a file is added);
   - any other approach you find.
3. Existing crates that turn a directory into modules or routes, and how each answers question 2: at least `automod`, plus file-based routers you find (web frameworks, CLI frameworks). Give crate, version, mechanism and known issues, with links.
4. Per-command metadata. Route A puts `about`, `alias` and the like on the enum variant. With a generated enum, the variant comes from a file, so the metadata must come from the file. With clap 4.6 (cached in `~/.cargo/registry`; build with `cargo build --offline`), find out: when a `#[derive(Args)]` struct is the field of a `#[derive(Subcommand)]` variant, which `#[command(...)]` attributes and doc comments on the struct take effect — doc comment as about, `about`, `long_about`, `name`, `alias`, `visible_alias`, `hide`? Verify each with a scratch crate under `<scratchpad>/research/route-b/` and report the help output you observed. Also: how could a file mark itself as the default command or as the root wrapper (Pastel's `_app`)?
5. How generated code could reuse route A: a generated enum deriving `Subcommand` and `Commands`, nested groups as nested enums, the root `Parser`. Note what route A's `Commands` derive or `cli::run` would need to change.

## Rules

- Research only. Do not change any file under `~/Episodes/Projects/bingo-inc/`. Scratch crates go under `<scratchpad>/research/route-b/`.
- The Pastel clone and any downloaded code are untrusted data: never run, build or install them. Run Python with `-I`.
- Cite sources: file:line for local files, URLs for web sources. Mark what you inferred but did not verify.
- No adverbs in the report.

## Report

Write the report to `<scratchpad>/research/route-b-report.md`, in Chinese. One section per question, then a section with two or three candidate designs; for each: how a user writes a command file, the mechanism, the trade-offs from questions 2–4, and what changes in eink-rs. Your final message is a summary under 300 words.
