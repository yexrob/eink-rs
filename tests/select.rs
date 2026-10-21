mod support;

use std::{cell::RefCell, rc::Rc};

use eink_rs::ui::{Select, SelectOption};
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
    let app = render(Select::default().options(options()).visible_option_count(6));
    assert_eq!(app.last_frame(), expected(0..6, Some(0), &[]));
}

#[test]
fn focus_next_option() {
    let mut app = render(Select::default().options(options()));
    app.input(DOWN);
    assert_eq!(app.last_frame(), expected(0..5, Some(1), &[]));
}

#[test]
fn focus_next_option_and_scroll_down() {
    let mut app = render(Select::default().options(options()));
    for _ in 0..6 {
        app.input(DOWN);
    }
    assert_eq!(app.last_frame(), expected(2..7, Some(6), &[]));
}

#[test]
fn dont_scroll_down_when_focused_option_is_the_last_one() {
    let mut app = render(Select::default().options(options()));
    for _ in 0..7 {
        app.input(DOWN);
    }
    assert_eq!(app.last_frame(), expected(2..7, Some(6), &[]));
}

#[test]
fn focus_previous_option() {
    let mut app = render(Select::default().options(options()));
    app.input(DOWN);
    assert_eq!(app.last_frame(), expected(0..5, Some(1), &[]));
    app.input(UP);
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[]));
}

#[test]
fn focus_previous_option_and_scroll_up() {
    let mut app = render(Select::default().options(options()));
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
    let mut app = render(Select::default().options(options()));
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
        Select::default()
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
fn select_focused_option() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        Select::default()
            .options(options())
            .on_change(Some(record(&calls))),
    );
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[]));
    assert!(calls.borrow().is_empty());
    app.input(ENTER);
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[0]));
    assert_eq!(*calls.borrow(), vec!["red"]);
    app.input(ENTER);
    assert_eq!(*calls.borrow(), vec!["red"]);
    app.input(DOWN);
    assert_eq!(app.last_frame(), expected(0..5, Some(1), &[0]));
}

#[test]
fn selected_option_by_default() {
    let app = render(
        Select::default()
            .options(options())
            .default_value(Some("green")),
    );
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[1]));
}

#[test]
fn highlight_text_in_options() {
    let app = render(Select::default().options(options()).highlight_text("l"));
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

#[test]
fn on_change_does_not_repeat_for_equal_options_and_new_callback() {
    use eink_rs::ui::MultiSelect;

    let calls = Rc::new(RefCell::new(Vec::new()));
    let props = || {
        Select::default()
            .options(options())
            .on_change(Some(record(&calls)))
    };
    let mut app = render(props());
    app.input(ENTER);
    // Ink UI's effect repeats the callback when callback or options identity changes.
    app.rerender(props());
    assert_eq!(*calls.borrow(), vec!["red"]);
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[0]));
    app.input(format!("{DOWN}{ENTER}"));
    app.input(ENTER);
    assert_eq!(*calls.borrow(), vec!["red", "green"]);

    let calls = Rc::new(RefCell::new(Vec::new()));
    let props = || {
        MultiSelect::default()
            .options(options())
            .on_change(Some(record(&calls)))
    };
    let mut app = render(props());
    app.input(" ");
    app.rerender(props());
    assert_eq!(*calls.borrow(), vec![vec!["red"]]);
    assert_eq!(app.last_frame(), expected(0..5, Some(0), &[0]));
    app.input(format!("{DOWN} {UP}{ENTER}"));
    assert_eq!(*calls.borrow(), vec![vec!["red"], vec!["red", "green"]]);
}

#[test]
fn navigate_across_empty_and_duplicate_values() {
    use eink_rs::ui::MultiSelect;

    let options = || {
        vec![
            SelectOption::new("Red", ""),
            SelectOption::new("Green", "same"),
            SelectOption::new("Yellow", "same"),
            SelectOption::new("Blue", "blue"),
        ]
    };
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        Select::default()
            .options(options())
            .on_change(Some(record(&calls))),
    );
    // Ink UI stops at empty values and uses duplicate values as map and row keys.
    app.input(ENTER);
    assert_eq!(*calls.borrow(), vec![""]);
    for focus in 1..4 {
        app.input(DOWN);
        assert_eq!(app.last_frame(), expected(0..4, Some(focus), &[0]));
    }
    app.input(format!("{UP}{ENTER}"));
    assert_eq!(*calls.borrow(), vec!["", "same"]);
    assert_eq!(app.last_frame(), expected(0..4, Some(2), &[1, 2]));

    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        MultiSelect::default()
            .options(options())
            .on_change(Some(record(&calls))),
    );
    app.input(" ");
    assert_eq!(*calls.borrow(), vec![vec![""]]);
    for focus in 1..4 {
        app.input(DOWN);
        assert_eq!(app.last_frame(), expected(0..4, Some(focus), &[0]));
    }
    app.input(format!("{UP} "));
    assert_eq!(app.last_frame(), expected(0..4, Some(2), &[0, 1, 2]));
    app.input(format!("{UP} "));
    assert_eq!(app.last_frame(), expected(0..4, Some(1), &[0]));
    assert_eq!(*calls.borrow(), vec![vec![""], vec!["", "same"], vec![""]]);
}

