// Port of Ink UI 2.0.0 test/password-input.tsx.
mod support;
use eink_rs::ui::PasswordInput;
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
        .child(Text::new("E").inverse(true))
        .child(Text::new("nter password...").dim_color(true))
}

#[test]
fn placeholder() {
    let app = render(PasswordInput::default().placeholder("Enter password..."));
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
}

#[test]
fn cursor_cant_be_moved_in_placeholder() {
    let mut app = render(PasswordInput::default().placeholder("Enter password..."));
    app.input(LEFT);
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
}

#[test]
fn hide_cursor_in_placeholder_when_disabled() {
    let app = render(
        PasswordInput::default()
            .placeholder("Enter password...")
            .is_disabled(true),
    );
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("Enter password...").dim_color(true))
    );
}

#[test]
fn insert_character() {
    let (changes, on_change) = recorder();
    let mut app = render(
        PasswordInput::default()
            .placeholder("Enter password...")
            .on_change(Some(on_change)),
    );
    app.input("T");
    assert_eq!(app.last_frame(), support::frame(end("*")));
    assert_eq!(&*changes.borrow(), &["T"]);
}

#[test]
fn insert_text() {
    let (changes, on_change) = recorder();
    let mut app = render(
        PasswordInput::default()
            .placeholder("Enter password...")
            .on_change(Some(on_change)),
    );
    app.input("Test");
    assert_eq!(app.last_frame(), support::frame(end("****")));
    assert_eq!(&*changes.borrow(), &["Test"]);
}

#[test]
fn ignore_input_when_disabled() {
    let (changes, on_change) = recorder();
    let mut app = render(
        PasswordInput::default()
            .placeholder("Enter password...")
            .is_disabled(true)
            .on_change(Some(on_change)),
    );
    app.input("T");
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("Enter password...").dim_color(true))
    );
    assert!(changes.borrow().is_empty());
}

#[test]
fn delete_character() {
    let (changes, on_change) = recorder();
    let mut app = render(PasswordInput::default().on_change(Some(on_change)));
    app.input("Test");
    assert_eq!(&*changes.borrow(), &["Test"]);
    app.input(DELETE);
    assert_eq!(app.last_frame(), support::frame(end("***")));
    assert_eq!(&*changes.borrow(), &["Test", "Tes"]);
}

#[test]
fn delete_all_text() {
    let (changes, on_change) = recorder();
    let mut app = render(
        PasswordInput::default()
            .placeholder("Enter password...")
            .on_change(Some(on_change)),
    );
    app.input("Test");
    assert_eq!(&*changes.borrow(), &["Test"]);
    for _ in 0..4 {
        app.input(DELETE);
    }
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
    assert_eq!(&*changes.borrow(), &["Test", "Tes", "Te", "T", ""]);
}

#[test]
fn move_cursor_to_the_left() {
    let mut app = render(PasswordInput::default());
    app.input("Test");
    for at in (0..4).rev() {
        app.input(LEFT);
        assert_eq!(app.last_frame(), support::frame(cursor_at("****", at)));
    }
}

#[test]
fn cursor_cant_be_moved_left_when_at_the_beginning_of_the_input() {
    let mut app = render(PasswordInput::default());
    app.input("Test");
    for _ in 0..5 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("****", 0)));
}

#[test]
fn move_cursor_to_the_right() {
    let mut app = render(PasswordInput::default());
    app.input("Test");
    assert_eq!(app.last_frame(), support::frame(end("****")));
    for _ in 0..4 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("****", 0)));
    for at in 1..4 {
        app.input(RIGHT);
        assert_eq!(app.last_frame(), support::frame(cursor_at("****", at)));
    }
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(end("****")));
}

#[test]
fn cursor_cant_be_moved_right_when_at_the_end_of_the_input() {
    let mut app = render(PasswordInput::default());
    app.input("Test");
    assert_eq!(app.last_frame(), support::frame(end("****")));
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(end("****")));
}

#[test]
fn insert_character_in_the_middle() {
    let (changes, on_change) = recorder();
    let mut app = render(PasswordInput::default().on_change(Some(on_change)));
    app.input("Hllo");
    assert_eq!(app.last_frame(), support::frame(end("****")));
    for _ in 0..3 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("****", 1)));
    app.input("e");
    assert_eq!(app.last_frame(), support::frame(cursor_at("*****", 2)));
    assert_eq!(&*changes.borrow(), &["Hllo", "Hello"]);
}

#[test]
fn insert_text_in_the_middle() {
    let (changes, on_change) = recorder();
    let mut app = render(PasswordInput::default().on_change(Some(on_change)));
    app.input("Hllo");
    assert_eq!(app.last_frame(), support::frame(end("****")));
    for _ in 0..3 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("****", 1)));
    app.input("eee");
    assert_eq!(app.last_frame(), support::frame(cursor_at("*******", 4)));
    assert_eq!(&*changes.borrow(), &["Hllo", "Heeello"]);
}

#[test]
fn submit_on_enter() {
    let (submissions, on_submit) = recorder();
    let mut app = render(PasswordInput::default().on_submit(Some(on_submit)));
    app.input("Test");
    app.input(ENTER);
    assert_eq!(&*submissions.borrow(), &["Test"]);
}
