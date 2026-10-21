//! Progress uses layout metrics instead of Ink UI's render-time state writes.
//! NaN means zero progress, rather than Ink UI's empty output.

use ink_rs::{Color, Element, Props, Scope, Styled, Text, View};

use super::{Palette, use_theme};

/// A percentage bar that fills the available width of its parent.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = progress_bar)]
pub struct ProgressBar {
    /// Percentage clamped to 0 through 100. NaN and the default value mean zero.
    pub value: f64,
}

/// Colors and glyphs for a [`ProgressBar`].
#[derive(Clone, Debug, PartialEq)]
pub struct ProgressBarTheme {
    /// Completed bar color, from [`Palette::accent`].
    pub completed_color: Color,
    /// Whether the remaining bar is dimmed, true by default.
    pub remaining_dim: bool,
    /// A one-cell completed glyph, `█` by default.
    pub completed_character: String,
    /// A one-cell remaining glyph, `░` by default.
    pub remaining_character: String,
}

impl From<&Palette> for ProgressBarTheme {
    fn from(palette: &Palette) -> Self {
        Self {
            completed_color: palette.accent,
            remaining_dim: true,
            completed_character: "█".into(),
            remaining_character: "░".into(),
        }
    }
}

fn progress_bar(cx: &mut Scope, props: &ProgressBar) -> Element {
    let theme = use_theme::<ProgressBarTheme>(cx);
    let node = cx.use_node_ref();
    let width = cx.use_box_metrics(node).width as usize;
    let value = if props.value.is_nan() {
        0.0
    } else {
        props.value.clamp(0.0, 100.0)
    };
    let complete = (value / 100.0 * width as f64).round() as usize;
    let remaining = width - complete;
    let mut bar = View::new().node_ref(node).flex_grow(1.0).min_width(0);
    if complete > 0 {
        bar = bar.child(
            Text::new(theme.completed_character.repeat(complete)).color(theme.completed_color),
        );
    }
    if remaining > 0 {
        bar = bar.child(
            Text::new(theme.remaining_character.repeat(remaining)).dim_color(theme.remaining_dim),
        );
    }
    bar.into()
}
