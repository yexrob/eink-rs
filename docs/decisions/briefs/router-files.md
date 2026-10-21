# Task: `eink_rs::router::routes!` — route enums from a directory, and the removal of `commands!`

## Goal

`#[derive(Routes)]` (done) declares in-app routes as nested enums. Add `routes!("src/routes")`, which generates those enums and the page modules from a directory, in the spirit of Next.js file routing. At the same time delete the CLI's directory macro `commands!` and its example package: the CLI keeps only `#[derive(Commands)]`.

## Project context

Read first:

- In this worktree: `CLAUDE.md`, `docs/decisions/011-router.md` (the decisions this brief implements; its 方案 step 2 lists the details), `docs/design.md` (the `cli` section's 路由 B part describes `commands!`), `Cargo.toml`, `macros/src/lib.rs`, `macros/src/fs_commands.rs` (scanning, naming, the `OUT_DIR` check and their unit tests), `macros/src/routes.rs` (the derive this macro emits), `src/router/mod.rs`, `src/cli/mod.rs` (the `commands!` re-export and its docs), `examples/cli-routes/` (the package being replaced), `examples/router.rs` and `tests/router.rs` (the same routes written by hand).
- ink-rs 0.1.3 at `~/Episodes/Projects/bingo-inc/ink-rs` (read only), if you need the runtime.

A check done before this task: a proc macro that emits `#[path = "[pair]"] pub mod pair { pub mod index; pub mod book; }` inside an inline module compiles with rustc, and rust-analyzer 1.96 loads the files under `[pair]/`.

## The model

```
src/routes/
  _layout.rs        Layout: wraps every route of this level
  index.rs          Index: /
  wallet.rs         Wallet: /wallet
  market/
    index.rs        Index: /market
    [pair]/
      _layout.rs    Layout: wraps /market/:pair/...
      index.rs      Index: /market/:pair
      book.rs       Book: /market/:pair/book
```

```rust
pub mod routes {
    eink_rs::router::routes!("src/routes");
}
```

expands to the equivalent of:

```rust
pub mod _layout;
pub mod index;
pub mod wallet;
pub mod market {
    pub mod index;
    #[path = "[pair]"]
    pub mod pair {
        /// The `:pair` path parameter.
        #[derive(Clone, Debug, PartialEq, Eq, Hash)]
        pub struct Pair(pub String);
        // impl FromStr (Err = Infallible) and Display for Pair
        pub mod _layout;
        pub mod index;
        pub mod book;
        #[derive(::eink_rs::router::Routes, Clone, Debug, PartialEq)]
        #[layout(_layout::Layout)]
        pub enum Route {
            #[route("/", index::Index)]
            Index,
            #[route("/book", book::Book)]
            Book,
        }
    }
    #[derive(::eink_rs::router::Routes, Clone, Debug, PartialEq)]
    pub enum Route {
        #[route("/", index::Index)]
        Index,
        #[route("/:pair")]
        Pair { pair: pair::Pair, route: pair::Route },
    }
}
#[derive(::eink_rs::router::Routes, Clone, Debug, PartialEq)]
#[layout(_layout::Layout)]
pub enum Route {
    #[route("/", index::Index)]
    Index,
    #[route("/market")]
    Market { route: market::Route },
    #[route("/wallet", wallet::Wallet)]
    Wallet,
}
```

A page under `[pair]/` reads the parameter with `use_param::<super::Pair>(cx)`.

## `routes!` (in `eink-rs-macros`, re-exported as `eink_rs::router::routes`)

- `routes!("<dir>")`, `<dir>` relative to `CARGO_MANIFEST_DIR`, called inside an inline module whose module directory is `<dir>`, as `commands!` is today. The package needs the same `build.rs` with `cargo::rerun-if-changed=<dir>`, and expansion keeps the `OUT_DIR` check with the message adapted.
- Entries are scanned and sorted as `fs_commands.rs` does: hidden entries and non-`.rs` files are skipped.
- Each level:
  - `foo.rs` is a page at `/foo`. Module `foo`, page type `foo::Foo`, variant `Foo`.
  - `index.rs` is the page at this level's `/`. Module `index`, page type `index::Index`, variant `Index`. It is allowed at the root.
  - `foo/` is a static child at `/foo`: an inline `pub mod foo { ... }` with its own `Route`, and the variant `Foo { route: foo::Route }`.
  - `[pair]/` is a dynamic child at `/:pair`: `#[path = "[pair]"] pub mod pair { ... }` with its own `Route` and the newtype `Pair(pub String)`, implementing `FromStr` (`Err = core::convert::Infallible`) and `Display`. The variant is `Pair { pair: pair::Pair, route: pair::Route }`.
  - `_layout.rs` is the layout of this level: module `_layout`, type `_layout::Layout`, emitted as `#[layout(_layout::Layout)]` on this level's enum.
  - The enum is `pub enum Route`, deriving `::eink_rs::router::Routes`, `Clone`, `Debug` and `PartialEq`. The derive already orders matches, so emit variants in scan order.
- Naming: file stems and directory names (the name inside `[...]` for dynamic ones) follow `plan_name`: lowercase snake_case, keywords as raw identifiers, PascalCase types. The reserved list becomes `self`, `super`, `crate` and `route`; `clap` is no longer reserved, since the generated code imports nothing.
- Errors, each a `compile_error!` at the macro call naming the path and the rule:
  - an invalid name;
  - `mod.rs`;
  - `[pair].rs`: dynamic segments are directories;
  - an `index/` directory;
  - an underscore-prefixed file other than `_layout.rs`, and any underscore-prefixed directory;
  - two entries with the same module name, such as `foo.rs` and `foo/`, or `pair.rs` and `[pair]/`;
  - two dynamic directories in one directory;
  - `[route]/`: the child field is named `route`;
  - a directory with no page and no child directory (`_layout.rs` does not count).
- Generated items carry `#[doc = ...]` docs, as `fs_commands.rs` does.
- Rename `fs_commands.rs` to `fs_routes.rs` and adapt it; keep its structure and the unit-test style.

## Removing `commands!`

- Delete the `commands` proc macro from `macros/src/lib.rs`, the re-export and its docs from `src/cli/mod.rs`, and any rustdoc mention of directory commands in `src/cli/`. The `cli` module doc says commands are clap derive types dispatched by `Commands`.
- Delete the `examples/cli-routes/` package and its workspace member.
- `macros/Cargo.toml` description: "The Commands and Routes derives and the routes! macro for eink-rs". `macros/src/lib.rs`'s crate doc follows.
- Do not bump versions; the release is decided later.

## Example package — `examples/file-routes/`

- A workspace member like the deleted `cli-routes`: package `file-routes`, `publish = false`, same edition, rust-version and license as the root package. It depends on `eink-rs` by path with `cli` and `router`, and on `ink-rs` as `cli-routes` did. A `build.rs` tracks `src/routes`.
- The root package must not pick it up as an example target, and `cargo package -p eink-rs --list` must not include it; `cli-routes` handled both, keep that.
- `src/lib.rs` holds `pub mod routes { ... }` and `Cli` (`#[command(version, about)]`, flattening `RouteArgs<routes::Route>`); `src/main.rs` calls `cli::run::<Cli>()`. The crate doc lists the routes and the commands to try.
- `src/routes/` is the tree in the model above, with the same behavior as `examples/router.rs`: the root layout shows the path and binds Esc to back, q to exit and keys to navigate; the `[pair]` layout shows the pair and a counter in its own state, and switches pages and pairs with keys.
- `examples/file-routes/tests/routes.rs`, two tests:
  - `"/market/ETH/book"` parses into the expected nested value, and `Display` gives the path back;
  - `cli::parse_from::<Cli>(["file-routes", "market", "ETH", "book"])` renders the expected frame, showing both layouts and the parameter.

## Tests

- `macros/src/fs_routes.rs` unit tests: the mapping of each entry kind (page, index, static child, dynamic child, layout), names including a raw identifier, and each error above.
- No other tests: the derive and runtime are covered by `tests/router.rs`.

## Rules

- No unsafe code; the crates forbid it.
- No new dependencies.
- Code reads like the surrounding code: same naming, comment density and idiom.
- Rustdoc on every public item, in plain declarative sentences. `routes!`'s rustdoc lives on the re-export in `src/router/mod.rs`, as `commands!`'s did in `src/cli/mod.rs`: layout, naming, parameters, layouts, the build.rs requirement and the errors.
- Change only:
  - the root `Cargo.toml` (workspace members), `Cargo.lock`;
  - `macros/` (`lib.rs`, `fs_commands.rs` → `fs_routes.rs`, `Cargo.toml` description);
  - `src/cli/mod.rs`, `src/router/mod.rs`;
  - delete `examples/cli-routes/`, add `examples/file-routes/`.
- Leave `docs/` and `README.md` alone; I update them. If you need a change elsewhere, say so in the report.
- If something needs a primitive ink-rs lacks, do not work around it: leave that part out and describe the gap in the report.
- The sandbox cannot write `.git`. Do not commit. Leave the changes in the worktree, and do not write report files there.

## Commands

- `export PATH=$HOME/.cargo/bin:$PATH` first; the mise shim cannot write its cache in the sandbox.
- `cargo test --offline -p eink-rs-macros`
- `cargo test --offline -p file-routes`
- `cargo clippy --offline --workspace --all-targets --all-features -- -D warnings`
- `cargo package -p eink-rs --list --offline --allow-dirty | grep -c file-routes` prints 0.
- `cargo run --offline -p file-routes -- --help` and `-- market ETH book < /dev/null`: put the outputs in the report.
- Check the `OUT_DIR` error by hand: remove the example's build.rs, build, put the error in the report, restore the file.
- Run `cargo clean` at the end.

Do not run the full test suite; I run it at integration.

## Report

Your final message is the report, in Chinese, no adverbs. It covers:

- the files changed, added and deleted;
- the expansion for the example tree, as the model above shows it;
- the error list with one message each;
- the test list and its results, and the command outputs asked for above;
- any deviation from this brief, and any ink-rs gap you hit.
