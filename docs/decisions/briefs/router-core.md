# Task: `eink_rs::router` — an in-app router with `#[derive(Routes)]`

## Goal

Add an in-app router to eink-rs in the spirit of React Router's `MemoryRouter`: a typed route enum declared with `#[derive(Routes)]` and path strings, a history stack held by a `Router` component, `use_router` for navigation, nested layouts that keep their state while the child route below an `Outlet` changes, path parameters read from context by type, and `RouteArgs` that turns argv words into the initial route. Directory-generated routes come in a later task; nothing here depends on them.

## Project context

Read first:

- In this worktree: `CLAUDE.md`, `docs/design.md`, `docs/decisions/010-router-research.md` and `docs/decisions/011-router.md` (the decisions this brief implements), `Cargo.toml`, `src/lib.rs`, `src/cli/mod.rs` (entry points and exit codes), `macros/src/` (how the `Commands` derive is written and how `fs_commands.rs` unit-tests its planning code), `tests/cli.rs`, `examples/cli.rs`.
- ink-rs 0.1.3 at `~/Episodes/Projects/bingo-inc/ink-rs` (read only): `src/runtime/hooks.rs` (`use_state`, `use_callback`, `provide`, `use_context`, `use_input`), `src/runtime/state.rs`, `src/runtime/context.rs` (the nearest provider wins), `src/element.rs` (`Element::empty`, `component`, `PartialEq`), `macros/` (`#[derive(Props)]` supports generics), `src/testing.rs`.
- A Codex experiment on ink-rs 0.1.3: `<scratchpad>/codex/router-check.report.md` and its crate at `<scratchpad>/research/router-check/`. Facts the design relies on:
  - a layout keeps its state when the child below it changes;
  - a component of the same type at the same position keeps its state when its props change, and a different type remounts;
  - a changed context value re-renders consumers below a memo-skipped parent;
  - `State<T>` is not `PartialEq` and cannot be provided, while a `Callback` can;
  - `State::update` panics during render.

## The model

```rust
use eink_rs::router::{Outlet, Router, Routes, use_param, use_router};

#[derive(Routes, Clone, PartialEq)]
#[layout(Shell)]                        // on the enum: wraps every route of this level
enum Route {
    #[route("/")]
    Home,                               // renders Home::default()
    #[route("/market/:pair")]
    #[layout(MarketFrame)]              // on a variant: wraps its child route
    Market { pair: Pair, page: MarketRoute },
    #[route("/wallet", WalletPage)]     // explicit page type
    Wallet,
}

#[derive(Routes, Clone, PartialEq)]
enum MarketRoute {
    #[route("/")]
    Chart,
    #[route("/book")]
    Book,
}

// "/market/ETH/book" <-> Route::Market { pair: Pair("ETH".into()), page: MarketRoute::Book }

fn market_frame(cx: &mut Scope, _: &MarketFrame) -> Element {
    let Pair(pair) = use_param::<Pair>(cx);
    rsx! { View { Text { "{pair}" } Outlet {} } }
}

fn book(cx: &mut Scope, _: &Book) -> Element {
    let Pair(pair) = use_param::<Pair>(cx);
    let router = use_router::<Route>(cx);
    cx.use_input(true, move |_, key| if key.escape { router.back() });
    ...
}

fn app(cx: &mut Scope, cli: &Cli) -> Element {
    Router::new(cli.route.route.clone()).into()
}
```

## `#[derive(Routes)]` (in `eink-rs-macros`)

- Accepts a non-generic enum. Every variant has exactly one `#[route("...")]`. A tuple variant is an error.
- The path starts with `/`. Segments are static text or `:name`; `"/"` alone is the index of its level. A segment cannot be empty or contain `:` past its first character. A name cannot repeat within one path.
- `#[route("/path", Type)]` names the page type. Otherwise the page type is the variant's identifier, resolved in the enum's module.
- Fields of a named-field variant:
  - each `:name` must have a field of that name, whose type implements `FromStr + Display + Clone + PartialEq + 'static`;
  - at most one field may be left out of the path. That field is the child route, and its type implements `Routes`;
  - a field matched by neither rule is an error.
