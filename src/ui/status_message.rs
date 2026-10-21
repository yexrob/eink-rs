//! Status messages use Unicode icons without Ink UI's terminal fallbacks.

use ink_rs::{Color, Element, Props, Scope, Styled, Text, View};

use super::{Palette, use_theme};

/// The semantic status of a [`StatusMessage`] or [`super::Alert`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Variant {
    /// Informational content.
    Info,
    /// A successful outcome.
    Success,
    /// A warning that needs attention.
    Warning,
    /// An error or failed outcome.
    Error,
}

/// A status icon followed by a message with a hanging indent.
#[derive(Clone, PartialEq, Props)]
#[props(component = status_message)]
pub struct StatusMessage {
    /// The status that selects the icon and color.
    pub variant: Variant,
    /// Inline text elements forming the message.
    pub children: Vec<Element>,
}

impl StatusMessage {
    /// Creates an empty message with the given status.
    pub fn new(variant: Variant) -> Self {
        Self {
            variant,
            children: Vec::new(),
        }
    }
}

/// Icons, colors and spacing for a [`StatusMessage`].
#[derive(Clone, Debug, PartialEq)]
pub struct StatusMessageTheme {
    /// Information icon, `ℹ` by default.
    pub info_icon: String,
    /// Success icon, `✔` by default.
    pub success_icon: String,
    /// Warning icon, `⚠` by default.
    pub warning_icon: String,
    /// Error icon, `✘` by default.
    pub error_icon: String,
    /// Information color, from [`Palette::info`].
    pub info_color: Color,
    /// Success color, from [`Palette::success`].
    pub success_color: Color,
    /// Warning color, from [`Palette::warning`].
    pub warning_color: Color,
    /// Error color, from [`Palette::error`].
    pub error_color: Color,
    /// Cells between the icon and message, one by default.
    pub gap: u16,
}

impl From<&Palette> for StatusMessageTheme {
    fn from(palette: &Palette) -> Self {
        Self {
            info_icon: "ℹ".into(),
            success_icon: "✔".into(),
            warning_icon: "⚠".into(),
            error_icon: "✘".into(),
            info_color: palette.info,
            success_color: palette.success,
            warning_color: palette.warning,
            error_color: palette.error,
            gap: 1,
        }
    }
}

fn status_message(cx: &mut Scope, props: &StatusMessage) -> Element {
    let theme = use_theme::<StatusMessageTheme>(cx);
    let (icon, color) = match props.variant {
        Variant::Info => (theme.info_icon, theme.info_color),
        Variant::Success => (theme.success_icon, theme.success_color),
        Variant::Warning => (theme.warning_icon, theme.warning_color),
        Variant::Error => (theme.error_icon, theme.error_color),
    };
    View::new()
        .gap(theme.gap)
        .min_width(0)
        .child(
            View::new()
                .flex_shrink(0.0)
                .child(Text::new(icon).color(color)),
        )
        .child(Text::default().children(props.children.clone()))
        .into()
}
