# eink-rs

Components and a CLI framework for [ink-rs](https://github.com/yexrob/ink-rs),
the Rust port of [Ink](https://github.com/vadimdemedes/ink).

- **`eink_rs::ui`** ports the components of
  [Ink UI](https://github.com/vadimdemedes/ink-ui): text, email, password and
  confirm inputs, select and multi-select, spinner, progress bar, badge, status
  message, alert, and ordered and unordered lists.
- **`eink_rs::cli`** is a CLI framework modeled on
  [Pastel](https://github.com/vadimdemedes/pastel): commands declared as clap
  types or generated from a directory with `commands!`, each rendered by a
  component.

| Feature | Enables |
|---|---|
| `cli` | `eink_rs::cli`, clap, `#[derive(Commands)]` and `commands!` |
| `completion` | `cli` plus a `completions <SHELL>` subcommand |
| `tokio` | `cli` plus async entry points on ink-rs's tokio driver |

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

`examples/cli-routes/` is a package whose commands come from the files under
`src/commands/`:

```sh
cargo run -p cli-routes -- --help
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

The components and their tests port Ink UI. Ink UI is copyright Vadym Demedes
and licensed under the MIT license; see [LICENSE-INK-UI](LICENSE-INK-UI).
