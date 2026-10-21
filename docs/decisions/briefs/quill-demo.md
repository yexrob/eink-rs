# Task: `quill`, an inline coding-assistant TUI example for eink-rs

## Goal

Add the workspace package `examples/quill/` to eink-rs at `<repo>`. `quill` is a fictional AI coding assistant with a scripted, simulated agent. Its terminal UI follows the layout and behavior of OpenAI's Codex CLI, but it carries its own name and no Codex or OpenAI names or branding. It shows the inline style of Ink apps: finished conversation cells go into the terminal's scrollback through ink-rs `Static`, and only the live turn, the status line, popups, the composer and the footer stay in the redrawn area.

The other big example, `examples/ship/`, covers full-screen pages and the router. This one covers inline rendering, streaming, a hand-built multi-line composer on eink-rs's headless input hook, popups, approvals, and a command line with a non-interactive mode.

## Reference: Codex CLI 0.162.1, observed in a terminal

```text
  >_ OpenAI Codex (v0.162.1)
     /path/to/project
  permissions: YOLO mode
  To get started, describe a task or try one of these commands:
  /init - create an AGENTS.md file with instructions for Codex
  /status - show current session configuration
  ...
› Ask Codex to do anything
  GPT-6 Astra · Sub2 high · Context 0% used · 0.162.1
  ← for agents · ? for shortcuts
```

- Typing `/` shows the command list above the composer: one row per command, name padded, then its description; the selected row has a `›` marker.
- `?` on an empty composer shows a shortcuts panel in columns (Compose, Session, Transcript): `/ Commands`, `@ Mention files`, `⌃j New line`, `esc esc Edit last message`, `⌃c Quit`, and so on; `? / esc close` replaces the footer hints.
- Conversation cells, from Codex's TUI: the user message prefixed with `›` on a tinted line; agent text prefixed with `•`; `• Explored` with `└ Read lib.rs` / `Search failing in tests` lines; `• Ran cargo test` with `└` and the first output lines, then `… +12 lines`; `• Edited src/lib.rs (+2 -1)` with a numbered, red/green diff; `• Updated Plan` with `✔` and `□` steps; reasoning summaries dim and italic; `■ Conversation interrupted` after Esc.
- While a turn runs, a status line above the composer reads `• Working (8s • esc to interrupt)`.
- An approval asks `Would you like to run the following command?`, shows `$ cargo test`, and offers `1. Yes, proceed (y)`, `2. Yes, and don't ask again for this command (a)`, `3. No, and tell Codex what to do differently (esc)`.
- After quitting, the terminal keeps the scrollback and prints `To continue this session, run codex resume <id>`, with a token usage line.

Match this look closely with quill's own name, models and wording. Do not copy Codex source; it is not in this environment.

## Project context

Read first:

- `<repo>/README.md`, `<repo>/docs/design.md`, `<repo>/CLAUDE.md`.
- `<repo>/src/cli/mod.rs`: `run_with`, exit handling, `Commands`; `<repo>/examples/cli.rs`.
- `<repo>/src/ui/`: `input.rs` (`InputState`, `InputAction`, `reduce_input`, `use_input_state`), `select.rs`, `option_list.rs` (`use_select_state`), `spinner.rs`, `status_message.rs`, `theme.rs`.
- `<repo>/examples/ship/`: the package layout to follow (Cargo.toml, lib plus bin, the backend trait behind a pointer-equality context handle, `tests/`). Its `commands.rs` shows `Static` and exit handling.
- ink-rs 0.1.3 at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/ink-rs-0.1.3` (read only): `Static`, `View` borders, nested `Text` styling, `Scope` hooks (`use_state`, `use_memo`, `use_effect`, `use_layout_effect`, `use_animation`, `use_input`, `use_paste`, `use_app`, `use_window_size`, `provide`, `use_context`, `use_callback`), `Options` (`exit_on_ctrl_c`), and `testing` (`mount`, `TestTerminal`, `TestApp::input`, `advance`, `exit_result`).

## Command line

```text
quill [PROMPT]                 interactive, inline in the normal screen; PROMPT is sent first
      -m, --model <MODEL>      a model the agent lists; unknown models exit 1
      -a, --ask-for-approval <untrusted|on-request|never>   default on-request
