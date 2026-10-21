//! A badge with a string label instead of Ink UI's React children.

use ink_rs::{Color, Element, Props, Scope, Text};

use super::{Palette, use_theme};

/// An uppercase label on a colored background.
#[derive(Clone, PartialEq, Props)]
#[props(component = badge)]
pub struct Badge {
    /// The label, converted to uppercase when rendered.
    #[props(into)]
    pub label: String,
    /// An override for the theme's background color.
    pub color: Option<Color>,
}

impl Badge {
    /// Creates a badge with the given label.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            color: None,
        }
    }
}

/// Colors for a [`Badge`].
#[derive(Clone, Debug, PartialEq)]
pub struct BadgeTheme {
    /// Background color, from [`Palette::accent`].
    pub background: Color,
    /// Label color, black by default.
    pub label_color: Color,
}

impl From<&Palette> for BadgeTheme {
    fn from(palette: &Palette) -> Self {
        Self {
            background: palette.accent,
            label_color: Color::Black,
        }
    }
}

fn badge(cx: &mut Scope, props: &Badge) -> Element {
    let theme = use_theme::<BadgeTheme>(cx);
    Text::new(" ")
        .background_color(props.color.unwrap_or(theme.background))
        .child(Text::new(props.label.to_uppercase()).color(theme.label_color))
        .child(" ")
        .into()
}
