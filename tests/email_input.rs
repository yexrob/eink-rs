// Port of Ink UI 2.0.0 test/email-input.tsx.
mod support;
use eink_rs::ui::EmailInput;
use ink_rs::testing::render;
use ink_rs::{Callback, Text};
use std::cell::RefCell;
use std::rc::Rc;
const LEFT: &str = "\x1b[D";
const RIGHT: &str = "\x1b[C";
const DELETE: &str = "\x7f";
const ENTER: &str = "\r";
fn recorder() -> (Rc<RefCell<Vec<String>>>, Callback<String>) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    (
        calls,
        Callback::new(move |value| log.borrow_mut().push(value)),
    )
}
fn cursor_at(value: &str, at: usize) -> Text {
    Text::default().children(
        value
            .chars()
            .enumerate()
            .map(|(index, ch)| Text::new(ch.to_string()).inverse(index == at)),
    )
}
fn end(value: &str) -> Text {
    Text::new(value).child(Text::new(" ").inverse(true))
}
fn placeholder_frame() -> Text {
    Text::default()
        .child(Text::new("S").inverse(true))
        .child(Text::new("tart typing...").dim_color(true))
}

#[test]
fn placeholder() {
    let app = render(EmailInput::default().placeholder("Start typing..."));
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
}

#[test]
fn cursor_cant_be_moved_in_placeholder() {
    let mut app = render(EmailInput::default().placeholder("Start typing..."));
    app.input(LEFT);
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
}

#[test]
fn hide_cursor_in_placeholder_when_disabled() {
    let app = render(
        EmailInput::default()
            .placeholder("Start typing...")
            .is_disabled(true),
    );
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("Start typing...").dim_color(true))
    );
}

#[test]
fn default_value() {
    let app = render(
        EmailInput::default()
            .default_value("test")
            .placeholder("Start typing..."),
    );
    assert_eq!(app.last_frame(), support::frame(end("test")));
}

#[test]
fn hide_cursor_in_value_when_disabled() {
    let app = render(
        EmailInput::default()
            .default_value("test")
            .placeholder("Start typing...")
            .is_disabled(true),
    );
    assert_eq!(app.last_frame(), support::frame(Text::new("test")));
}

#[test]
fn insert_character() {
    let (changes, on_change) = recorder();
    let mut app = render(
        EmailInput::default()
            .placeholder("Start typing...")
            .on_change(Some(on_change)),
    );
    app.input("t");
    assert_eq!(app.last_frame(), support::frame(end("t")));
    assert_eq!(&*changes.borrow(), &["t"]);
}

#[test]
fn insert_text() {
    let (changes, on_change) = recorder();
    let mut app = render(
        EmailInput::default()
            .placeholder("Start typing...")
            .on_change(Some(on_change)),
    );
    app.input("test");
    assert_eq!(app.last_frame(), support::frame(end("test")));
    assert_eq!(&*changes.borrow(), &["test"]);
}

#[test]
fn prevent_two_at_characters() {
    let (changes, on_change) = recorder();
    let mut app = render(
        EmailInput::default()
            .placeholder("Start typing...")
            .on_change(Some(on_change)),
    );
    app.input("test@");
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("test@")
                .child(Text::new("a").inverse(true))
                .child(Text::new("ol.com").dim_color(true))
        )
    );
    assert_eq!(&*changes.borrow(), &["test@"]);
    app.input("@");
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("test@")
                .child(Text::new("a").inverse(true))
                .child(Text::new("ol.com").dim_color(true))
        )
    );
    assert_eq!(&*changes.borrow(), &["test@"]);
}

#[test]
fn ignore_input_when_disabled() {
    let (changes, on_change) = recorder();
    let mut app = render(
        EmailInput::default()
            .placeholder("Start typing...")
            .is_disabled(true)
            .on_change(Some(on_change)),
    );
    app.input("t");
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("Start typing...").dim_color(true))
    );
    assert!(changes.borrow().is_empty());
}

#[test]
fn delete_character() {
    let (changes, on_change) = recorder();
    let mut app = render(
        EmailInput::default()
            .default_value("test")
            .on_change(Some(on_change)),
    );
    app.input(DELETE);
    assert_eq!(app.last_frame(), support::frame(end("tes")));
    assert_eq!(&*changes.borrow(), &["tes"]);
}

