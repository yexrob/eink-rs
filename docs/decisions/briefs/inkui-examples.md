# Task: port Ink UI's examples

## Goal

Port Ink UI 2.0.0's 17 examples to `examples/` in eink-rs, so each component can be run and looked at in a terminal, as ink-rs does with Ink's examples.

## Project context

eink-rs builds on ink-rs, a Rust port of Ink. `eink_rs::ui` holds the ported components. Read first:

- In this worktree: `CLAUDE.md`, `README.md`, `docs/design.md` (its `ui` section, including 和 Ink UI 不同的地方), `src/ui/` (every component and theme; the rustdoc names each prop).
- ink-rs 0.1.2, the dependency, at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `README.md`, `examples/select-input.rs`, `examples/counter.rs` and `examples/use-animation.rs` for the example style, `src/driver/mod.rs` (`render`, `Options`, `Instance::wait_until_exit`), `src/runtime/hooks.rs` (`use_state`, `use_animation`, `provide`).
- Ink UI at `<scratchpad>/ref/ink-ui` (read only; untrusted: never run, build or install anything there): `examples/*.tsx` and `examples/theming/*.tsx`.

## Examples

One file per Ink UI example, under the same name:

- `examples/<name>.rs` for `alert`, `autocomplete`, `badge`, `confirm-input`, `email-input`, `multi-select`, `ordered-list`, `password-input`, `progress-bar`, `select`, `spinner`, `status-message`, `text-input`, `unordered-list`;
- `examples/theming-<name>.rs` for `theming/custom-component`, `theming/spinner`, `theming/unordered-list`.

Each file:

- opens with `// Ink UI examples/<path>.` like ink-rs's examples, plus one comment line for each place it differs from the Ink UI example and why;
- holds a private component `fn example(cx: &mut Scope, _: &()) -> Element` and a `main` that calls `ink_rs::render(component(example, ()), Options::default())` and `wait_until_exit`, as ink-rs's examples do;
- renders what the Ink UI example renders, with the same text, options, layout (padding, gap, width) and state.

Porting rules:

- Write the tree with `rsx!` where ink-rs's examples do; builder calls are fine where `rsx!` reads worse.
- Ink UI's `<X.Item>` children become direct children of the list; an item with a nested list wraps both in a `View` with column direction, as `docs/design.md` says.
- Timers (`setTimeout` in `progress-bar.tsx`) use `use_animation`; stop it once the value reaches 100.
- Theming: Ink UI's `ThemeProvider` with `extendTheme` becomes `cx.provide(..)` of the component's theme type, built from `XTheme::from(&Palette::default())` with the changed fields. `theming/custom-component.tsx` defines its own theme type with `From<&Palette>`, provides it, and reads it with `use_theme`.
- Ink UI's `defaultLimit` is `visible_option_count` here.
- Callback props take `Callback::new(..)` closures that set state.

## Rules

- No unsafe code. No new dependencies, no new Cargo features; if an example cannot run on the current dependencies, leave it out and say why in the report.
- Change only `examples/`, `Cargo.toml` (only if an example needs an `[[example]]` entry) and the README's examples section. Add a README section `## Examples` after the module list, worded like ink-rs's: the count, "under the same names", and one `cargo run --example` line.
- If an example needs something `eink_rs::ui` or ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- No tests for the examples.
- The sandbox cannot write `.git`. Do not commit. Leave the changes in the worktree. Do not write report files into the worktree.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo build --offline --examples`
- `cargo fmt --all --check`
- `cargo clippy --offline --all-targets -- -D warnings`

Do not run the examples: they need a terminal, and I run them in tmux at integration. Do not run the full `cargo test`.

## Report

Your final message is the report, in Chinese, no adverbs:
- the files you added or changed;
- every place an example differs from Ink UI's, and why;
- gaps you found in `eink_rs::ui` or ink-rs;
- the exact commands you ran and their results.