- `#[layout(Type)]` on a variant is allowed only on a variant with a child. On the enum it wraps every route of that enum. Layouts and pages are built with `Type::default()` and converted with `Into<Element>`, as `rsx!` does.
- Generated:
  - `impl ::eink_rs::router::Routes`;
  - `impl FromStr`, with `Err = ::eink_rs::router::RouteError`, which splits on `/` and ignores empty segments;
  - `impl Display`, which writes the path: `/` for the root index.
- Matching:
  - A leaf variant matches when its pattern consumes every segment. A variant with a child matches its pattern as a prefix and hands the rest, possibly empty, to the child's `from_segments`.
  - A parameter whose `FromStr` fails means the variant does not match, and matching continues.
  - Order: compare patterns segment by segment, and a static segment comes before a parameter. Under the same pattern, a leaf comes before a variant with a child. Anything else keeps declaration order.
  - Two leaf variants with the same pattern are a compile error, comparing parameter positions and ignoring parameter names.
- Rendering one variant: each parameter value is provided as context around everything below it, the layout if any, then the page or child. With a layout, the child's element is provided to the layout's `Outlet`. With no layout, a variant with a child renders the child directly. An enum-level layout wraps the variant's output at the same position for every variant, so it keeps its state across routes of that level. A variant-level layout sees the variant's parameters; an enum-level layout does not.
- Errors are `syn::Error`s spanned at the offending item, with a message naming the rule. Put the path parsing and validation in plain functions with unit tests in the macros crate, as `fs_commands.rs` does.

## Runtime — `src/router/` behind feature `router`

- **Feature**: `router = ["dep:eink-rs-macros"]`. `src/lib.rs` adds `#[cfg(feature = "router")] pub mod router;`. The module re-exports the derive next to the trait, as serde does. `RouteArgs` needs `cli` too.
- **Trait `Routes: Clone + PartialEq + 'static`**: methods to match segments (`&[&str] -> Option<Self>`), to write this route's segments, and to render the route as an `Element`. Their exact signatures are yours; document them, since derived parents call derived children through them. Hidden helper components the generated code needs (a parameter provider, an outlet provider) live in the module under `#[doc(hidden)]`.
- **`RouteError`**: holds the path that did not match, implements `Display` ("no route matches `/x`") and `std::error::Error`.
- **`Router<R>`**: a generic `Props` component, built with `Router::new(initial)`.
  - Its history (`Vec<R>`) is in `use_state`, starting as `[initial]`. A later change to `initial` is ignored, as in `MemoryRouter`.
  - It provides the current route and depth, and a navigation `Callback` from `use_callback` that applies push, replace or back to the history.
  - It renders the current route.
- **`use_router::<R>(cx) -> RouterHandle<R>`**: it panics, naming `R`, when no `Router<R>` is above. `RouterHandle<R>` is `Clone`, so it can move into input handlers. Its methods:
  - `route() -> &R`, the route at render time;
  - `can_go_back()`;
  - `push(R)`: no change when `R` equals the current route;
  - `replace(R)`;
  - `back()`: no change at the bottom of the stack;
  - `push_path(&str) -> Result<(), RouteError>`.
- **`use_param::<T>(cx) -> T`**: the nearest provided parameter of type `T`. It panics, naming `T`, when there is none.
- **`Outlet`**: a `Props` component with `Default`. It renders the nearest child element provided by a layout, or `Element::empty()` when there is none.
- **`RouteArgs<R>`** (`cli` and `router`): a struct with `pub route: R`, implementing `clap::Args`, `clap::FromArgMatches`, `Clone` and `PartialEq`, so a root `Parser` can `#[command(flatten)]` it.
  - It adds a positional `[ROUTE]...` that takes the remaining words; each word is one path segment.
  - It builds `R::from_segments` from them. When nothing matches, it returns a clap error of kind `InvalidValue` naming the path, so `cli::run` prints it and exits 2.
  - No words means the root index.
  - A clap subcommand of the same root still takes precedence; check this in a test.