#[test]
fn delete_all_text() {
    let (changes, on_change) = recorder();
    let mut app = render(
        EmailInput::default()
            .placeholder("Start typing...")
            .default_value("test")
            .on_change(Some(on_change)),
    );
    for _ in 0..4 {
        app.input(DELETE);
    }
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
    assert_eq!(&*changes.borrow(), &["tes", "te", "t", ""]);
}

#[test]
fn move_cursor_to_the_left() {
    let mut app = render(EmailInput::default().default_value("test"));
    for at in (0..4).rev() {
        app.input(LEFT);
        assert_eq!(app.last_frame(), support::frame(cursor_at("test", at)));
    }
}

#[test]
fn cursor_cant_be_moved_left_when_at_the_beginning_of_the_input() {
    let mut app = render(EmailInput::default().default_value("test"));
    for _ in 0..5 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("test", 0)));
}

#[test]
fn move_cursor_to_the_right() {
    let mut app = render(EmailInput::default().default_value("test"));
    for _ in 0..4 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("test", 0)));
    for at in 1..4 {
        app.input(RIGHT);
        assert_eq!(app.last_frame(), support::frame(cursor_at("test", at)));
    }
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(end("test")));
}

#[test]
fn cursor_cant_be_moved_right_when_at_the_end_of_the_input() {
    let mut app = render(EmailInput::default().default_value("test"));
    assert_eq!(app.last_frame(), support::frame(end("test")));
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(end("test")));
}

#[test]
fn insert_character_in_the_middle() {
    let (changes, on_change) = recorder();
    let mut app = render(
        EmailInput::default()
            .default_value("hllo")
            .on_change(Some(on_change)),
    );
    assert_eq!(app.last_frame(), support::frame(end("hllo")));
    for _ in 0..3 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("hllo", 1)));
    app.input("e");
    assert_eq!(app.last_frame(), support::frame(cursor_at("hello", 2)));
    assert_eq!(&*changes.borrow(), &["hello"]);
}

#[test]
fn insert_text_in_the_middle() {
    let (changes, on_change) = recorder();
    let mut app = render(
        EmailInput::default()
            .default_value("hllo")
            .on_change(Some(on_change)),
    );
    assert_eq!(app.last_frame(), support::frame(end("hllo")));
    for _ in 0..3 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("hllo", 1)));
    app.input("eee");
    assert_eq!(app.last_frame(), support::frame(cursor_at("heeello", 4)));
    assert_eq!(&*changes.borrow(), &["heeello"]);
}

#[test]
fn autocomplete_domain() {
    let (changes, on_change) = recorder();
    let (submissions, on_submit) = recorder();
    let mut app = render(
        EmailInput::default()
            .placeholder("Start typing...")
            .on_change(Some(on_change))
            .on_submit(Some(on_submit)),
    );
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
    app.input("test@");
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("test@")
                .child(Text::new("a").inverse(true))
                .child(Text::new("ol.com").dim_color(true))
        )
    );
    assert_eq!(&*changes.borrow(), &["test@"]);
    app.input("a");
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("test@a")
                .child(Text::new("o").inverse(true))
                .child(Text::new("l.com").dim_color(true))
        )
    );
    assert_eq!(&*changes.borrow(), &["test@", "test@a"]);
    app.input("o");
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("test@ao")
                .child(Text::new("l").inverse(true))
                .child(Text::new(".com").dim_color(true))
        )
    );
    assert_eq!(&*changes.borrow(), &["test@", "test@a", "test@ao"]);
    app.input(ENTER);
    assert_eq!(app.last_frame(), support::frame(end("test@aol.com")));
    assert_eq!(
        &*changes.borrow(),
        &["test@", "test@a", "test@ao", "test@aol.com"]
    );
    assert_eq!(&*submissions.borrow(), &["test@aol.com"]);
}

#[test]
fn submit_on_enter() {
    let (submissions, on_submit) = recorder();
    let mut app = render(
        EmailInput::default()
            .default_value("test")
            .on_submit(Some(on_submit)),
    );
    app.input(ENTER);
    assert_eq!(&*submissions.borrow(), &["test"]);
}
