//! Bordered alerts use Unicode icons without Ink UI's terminal fallbacks.

use ink_rs::{BorderStyle, Color, Element, FlexDirection, Props, Scope, Styled, Text, View};

use super::{Palette, Variant, use_theme};

/// A bordered status message with an optional title.
#[derive(Clone, PartialEq, Props)]
#[props(component = alert)]
pub struct Alert {
    /// The status that selects the icon and border color.
    pub variant: Variant,
    /// A title above the message. Empty titles are omitted.
    pub title: Option<String>,
    /// Inline text elements forming the message.
    pub children: Vec<Element>,
}

impl Alert {
    /// Creates an empty alert with the given status.
    pub fn new(variant: Variant) -> Self {
        Self {
            variant,
            title: None,
            children: Vec::new(),
        }
    }
}

/// Icons, colors, border and spacing for an [`Alert`].
#[derive(Clone, Debug, PartialEq)]
pub struct AlertTheme {
    /// Information icon, `ℹ` by default.
    pub info_icon: String,
    /// Success icon, `✔` by default.
    pub success_icon: String,
    /// Warning icon, `⚠` by default.
    pub warning_icon: String,
    /// Error icon, `✘` by default.
    pub error_icon: String,
    /// Information icon and border color, from [`Palette::info`].
    pub info_color: Color,
    /// Success icon and border color, from [`Palette::success`].
    pub success_color: Color,
    /// Warning icon and border color, from [`Palette::warning`].
    pub warning_color: Color,
    /// Error icon and border color, from [`Palette::error`].
    pub error_color: Color,
    /// Border shape, round by default.
    pub border_style: BorderStyle,
    /// Horizontal padding inside the border, one by default.
    pub padding_x: u16,
    /// Vertical padding inside the border, zero by default.
    pub padding_y: u16,
    /// Cells between the icon and content, one by default.
    pub gap: u16,
    /// Rows between the title and message, one by default.
    pub content_gap: u16,
    /// Whether the title is bold, true by default.
    pub title_bold: bool,
}

impl From<&Palette> for AlertTheme {
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
            border_style: BorderStyle::Round,
            padding_x: 1,
            padding_y: 0,
            gap: 1,
            content_gap: 1,
            title_bold: true,
        }
    }
}

fn alert(cx: &mut Scope, props: &Alert) -> Element {
    let theme = use_theme::<AlertTheme>(cx);
    let (icon, color) = match props.variant {
        Variant::Info => (theme.info_icon, theme.info_color),
        Variant::Success => (theme.success_icon, theme.success_color),
        Variant::Warning => (theme.warning_icon, theme.warning_color),
        Variant::Error => (theme.error_icon, theme.error_color),
    };
    let mut content = View::new()
        .flex_direction(FlexDirection::Column)
        .flex_grow(1.0)
        .flex_shrink(1.0)
        .min_width(0)
        .gap(theme.content_gap);
    if let Some(title) = props.title.as_ref().filter(|title| !title.is_empty()) {
        content = content.child(Text::new(title).bold(theme.title_bold));
    }
    View::new()
        .flex_grow(1.0)
        .border_style(theme.border_style)
        .border_color(color)
        .padding_x(theme.padding_x)
        .padding_y(theme.padding_y)
        .gap(theme.gap)
        .child(
            View::new()
                .flex_shrink(0.0)
                .child(Text::new(icon).color(color)),
        )
        .child(content.child(Text::default().children(props.children.clone())))
        .into()
}
