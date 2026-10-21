//! Components ported from [Ink UI](https://github.com/vadimdemedes/ink-ui).
//!
//! Each component comes in three layers: a headless state hook, the
//! component, and a theme. Interactive components read keys while
//! `is_disabled` is false; focus is left to the application, which can derive
//! `is_disabled` from [`Scope::use_focus`](ink_rs::Scope::use_focus).
//!
//! A component reads its theme with [`use_theme`].

mod alert;
mod badge;
mod ordered_list;
mod progress_bar;
mod spinner;
mod status_message;
mod theme;
mod unordered_list;

pub use alert::{Alert, AlertTheme};
pub use badge::{Badge, BadgeTheme};
pub use ordered_list::{OrderedList, OrderedListTheme};
pub use progress_bar::{ProgressBar, ProgressBarTheme};
pub use spinner::{Spinner, SpinnerFrames, SpinnerTheme, use_spinner};
pub use status_message::{StatusMessage, StatusMessageTheme, Variant};
pub use theme::{Palette, use_theme};
pub use unordered_list::{UnorderedList, UnorderedListTheme};
