//! Components and a CLI framework built on [ink-rs](https://docs.rs/ink-rs).
//!
//! [`ui`] ports the components of [Ink UI](https://github.com/vadimdemedes/ink-ui).
//! With the `cli` feature, `cli` renders clap commands through components,
//! borrowing its command model from [Pastel](https://github.com/vadimdemedes/pastel).

#![forbid(unsafe_code)]

#[cfg(feature = "cli")]
pub mod cli;
pub mod ui;