quill exec <PROMPT>            (alias e) run one turn without interaction, print its cells,
                               exit 0, or 1 when the turn fails
quill completions <SHELL>      from eink-rs's `completion` feature
```

- `main` calls `eink_rs::cli::run_with`. No mode uses the alternate screen. The interactive mode sets `exit_on_ctrl_c: false` and handles Ctrl-C itself; `exec` keeps the default.
- `-a` is a clap `ValueEnum`. `-m` is a string checked against the agent's model list, ending with `AppHandle::exit_with_error` when unknown, as `ship` does for service names.
- `exec` runs with approvals `never`.

## Agent

- `src/agent.rs`: an `Agent` trait and its types. It is an event stream, pulled on the app's clock: the app submits a prompt, polls with the elapsed time and receives the events that are due, answers approval requests, and can interrupt. Events cover at least: reasoning summary, agent message delta, exploration (files read, searches), command begin, command output, command end with exit code, file change with a diff, plan update, approval request, token usage, turn complete, turn failed, turn interrupted. The trait also lists models with their reasoning efforts and the workspace's files (for `@`). No threads; the same elapsed times give the same events, so tests can use ink-rs's fake clock.
- A context value wraps `Rc<RefCell<dyn Agent>>` with pointer equality, as `ship`'s `BackendHandle`. The root creates the agent once.
- `src/script.rs`: `ScriptedAgent`, the only implementation, with a small simulated workspace (a Rust crate: `Cargo.toml`, `src/lib.rs`, `src/parse.rs`, `tests/parse.rs`, `README.md`). It picks a script by keywords in the prompt:
  - "test" or "fix": reasoning, explore files, run `cargo test` (fails, with output long enough to truncate), plan, edit with a diff, run `cargo test` again (passes), final message.
  - "explain", "what" or "how": read-only exploration, then a Markdown answer with a list and a fenced code block.
  - "refactor" or "plan": a three-step plan updated as it goes, two edits, one command.
  - "fail": the stream breaks mid-turn; the turn fails with an error.
  - anything else: a short answer.
  - Commands flagged as needing escalation (the test runs, a write) trigger approval under `on-request`; under `untrusted` every command asks; under `never` none does. A declined command ends that step with a message that it was not run, and the agent continues or stops as the script says.
  - Deltas arrive in small chunks over a few seconds, so streaming is visible. A whole script finishes within about ten seconds of simulated time.
- Nothing outside `script.rs` and the root names `ScriptedAgent`.

## Interactive UI

From top to bottom of the terminal:

- **History** (one ink-rs `Static`; ink-rs writes only the first `Static` in the tree): the session card first (a bordered box: `>_ Quill (v0.1.0)`, the process's current directory, model and effort, approval mode, and a few starter commands), then each finished cell. A cell moves into history when its step finishes; the streaming message moves when the turn ends.
- **Live turn**: the cells still streaming.
- **Status line** while a turn runs: an animated marker or Spinner, `Working (Ns • esc to interrupt)`. Esc, or Ctrl-C, interrupts: the turn ends with `■ Conversation interrupted`. Messages submitted while busy are queued and listed under the status line (`↳ queued: ...`), then sent in order after the turn ends.
- **Approval**: replaces the composer while pending. eink-rs `Select` with the three options, plus y, a and n (or Esc) as direct keys. "Don't ask again" remembers the command for the session.
- **Popups**, above the composer, drawn from `use_select_state` (not the `Select` component), Up/Down to move:
  - `/` at the start of the composer, before any space: commands filtered by prefix; Tab completes the selected name; Enter runs it. Commands: `/model` (a `Select` of model and effort pairs, then the footer and later turns use it), `/approvals` (a `Select` of the three modes), `/status` (prints a status cell: model, effort, approvals, directory, session id, token usage), `/diff` (prints every diff the session made, or a note that there is none), `/new` (prints a separator and starts a new session id and token count), `/quit`.
  - `@` starting the word at the cursor: workspace files matching the rest of the word; Tab or Enter replaces the word with the path.
  - `?` on an empty composer: the shortcuts panel; `?` or Esc closes it.
- **Composer** (`src/composer.rs`), a component on eink-rs's `use_input_state` and `reduce_input`, not `TextInput`: a `›` prompt, a dim placeholder (`Ask Quill to do anything`), multi-line text with the cursor drawn inverted. Enter submits; Ctrl-J inserts a newline; Left/Right, Backspace; pasted text (`use_paste`) inserts as is, newlines included; Up/Down recall earlier submissions when the composer is empty or holds a recalled entry. Prompts may use `@path`.
- **Footer**: `quill-1 · high · Context 12% used` on the left and `? for shortcuts` on the right; it shows `ctrl+c again to quit` for two seconds after a first Ctrl-C on an idle, empty composer. Ctrl-C with text in the composer clears it.
- **Quit** (`/quit` or the second Ctrl-C): render a last frame with the token usage line and `To continue this session, run quill resume <session id>`, then exit; the inline frame stays in the terminal. There is no `resume` command; the line is part of the simulation and the crate doc says so.

Cells (`src/cells.rs`) are plain data with one render function, used by the interactive history, the live turn and `exec`. Agent messages render a Markdown subset (`src/markdown.rs`): paragraphs, `**bold**`, `` `code` ``, `-` and numbered lists, fenced code blocks; nothing more.

## Package

- `examples/quill/Cargo.toml`: `name = "quill"`, `publish = false`, edition 2024, rust-version 1.88, as `examples/ship`. Dependencies: `eink-rs = { path = "../..", features = ["cli", "completion"] }` and `ink-rs = { version = "0.1.3", default-features = false, features = ["macros"] }`. Nothing else, no build.rs.
- `src/lib.rs`: the crate doc (what the example shows, the keys, the scripts and the keywords that pick them, and `cargo run -p quill -- ...` lines to try), the modules, `Cli`, `Command`, and the root component. `src/main.rs`: `run_with`.
- Add `examples/quill` to the root `Cargo.toml` workspace members.

## Tests

Exactly two, in `examples/quill/tests/quill.rs`:

1. Interactive turn: mount `quill -a untrusted "fix the failing test"` with `ink_rs::testing::mount`; advance the clock until the approval shows; send `y`; advance until the turn completes. Assert that the terminal output contains the command cell, the diff cell and the final message, and that no approval is pending.
2. `exec` exit codes: `exec "fix the failing test"` ends without error; `exec "fail"` ends with the error that `run` turns into exit code 1.

No other tests: this is an example.

## Rules

- No unsafe code. No dependencies beyond the two above and std.
- Change only `examples/quill/` and the workspace members line of the root `Cargo.toml`. Do not touch eink-rs's `src/`, `macros/`, docs, other examples, or ink-rs. If something needs a primitive that eink-rs or ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- Code reads like the existing examples: naming, comment density, idiom. Doc comments on public items; no comments that restate code.
- The sandbox cannot write `.git`. Do not commit. Do not write report files into `<repo>`.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo build --offline -p quill`
- `cargo fmt --all --check`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo test --offline -p quill` only; the main agent runs the full suite.
- Through a pipe, which renders without a terminal: `cargo run --offline -q -p quill -- exec "fix the failing test" | cat`, `-- exec "explain the parser" | cat`, `-- exec fail; echo $?` (expect 1), `-- -m nope; echo $?` (expect `Error: ...` and 1), `-- --help`, `-- exec --help`, `-- completions zsh | head -3`.
- Do not run the interactive mode; it needs a terminal. The main agent runs it in tmux.

## Report

Your final message is the report, in Chinese, no adverbs:

- the files you added or changed, and what each holds;
- design choices this brief left open, and what you chose;
- every deviation from this brief, and why;
- gaps found in eink-rs or ink-rs;
- the exact commands you ran and their results, with the full output of `exec "fix the failing test"`.
