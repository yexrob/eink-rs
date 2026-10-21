//! Spinners support dots and custom frames instead of the cli-spinners catalog.
//! Changing frames restarts the animation, fixing Ink UI's stale frame index.

use std::time::Duration;

use ink_rs::{AnimationOptions, Color, Element, Props, Scope, Styled, Text, View};

use super::{Palette, use_theme};

/// The frames and tick interval of a spinner.
#[derive(Clone, Debug, PartialEq)]
pub struct SpinnerFrames {
    /// Frames in playback order. An empty list renders no frame and has no timer.
    pub frames: Vec<String>,
    /// Time between frames, clamped by [`Scope::use_animation`].
    pub interval: Duration,
}

impl SpinnerFrames {
    /// Creates a spinner from custom frames and an interval.
    pub fn new<I, S>(frames: I, interval: Duration) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            frames: frames.into_iter().map(Into::into).collect(),
            interval,
        }
    }

    /// Creates cli-spinners' ten-frame dots animation with an 80 ms interval.
    pub fn dots() -> Self {
        Self::new(
            ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"],
            Duration::from_millis(80),
        )
    }
}

impl Default for SpinnerFrames {
    fn default() -> Self {
        Self::dots()
    }
}

/// Returns the current frame, restarting before paint when the frames change.
/// An empty frame list returns an empty string and does not subscribe to ticks.
pub fn use_spinner(cx: &mut Scope, frames: &SpinnerFrames) -> String {
    let animation = cx.use_animation(AnimationOptions {
        interval: frames.interval,
        active: !frames.frames.is_empty(),
    });
    cx.use_layout_effect(frames.frames.clone(), move || animation.reset());
    if frames.frames.is_empty() {
        String::new()
    } else {
        frames.frames[(animation.frame % frames.frames.len() as u64) as usize].clone()
    }
}

/// An animated frame followed by an optional label.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = spinner)]
pub struct Spinner {
    /// Label shown when non-empty, empty by default.
    #[props(into)]
    pub label: String,
    /// Animation frames and interval, dots by default.
    pub frames: SpinnerFrames,
}

/// Colors and spacing for a [`Spinner`].
#[derive(Clone, Debug, PartialEq)]
pub struct SpinnerTheme {
    /// Frame color, from [`Palette::primary`].
    pub frame_color: Color,
    /// Label color, inherited by default.
    pub label_color: Option<Color>,
    /// Cells between frame and label, one by default.
    pub gap: u16,
}

impl From<&Palette> for SpinnerTheme {
    fn from(palette: &Palette) -> Self {
        Self {
            frame_color: palette.primary,
            label_color: None,
            gap: 1,
        }
    }
}

fn spinner(cx: &mut Scope, props: &Spinner) -> Element {
    let frame = use_spinner(cx, &props.frames);
    let theme = use_theme::<SpinnerTheme>(cx);
    let mut row = View::new()
        .gap(theme.gap)
        .child(Text::new(frame).color(theme.frame_color));
    if !props.label.is_empty() {
        row = row.child(Text::new(&props.label).color(theme.label_color));
    }
    row.into()
}
