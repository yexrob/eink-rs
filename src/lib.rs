//! Components, a CLI framework and an in-app router built on
//! [ink-rs](https://docs.rs/ink-rs).
//!
//! [`ui`] ports the components of [Ink UI](https://github.com/vadimdemedes/ink-ui).
//! With the `cli` feature, `cli` renders clap commands through components,
//! borrowing its command model from [Pastel](https://github.com/vadimdemedes/pastel).
//! With the `router` feature, `router` switches pages by a typed route enum,
//! modeled on [React Router](https://reactrouter.com/).

#![forbid(unsafe_code)]

#[cfg(feature = "cli")]
pub mod cli;
#[cfg(feature = "highlight")]
pub(crate) mod highlight;
#[cfg(feature = "markdown")]
pub mod markdown;
#[cfg(feature = "router")]
pub mod router;
pub mod ui;
