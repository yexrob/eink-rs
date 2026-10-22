use ink_rs::Color;

use crate::ui::Palette;

/// Colors and characters for [`super::Markdown`].
#[derive(Clone, Debug, PartialEq)]
pub struct MarkdownTheme {
    /// Heading text, from [`Palette::primary`].
    pub heading_color: Color,
    /// Inline code, cyan by default.
    pub code_color: Color,
    /// Code blocks without highlighting, cyan by default.
    pub code_block_color: Color,
    /// Link text, from [`Palette::info`].
    pub link_color: Color,
    /// Block quote bars, gray by default.
    pub quote_color: Color,
    /// List markers, gray by default.
    pub bullet_color: Color,
    /// Bullets by nesting depth, repeating past the last one.
    pub bullets: Vec<String>,
    /// Marker of a checked task, `[x]` by default.
    pub task_done: String,
    /// Marker of an open task, `[ ]` by default.
    pub task_open: String,
    /// Thematic breaks and table borders, gray by default.
    pub rule_color: Color,
    /// `[!NOTE]` alerts, from [`Palette::info`].
    pub note_color: Color,
    /// `[!TIP]` alerts, from [`Palette::success`].
    pub tip_color: Color,
    /// `[!IMPORTANT]` alerts, from [`Palette::accent`].
    pub important_color: Color,
    /// `[!WARNING]` alerts, from [`Palette::warning`].
    pub warning_color: Color,
    /// `[!CAUTION]` alerts, from [`Palette::error`].
    pub caution_color: Color,
    /// Emits OSC 8 hyperlinks. When false, a link shows its URL after the text.
    pub hyperlinks: bool,
    /// The highlighting theme, a bat theme name. `ansi` by default, which
    /// takes the terminal's own 16 colors.
    #[cfg(feature = "markdown-highlight")]
    pub code_theme: String,
}

impl From<&Palette> for MarkdownTheme {
    fn from(palette: &Palette) -> Self {
        Self {
            heading_color: palette.primary,
            code_color: Color::Cyan,
            code_block_color: Color::Cyan,
            link_color: palette.info,
            quote_color: Color::Gray,
            bullet_color: Color::Gray,
            bullets: vec!["•".into(), "◦".into(), "▪".into()],
            task_done: "[x]".into(),
            task_open: "[ ]".into(),
            rule_color: Color::Gray,
            note_color: palette.info,
            tip_color: palette.success,
            important_color: palette.accent,
            warning_color: palette.warning,
            caution_color: palette.error,
            hyperlinks: true,
            #[cfg(feature = "markdown-highlight")]
            code_theme: "ansi".into(),
        }
    }
}
