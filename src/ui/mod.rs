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
mod confirm_input;
mod email_input;
mod input;
mod multi_select;
mod option_list;
mod ordered_list;
mod password_input;
mod progress_bar;
mod select;
mod spinner;
mod status_message;
mod text_input;
mod theme;
mod unordered_list;

pub use alert::{Alert, AlertTheme};
pub use badge::{Badge, BadgeTheme};
pub use confirm_input::{ConfirmChoice, ConfirmInput, ConfirmInputTheme};
pub use email_input::EmailInput;
pub use input::{InputAction, InputState, InputTheme, reduce_input, use_input_state};
pub use multi_select::{MultiSelect, MultiSelectState, MultiSelectTheme, use_multi_select_state};
pub use option_list::SelectOption;
pub use ordered_list::{OrderedList, OrderedListTheme};
pub use password_input::PasswordInput;
pub use progress_bar::{ProgressBar, ProgressBarTheme};
pub use select::{Select, SelectState, SelectTheme, use_select_state};
pub use spinner::{Spinner, SpinnerFrames, SpinnerTheme, use_spinner};
pub use status_message::{StatusMessage, StatusMessageTheme, Variant};
pub use text_input::TextInput;
pub use theme::{Palette, use_theme};
pub use unordered_list::{UnorderedList, UnorderedListTheme};
