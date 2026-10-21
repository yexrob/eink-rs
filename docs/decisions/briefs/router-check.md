# Research: ink-rs behaviors an in-app router depends on

## Goal

eink-rs will add an in-app router in the spirit of React Router: route state and history provided through context, `navigate` and `back`, nested layouts whose state survives navigation while the child route below an outlet changes. Before designing it, confirm by experiment how ink-rs 0.1.3 behaves in each case below, and find any primitive it lacks. Do not design the router and do not change ink-rs.

## Context

- ink-rs at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `src/runtime/hooks.rs` (`use_state`, `use_memo`, `use_effect`, `use_input`, `use_focus`, `provide`, `use_context`, `use_callback`), `Callback`, `component`, `Static`, `testing` (`render`, `mount`, writing input, reading frames), `docs/design.md`.
- eink-rs at `~/Episodes/Projects/bingo-inc/eink-rs` (read only), for how its components use these hooks.

## Setup

Create a scratch package in this directory: `ink-rs = { path = "<ink-rs>", default-features = false, features = ["macros"] }`. Answer each question with a test driven by `ink_rs::testing`, and record what it observed.

## Questions

1. **Layout persistence.** A `Layout` component with its own `use_state` counter renders a page chosen by router state held above it (page A or page B). After switching from A to B: does `Layout` keep its counter? Does A unmount: its effect cleanup runs, its `use_input` handler stops receiving input, its timers or animations stop? Does B start with fresh state?
2. **Same type, new props.** The same page component at the same position with new props (pair ETH, then BTC): is its state kept or reset? Does a changed `key` force a remount?
3. **Context.** A root provides router state that changes on navigation. Does a deep consumer re-render, including below a parent whose own props did not change? How does a descendant change state owned by an ancestor: a `State` handle or a `Callback` passed through context? Are `State<T>` and `Callback` `Clone + PartialEq` and usable as context values? Can a non-`Clone` value, such as a history `Vec`, live in `use_state`, and how is it read and updated?
4. **Navigation from handlers and effects.** Change router state from a `use_input` handler and from an effect. Does the next frame show the new page? Are there stale closures?
5. **Focus.** With `use_focus`, the focused component unmounts on navigation. What has focus afterward? Does Tab cycling still work on the new page? Can a page take focus on mount?
6. **Static.** Does output written through `Static` on one page stay after navigating away?
7. **Route tables.** Can a table of routes hold components of different props types: a function pointer `fn(&mut Scope, &P) -> Element` plus props, an `Element` built ahead of time, or a boxed closure producing an `Element`? Is `Element` `Clone`? What does comparing `Element`s cost on each render?
8. **Anything else** a router would need that ink-rs lacks or makes awkward.

## Rules

- No unsafe code. Do not modify ink-rs or eink-rs. Write only in this directory.
- The sandbox cannot write `.git`. Do not commit.
- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox. Use `cargo test --offline`.

## Report

Your final message is the report, in Chinese, no adverbs. For each question: the test (short code), the observed result, and the conclusion. End with the list of gaps in ink-rs, each with what a router needs and what ink-rs does instead.
