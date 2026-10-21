mod support;

use std::{cell::RefCell, rc::Rc};

use eink_rs::ui::{MultiSelect, SelectOption};
use ink_rs::{Callback, Color, FlexDirection, Styled, Text, View, testing::render};

const DOWN: &str = "\x1b[B";
const UP: &str = "\x1b[A";
const ENTER: &str = "\r";
const LABELS: [&str; 7] = ["Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White"];
const VALUES: [&str; 7] = ["red", "green", "yellow", "blue", "magenta", "cyan", "white"];

fn options() -> Vec<SelectOption<&'static str>> {
    LABELS
        .into_iter()
        .zip(VALUES)
        .map(|(label, value)| SelectOption::new(label, value))
        .collect()
}

fn expected(range: std::ops::Range<usize>, focus: Option<usize>, selected: &[usize]) -> String {
    support::frame(
        View::new()
            .flex_direction(FlexDirection::Column)
            .children(range.map(|index| {
                let focused = focus == Some(index);
                let selected = selected.contains(&index);
                View::new()
                    .gap(1)
                    .padding_left(if focused { 0 } else { 2 })
                    .child(focused.then(|| Text::new("❯").color(Color::Blue)))
                    .child(Text::new(LABELS[index]).color(if focused {
                        Some(Color::Blue)
                    } else if selected {
                        Some(Color::Green)
                    } else {
                        None
                    }))
                    .child(selected.then(|| Text::new("✔").color(Color::Green)))
            })),
    )
}

fn record<T: 'static>(calls: &Rc<RefCell<Vec<T>>>) -> Callback<T> {
    let calls = calls.clone();
    Callback::new(move |value| calls.borrow_mut().push(value))
}

#[test]
fn limit_number_of_visible_options() {
    let app = render(
        MultiSelect::default()
            .options(options())
            .visible_option_count(6),
    );
    assert_eq!(app.last_frame(), expected(0..6, Some(0), &[]));
}

#[test]
fn focus_next_option() {
    let mut app = render(MultiSelect::default().options(options()));
    app.input(DOWN);
    assert_eq!(app.last_frame(), expected(0..5, Some(1), &[]));
}

#[test]
fn focus_next_option_and_scroll_down() {
    let mut app = render(MultiSelect::default().options(options()));
    for _ in 0..6 {
        app.input(DOWN);
    }
    assert_eq!(app.last_frame(), expected(2..7, Some(6), &[]));
}

#[test]
fn dont_scroll_down_when_focused_option_is_the_last_one() {
    let mut app = render(MultiSelect::default().options(options()));
    for _ in 0..7 {
        app.input(DOWN);
    }
    assert_eq!(app.last_frame(), expected(2..7, Some(6), &[]));
}

#[test]
fn focus_previous_option() {
    let mut app = render(MultiSelect::default().options(options()));
    app.input(DOWN);
    assert_eq!(app.last_frame(), expected(0..5, Some(1), &[]));
    app.input(UP);
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[]));
}

#[test]
fn focus_previous_option_and_scroll_up() {
    let mut app = render(MultiSelect::default().options(options()));
    for _ in 0..6 {
        app.input(DOWN);
    }
    assert_eq!(app.last_frame(), expected(2..7, Some(6), &[]));
    for focus in (0..6).rev() {
        app.input(UP);
        // Previous scrolls when focus reaches the first visible row, unlike Next.
        let from = match focus {
            3..=5 => 2,
            2 => 1,
            _ => 0,
        };
        assert_eq!(app.last_frame(), expected(from..from + 5, Some(focus), &[]));
    }
}

#[test]
fn dont_scroll_up_when_focused_option_is_the_first_one() {
    let mut app = render(MultiSelect::default().options(options()));
    for _ in 0..6 {
        app.input(DOWN);
    }
    assert_eq!(app.last_frame(), expected(2..7, Some(6), &[]));
    for _ in 0..7 {
        app.input(UP);
    }
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[]));
}

#[test]
fn ignore_input_when_disabled() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        MultiSelect::default()
            .options(options())
            .is_disabled(true)
            .on_change(Some(record(&calls))),
    );
    assert_eq!(app.last_frame(), expected(0..5, None, &[]));
    assert!(calls.borrow().is_empty());
    for key in [DOWN, UP, ENTER, " "] {
        app.input(key);
        assert_eq!(app.last_frame(), expected(0..5, None, &[]));
        assert!(calls.borrow().is_empty());
    }
}

#[test]
fn toggle_focused_option() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        MultiSelect::default()
            .options(options())
            .on_change(Some(record(&calls))),
    );
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[]));
    assert!(calls.borrow().is_empty());
    app.input(" ");
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[0]));
    assert_eq!(*calls.borrow(), vec![vec!["red"]]);
    app.input(DOWN);
    assert_eq!(app.last_frame(), expected(0..5, Some(1), &[0]));
    app.input(" ");
    assert_eq!(app.last_frame(), expected(0..5, Some(1), &[0, 1]));
    assert_eq!(*calls.borrow(), vec![vec!["red"], vec!["red", "green"]]);
    app.input(" ");
    assert_eq!(app.last_frame(), expected(0..5, Some(1), &[0]));
    assert_eq!(
        *calls.borrow(),
        vec![vec!["red"], vec!["red", "green"], vec!["red"]]
    );
}

#[test]
fn selected_options_by_default() {
    let app = render(
        MultiSelect::default()
            .options(options())
            .default_value(vec!["green", "magenta"]),
    );
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[1, 4]));
}

#[test]
fn submit_selected_options() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        MultiSelect::default()
            .options(options())
            .default_value(vec!["green", "magenta"])
            .on_submit(Some(record(&calls))),
    );
    app.input(ENTER);
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[1, 4]));
    assert_eq!(*calls.borrow(), vec![vec!["green", "magenta"]]);
}

#[test]
fn highlight_text_in_options() {
    let app = render(
        MultiSelect::default()
            .options(options())
            .highlight_text("l"),
    );
    let expected = View::new()
        .flex_direction(FlexDirection::Column)
        .child(
            View::new()
                .gap(1)
                .child(Text::new("❯").color(Color::Blue))
                .child(Text::new("Red").color(Color::Blue)),
        )
        .child(View::new().padding_left(2).child(Text::new("Green")))
        .child(
            View::new().padding_left(2).child(
                Text::new("Ye")
                    .child(Text::new("l").bold(true))
                    .child("low"),
            ),
        )
        .child(
            View::new()
                .padding_left(2)
                .child(Text::new("B").child(Text::new("l").bold(true)).child("ue")),
        )
        .child(View::new().padding_left(2).child(Text::new("Magenta")));
    assert_eq!(app.last_frame(), support::frame(expected));
}