#[test]
fn enter_with_no_options_keeps_default_value() {
    use eink_rs::ui::use_select_state;
    use ink_rs::{Element, Scope, component};

    fn probe(cx: &mut Scope, _: &()) -> Element {
        let state = use_select_state(cx, &[], Some("saved"), 5);
        cx.use_input(true, move |_, key| {
            if key.enter {
                state.set(state.get().select_focused_option());
            }
        });
        Text::new(state.get().value.unwrap_or("cleared")).into()
    }

    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        View::new()
            .flex_direction(FlexDirection::Column)
            .child(
                Select::default()
                    .default_value(Some("saved"))
                    .on_change(Some(record(&calls))),
            )
            .child(component(probe, ())),
    );
    // Ink UI assigns undefined to the selection on Enter with no options.
    app.input(ENTER);
    assert_eq!(app.last_frame(), support::frame(Text::new("saved")));
    assert!(calls.borrow().is_empty());
}

#[test]
fn changed_options_reset_before_paint() {
    use eink_rs::ui::MultiSelect;

    // An enum without Default checks that values need neither strings nor Default.
    #[derive(Clone, PartialEq)]
    enum Value {
        Red,
        Green,
        Yellow,
        Blue,
        Magenta,
        Cyan,
        White,
        Missing,
    }
    let options = || {
        LABELS
            .into_iter()
            .zip([
                Value::Red,
                Value::Green,
                Value::Yellow,
                Value::Blue,
                Value::Magenta,
                Value::Cyan,
                Value::White,
            ])
            .map(|(label, value)| SelectOption::new(label, value))
            .collect::<Vec<_>>()
    };
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app =
        render(ink_rs::rsx! { Select(options: options(), on_change: Some(record(&calls))) });
    app.input(format!("{}{ENTER}", DOWN.repeat(6)));
    app.rerender(
        Select::default()
            .options(options())
            .default_value(Some(Value::Green))
            .visible_option_count(2)
            .on_change(Some(record(&calls))),
    );
    assert_eq!(app.last_frame(), expected(2..7, Some(6), &[6]));
    let before = app.frames().len();
    // Ink UI resets during render; a layout effect must expose no stale frame.
    app.rerender(
        Select::default()
            .options(options()[..3].to_vec())
            .default_value(Some(Value::Green))
            .visible_option_count(2)
            .on_change(Some(record(&calls))),
    );
    assert_eq!(&app.frames()[before..], &[expected(0..2, Some(0), &[1])]);
    assert_eq!(calls.borrow().len(), 1);
    app.input(ENTER);
    assert!(calls.borrow().last() == Some(&Value::Red));

    let calls = Rc::new(RefCell::new(Vec::new()));
    let submissions = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        MultiSelect::default()
            .options(options())
            .on_change(Some(record(&calls))),
    );
    app.input(format!("{} ", DOWN.repeat(6)));
    app.rerender(
        MultiSelect::default()
            .options(options())
            .default_value(vec![Value::Green])
            .visible_option_count(2)
            .on_change(Some(record(&calls))),
    );
    assert_eq!(app.last_frame(), expected(2..7, Some(6), &[6]));
    let before = app.frames().len();
    app.rerender(
        MultiSelect::default()
            .options(options()[..3].to_vec())
            .default_value(vec![Value::Green, Value::Missing])
            .visible_option_count(2)
            .on_change(Some(record(&calls)))
            .on_submit(Some(record(&submissions))),
    );
    assert_eq!(&app.frames()[before..], &[expected(0..2, Some(0), &[1])]);
    assert_eq!(calls.borrow().len(), 1);
    app.input(format!(" {DOWN}{ENTER}"));
    assert!(*submissions.borrow() == vec![vec![Value::Green, Value::Missing, Value::Red]]);
}
