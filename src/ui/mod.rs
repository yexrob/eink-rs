//! Components ported from [Ink UI](https://github.com/vadimdemedes/ink-ui).
//!
//! Each component comes in three layers: a headless state hook, the
//! component, and a theme. Interactive components read keys while
//! `is_disabled` is false; focus is left to the application, which can derive
//! `is_disabled` from [`Scope::use_focus`](ink_rs::Scope::use_focus).
//!
//! A component reads its theme with [`use_theme`].

mod theme;

pub use theme::{Palette, use_theme};
