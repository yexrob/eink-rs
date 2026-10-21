//! Ported from Ink UI 2.0.0 test/spinner.tsx, with a frame-change regression.

mod support;

use std::time::Duration;

use eink_rs::ui::{Spinner, SpinnerFrames};
use ink_rs::{Color, Text, rsx, testing::render};

#[test]
fn spinner() {
    let mut instance = render(rsx! { Spinner(label: "Loading") });
    instance.advance(Duration::from_millis(800));
    let mut unique_frames = Vec::new();
    for frame in instance.frames() {
        if !unique_frames.contains(frame) {
            unique_frames.push(frame.clone());
        }
    }
    instance.unmount();
    let expected: Vec<_> = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]
        .into_iter()
        .map(|frame| {
            support::frame(Text::new(Text::new(frame).color(Color::Blue)).child(" Loading"))
        })
        .collect();
    assert_eq!(unique_frames, expected);
}

#[test]
fn restart_on_frames_change() {
    let mut instance = render(Spinner::default());
    instance.advance(Duration::from_millis(440));
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new("⠴").color(Color::Blue))
    );

    // Ink UI keeps an out-of-range index when switching to a shorter sequence.
    let frames = SpinnerFrames::new(["a", "b"], Duration::from_millis(80));
    let first_new_frame = instance.frames().len();
    instance.rerender(Spinner::default().frames(frames));
    let expected = support::frame(Text::new("a").color(Color::Blue));
    assert_eq!(instance.last_frame(), expected);
    assert_eq!(&instance.frames()[first_new_frame..], &[expected]);

    // The interval restarts too: the old dots deadline was 40 ms away.
    instance.advance(Duration::from_millis(79));
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new("a").color(Color::Blue))
    );
    instance.advance(Duration::from_millis(1));
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new("b").color(Color::Blue))
    );
    instance.advance(Duration::from_millis(80));
    assert_eq!(
        instance.last_frame(),
        support::frame(Text::new("a").color(Color::Blue))
    );
}
