# eink-rs

Components, a CLI framework and an in-app router for
[ink-rs](https://github.com/yexrob/ink-rs), the Rust port of
[Ink](https://github.com/vadimdemedes/ink).

- **`eink_rs::ui`** ports the components of
  [Ink UI](https://github.com/vadimdemedes/ink-ui): text, email, password and
  confirm inputs, select and multi-select, spinner, progress bar, badge, status
  message, alert, and ordered and unordered lists.
- **`eink_rs::cli`** is a CLI framework modeled on
  [Pastel](https://github.com/vadimdemedes/pastel): commands declared as clap
  types, each rendered by a component.
- **`eink_rs::router`** is an in-app router modeled on
  [React Router](https://reactrouter.com/): a route enum with
  `#[derive(Routes)]` or generated from a directory with `routes!`, a history
  with push, replace and back, nested layouts with `Outlet`, typed path
  parameters, and argv as the initial route.
- **`eink_rs::markdown`** renders Markdown and streams model replies: finished
  pieces go to the scrollback through `Static`, the live tail repaints, and
  `markdown-highlight` colors fenced code on a background thread.

| Feature | Enables |
|---|---|
| `cli` | `eink_rs::cli`, clap and `#[derive(Commands)]` |
| `router` | `eink_rs::router`, `#[derive(Routes)]` and `routes!`; with `cli`, `RouteArgs` |
| `completion` | `cli` plus a `completions <SHELL>` subcommand |
| `tokio` | `cli` plus async entry points on ink-rs's tokio driver |
| `markdown` | `eink_rs::markdown` and pulldown-cmark |
| `markdown-highlight` | `markdown` plus code highlighting with syntect |

No feature is on by default; `eink_rs::ui` needs none.

```sh
cargo add eink-rs                 # eink_rs::ui
cargo add eink-rs --features cli  # plus eink_rs::cli
```

## Examples

`examples/` ports Ink UI 2.0.0's 17 examples under the same names
(theming examples use the `theming-` prefix):

```sh
cargo run --example select
```

`cli` and `cli-async` show the CLI framework:

```sh
cargo run --example cli --features cli -- --help
cargo run --example cli-async --features tokio
```

`router` shows the router, and `examples/file-routes/` is a package whose
routes come from the files under `src/routes/`:

```sh
cargo run --example router --features cli,router -- market ETH
cargo run -p file-routes -- market ETH book
```

`examples/ship/` combines all three in a simulated deploy console: directory
routes with nested layouts, a deploy wizard, theme switching, and commands that
print to the normal screen:

```sh
cargo run -p ship
cargo run -p ship -- services api logs
cargo run -p ship -- status
```

`examples/quill/` is a simulated coding assistant that renders inline, as
terminal coding agents do: finished turns go to the scrollback through
`Static`, a multi-line composer is built on the headless input state, and
`exec` prints a turn without interaction:

```sh
cargo run -p quill
cargo run -p quill -- "fix the failing test"
cargo run -p quill -- exec "plan a refactor"
```

`markdown` streams a reply the way a chat client shows it, from a canned
answer or a file:

```sh
cargo run --example markdown --features markdown-highlight
cargo run --example markdown --features markdown-highlight -- --file README.md --rate 4096
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

The components and their tests port Ink UI. Ink UI is copyright Vadym Demedes
and licensed under the MIT license; see [LICENSE-INK-UI](LICENSE-INK-UI).
