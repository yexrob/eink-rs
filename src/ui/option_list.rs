//! Shared index-based navigation and rows, replacing Ink UI's value-keyed map.

use std::ops::Range;

use ink_rs::{Color, Element, FlexDirection, Styled, Text, View};

/// A labeled value used by [`super::Select`] and [`super::MultiSelect`].
#[derive(Clone, Debug, PartialEq)]
pub struct SelectOption<T> {
    /// Text displayed in the option row.
    pub label: String,
    /// Value returned when this option is selected.
    pub value: T,
}

impl<T> SelectOption<T> {
    /// Creates an option with a label and value.
    pub fn new(label: impl Into<String>, value: T) -> Self {
        Self {
            label: label.into(),
            value,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct OptionList<T> {
    pub options: Vec<SelectOption<T>>,
    pub focused: Option<usize>,
    pub window: Range<usize>,
    pub count: usize,
}

impl<T> OptionList<T> {
    pub fn new(options: Vec<SelectOption<T>>, visible_option_count: usize) -> Self {
        let count = visible_option_count.min(options.len());
        Self {
            focused: (!options.is_empty()).then_some(0),
            options,
            window: 0..count,
            count,
        }
    }

    pub fn next(mut self) -> Self {
        if let Some(index) = self.focused.filter(|index| index + 1 < self.options.len()) {
            let next = index + 1;
            self.focused = Some(next);
            if next >= self.window.end {
                let to = self.options.len().min(self.window.end + 1);
                self.window = to - self.count..to;
            }
        }
        self
    }

    pub fn previous(mut self) -> Self {
        if let Some(index) = self.focused.filter(|index| *index > 0) {
            let previous = index - 1;
            self.focused = Some(previous);
            // Ink UI scrolls when Previous reaches the window's first row.
            if previous <= self.window.start {
                let from = self.window.start.saturating_sub(1);
                self.window = from..from + self.count;
            }
        }
        self
    }

    pub fn focused_option(&self) -> Option<&SelectOption<T>> {
        self.focused.and_then(|index| self.options.get(index))
    }

    pub fn visible_options(&self) -> &[SelectOption<T>] {
        &self.options[self.window.clone()]
    }
}

pub(super) struct RowTheme<'a> {
    pub focus_color: Color,
    pub selected_color: Color,
    pub pointer: &'a str,
    pub tick: &'a str,
    pub highlight_bold: bool,
    pub row_gap: u16,
    pub unfocused_indent: u16,
}

macro_rules! option_theme {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Clone, Debug, PartialEq)]
        pub struct $name {
            /// Color of the focused label and pointer. Defaults to the primary token.
            pub focus_color: ink_rs::Color,
            /// Color of selected labels and ticks. Defaults to the success token.
            pub selected_color: ink_rs::Color,
            /// Focus indicator. Defaults to `❯`.
            pub pointer: String,
            /// Selection indicator. Defaults to `✔`.
            pub tick: String,
            /// Bold style for the first highlight match. Defaults to true.
            pub highlight_bold: bool,
            /// Space between row elements, in cells. Defaults to 1.
            pub row_gap: u16,
            /// Left padding of unfocused rows, in cells. Defaults to 2.
            pub unfocused_indent: u16,
        }

        impl From<&super::Palette> for $name {
            fn from(palette: &super::Palette) -> Self {
                Self {
                    focus_color: palette.primary,
                    selected_color: palette.success,
                    pointer: "❯".into(),
                    tick: "✔".into(),
                    highlight_bold: true,
                    row_gap: 1,
                    unfocused_indent: 2,
                }
            }
        }

        impl $name {
            fn rows(&self) -> super::option_list::RowTheme<'_> {
                super::option_list::RowTheme {
                    focus_color: self.focus_color,
                    selected_color: self.selected_color,
                    pointer: &self.pointer,
                    tick: &self.tick,
                    highlight_bold: self.highlight_bold,
                    row_gap: self.row_gap,
                    unfocused_indent: self.unfocused_indent,
                }
            }
        }
    };
}

pub(super) use option_theme;

pub(super) fn render<T>(
    list: &OptionList<T>,
    is_disabled: bool,
    highlight_text: &str,
    theme: RowTheme<'_>,
    is_selected: impl Fn(&T) -> bool,
) -> Element {
    View::new()
        .flex_direction(FlexDirection::Column)
        .children(
            list.visible_options()
                .iter()
                .enumerate()
                .map(|(offset, option)| {
                    let focused = !is_disabled && list.focused == Some(list.window.start + offset);
                    let selected = is_selected(&option.value);
                    let color = if focused {
                        Some(theme.focus_color)
                    } else if selected {
                        Some(theme.selected_color)
                    } else {
                        None
                    };
                    let mut label = Text::default().color(color);
                    if let Some(index) = option
                        .label
                        .find(highlight_text)
                        .filter(|_| !highlight_text.is_empty())
                    {
                        label = label
                            .child(&option.label[..index])
                            .child(Text::new(highlight_text).bold(theme.highlight_bold))
                            .child(&option.label[index + highlight_text.len()..]);
                    } else {
                        label = label.child(option.label.as_str());
                    }
                    View::new()
                        .gap(theme.row_gap)
                        .padding_left(if focused { 0 } else { theme.unfocused_indent })
                        .child(focused.then(|| Text::new(theme.pointer).color(theme.focus_color)))
                        .child(label)
                        .child(selected.then(|| Text::new(theme.tick).color(theme.selected_color)))
                }),
        )
        .into()
}
