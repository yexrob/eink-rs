# Task: rewrite the web3 market demo as a Pastel-style CLI on eink-rs

## Goal

`<demo>` is a full-screen market terminal on simulated data, built on ink-rs alone. Rewrite it as a CLI named `web3` on eink-rs's CLI framework, with commands generated from `src/commands/` (route B, `eink_rs::cli::commands!`). The full-screen terminal stays as the default command; the other commands print one view of the same data, follow it live, or run an interactive swap with eink-rs components.

## Project context

Read first:

- `<demo>`: every file. `feed.rs` is the data interface the UI reads through; `sim.rs` its simulated implementation; `ui.rs`, `chart.rs` and `canvas.rs` the screen; `format.rs` number formatting. The original is committed in `<demo>`'s git history.
- eink-rs at `<eink-rs>` (read only): `README.md`; `docs/design.md`, its `cli` and `ui` sections; `src/cli/mod.rs`, including `run_with`; `src/ui/` (Select, TextInput, ConfirmInput, Spinner, StatusMessage, Badge and the others); the route B example `examples/cli-routes/`.
- ink-rs 0.1.3 at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `Scope` hooks (`provide`, `use_context`, `use_state`, `use_memo`, `use_animation`, `use_input`, `use_app`), `AppHandle::exit_with_error`, and `Static`.

## Commands

```text
web3                         the default: same as `web3 watch`
web3 watch [PAIR] -i 5s      the full-screen terminal of the original demo
web3 price [PAIRS..]         a quote table; --follow keeps it live
web3 book <PAIR> -d 10       an order book snapshot
web3 trades <PAIR> -n 20     recent trades; --follow streams new ones
web3 chain                   network, block and gas
web3 chain events -n 10      recent chain events; --follow streams new ones
web3 wallet                  holdings, total value and 24h profit and loss
web3 swap                    interactive: pick assets, enter an amount, confirm,
                             wait on a Spinner, see the result
web3 completions <SHELL>     from eink-rs's `completion` feature
```

Files: `src/commands/{watch,price,book,trades,wallet,swap}.rs`, `src/commands/chain/{index,events}.rs`.

- `watch`: the original screen and keys. `PAIR` picks the starting pair (default ETH), `-i/--interval` the starting candle length, one of the existing intervals (`1s`, `5s`, `15s`, `1m`, `5m`), parsed by clap.
- `price`: with no pairs, every ticker. Columns as the original watchlist plus 24h high, low and volume.
- `book`, `trades`, `chain`, `chain events`, `wallet`: one view in the original's colors and formats, then exit. Reuse the original's drawing (order book canvas, trade rows, event tags, wallet totals) instead of copying it.
- `--follow`: the view updates as the feed moves, until `q`, Esc or Ctrl-C. For streams (trades, events), append new lines as permanent output with ink-rs `Static`, the Ink idiom for logs, if it fits; say in the report what you chose.
- `swap`: pick the asset to sell from the wallet's holdings (Select), the asset to buy (Select), enter an amount (TextInput, a positive number up to the holding), see a quote (rate, amount received, a simulated fee), confirm (ConfirmInput), then a Spinner for about a second and a success StatusMessage with a simulated transaction hash. Declining ends with a StatusMessage saying the swap was cancelled, exit code 0. Nothing is sent anywhere; the screen stays labeled as simulated, as the original header is.
- A pair argument matches a ticker symbol, ignoring case. An unknown pair ends with `AppHandle::exit_with_error`, so the process prints `Error: ...` listing the known pairs and exits with 1. The symbols come from the feed, not from clap.

## Architecture

- Package `web3-market` stays; add `[[bin]] name = "web3"`. Dependencies: `eink-rs = { path = "<eink-rs>", features = ["cli", "completion"] }` and `ink-rs = { version = "0.1.3", default-features = false, features = ["macros"] }`. Add the one-line build.rs that route B requires.
- `src/main.rs`: the shared modules, `pub mod commands { eink_rs::cli::commands!("src/commands"); }`, the root `Cli` (`#[command(name = "web3", version, about)]`, `Option<commands::Command>`), and `main` calling `eink_rs::cli::run_with::<Cli>(..)`. The closure turns on `alternate_screen` for `watch` and for no command, and leaves the default options for the rest, so their output stays in the terminal's scrollback.
- The root component creates the feed once and provides it through ink-rs context; commands take it with `use_context`. The context value holds `Rc<RefCell<dyn Feed>>`, as the original `Props` does. Commands read data only through `Feed`; nothing outside `sim.rs` and `main.rs` names `Simulator`.
- Split `ui.rs` where commands share its pieces: shared colors and formatting, the full screen, and the parts the one-shot views reuse. Keep `feed.rs`, `sim.rs`, `chart.rs`, `canvas.rs` and `format.rs` unless a command needs a change; say what changed and why.
- The crate doc in `main.rs` lists the commands to try.

## Rules

- No unsafe code. No dependencies beyond the two above and std. Code reads like the existing code: same naming, comment density and idiom.
- Do not write tests. The original has none; this is a demo.
- Change only files under `<demo>`. Do not touch `<eink-rs>` or ink-rs. If something needs a primitive eink-rs or ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- The sandbox cannot write `.git`. Do not commit. Do not write report files into `<demo>`.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo build --offline`
- `cargo fmt --check`
- `cargo clippy --offline --all-targets -- -D warnings`
- One-shot commands through a pipe, which renders without a terminal: `cargo run --offline -q -- price | cat`, and the same for `price btc sol`, `book eth -d 5`, `trades sol -n 5`, `chain`, `chain events -n 5`, `wallet`; `-- price nope` (expect `Error: ...` and exit 1); `-- --help`, `-- chain --help`, `-- completions zsh | head -3`.
- Time `target/debug/web3 price | cat` and report it: the simulator builds four hours of history at start.
- Do not run `watch`, the bare `web3`, `--follow` or `swap`; they need a terminal. I run them in tmux.

## Report

Your final message is the report, in Chinese, no adverbs:

- the files you added or changed, and what each holds;
- design choices this brief left open, and what you chose;
- every deviation from this brief, and why;
- gaps found in eink-rs or ink-rs;
- the exact commands you ran and their results, with the output of `price` and `book eth -d 5`.
