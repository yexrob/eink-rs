# eink-rs

Components and a CLI framework for [ink-rs](https://github.com/yexrob/ink-rs),
the Rust port of [Ink](https://github.com/vadimdemedes/ink).

- **`eink_rs::ui`** ports the components of
  [Ink UI](https://github.com/vadimdemedes/ink-ui): text, email, password and
  confirm inputs, select and multi-select, spinner, progress bar, badge, status
  message, alert, and ordered and unordered lists.
- **`eink_rs::cli`** will be a CLI framework modeled on
  [Pastel](https://github.com/vadimdemedes/pastel): commands declared as clap
  types, each rendered by a component. Planned.

eink-rs is not published on crates.io yet.

## Examples

`examples/` ports Ink UI 2.0.0's 17 examples under the same names
(theming examples use the `theming-` prefix):

```sh
cargo run --example select
```

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in this crate by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.

The components and their tests port Ink UI. Ink UI is copyright Vadym Demedes
and licensed under the MIT license; see [LICENSE-INK-UI](LICENSE-INK-UI).