- Navigation happens in input handlers and effects. Never call `State::update` during render.
- Document in the module rustdoc, since the router does not handle them:
  - The app binds its own back key.
  - The key that triggers navigation also reaches the other input handlers that were registered when the key arrived.
  - A focused component that unmounts leaves focus empty, so the new page's `auto_focus` applies. When a layout holds focus, the page calls `focus(id)`.
  - A page remounts when it is navigated back to.

## Tests — `tests/router.rs`, behind `#![cfg(feature = "router")]`

Each test guards behavior the router promises. Write these and no others:

1. Parsing and printing:
   - `FromStr` and `Display` round-trip through the root index, a static route, a parameter, a nested child and a nested index.
   - A static segment wins over a parameter.
   - A failing parameter `FromStr` falls through to the next match.
   - An unmatched path returns `RouteError`.
2. History through `ink_rs::testing::render` and input:
   - push, replace and back change the rendered page;
   - `can_go_back` is reported;
   - pushing the current route adds nothing;
   - back at the bottom stays.
3. Layouts:
   - A variant-level layout's own `use_state` survives navigation between its children.
   - An enum-level layout survives navigation between variants.
   - A page navigated away from and back to starts fresh.
4. Parameters:
   - `use_param` reads the right value in a layout and in a page two levels down.
   - Changing only the parameter (ETH to BTC) updates the reader and keeps the layout's state.
5. Argv, with `cli` (`#[cfg(feature = "cli")]` inside the file):
   - `cli::parse_from` turns `["app", "market", "ETH", "book"]` into the expected frame.
   - `cli::run_from` with an unmatched route returns exit code 2.
   - A subcommand next to the flattened `RouteArgs` still dispatches.

The macros crate gets unit tests for path parsing, ordering and each validation error.

## Example — `examples/router.rs`

`required-features = ["cli", "router"]`:

- Holds a few routes: a root layout with a header showing the current path, a nested layout with a parameter and a counter in its own state, and two pages under it.
- Binds Esc to `back` and q to exit in the root layout.
- Navigates with keys.
- Takes argv as the initial route.
- Its module doc lists the routes and commands to try: `cargo run --example router --features cli,router -- market ETH`.

## Rules

- No unsafe code; the crates forbid it.
- No new dependencies.
- Code reads like the surrounding code: same naming, comment density and idiom.
- Rustdoc on every public item, in plain declarative sentences.
- Change only:
  - `Cargo.toml`, `Cargo.lock`;
  - `macros/src/` (new `routes.rs` plus `lib.rs`), `macros/Cargo.toml` (description only);
  - `src/lib.rs`, `src/router/`;
  - `tests/router.rs`, `examples/router.rs`.
- Leave `commands!`, `fs_commands.rs`, `examples/cli-routes`, `docs/` and `README.md` alone; another task and I handle them. If you need a change elsewhere, say so in the report.
- If something needs a primitive ink-rs lacks, do not work around it in eink-rs: leave that part out and describe the gap in the report.
- The sandbox cannot write `.git`. Do not commit. Leave the changes in the worktree, and do not write report files there.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo test --offline --features router --test router`
- `cargo test --offline --features cli,router --test router`
- `cargo test --offline -p eink-rs-macros`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo build --offline --example router --features cli,router`

Do not run the full test suite; I run it at integration.

## Report

Your final message is the report, in Chinese, no adverbs. It covers:

- the files changed and the public API with signatures;
- the trait's method signatures and why you chose them;
- the ordering rule as you implemented it;
- the test list and its results;
- any deviation from this brief, and any ink-rs gap you hit.
