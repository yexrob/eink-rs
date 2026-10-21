// Port of Ink UI 2.0.0 test/text-input.tsx.
mod support;
use eink_rs::ui::TextInput;
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
    let app = render(TextInput::default().placeholder("Start typing..."));
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
}

#[test]
fn cursor_cant_be_moved_in_placeholder() {
    let mut app = render(TextInput::default().placeholder("Start typing..."));
    app.input(LEFT);
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
}

#[test]
fn hide_cursor_in_placeholder_when_disabled() {
    let app = render(
        TextInput::default()
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
        TextInput::default()
            .default_value("Test")
            .placeholder("Start typing..."),
    );
    assert_eq!(app.last_frame(), support::frame(end("Test")));
}

#[test]
fn hide_cursor_in_value_when_disabled() {
    let app = render(
        TextInput::default()
            .default_value("Test")
            .placeholder("Start typing...")
            .is_disabled(true),
    );
    assert_eq!(app.last_frame(), support::frame(Text::new("Test")));
}

#[test]
fn insert_character() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .placeholder("Start typing...")
            .on_change(Some(on_change)),
    );
    app.input("T");
    assert_eq!(app.last_frame(), support::frame(end("T")));
    assert_eq!(&*changes.borrow(), &["T"]);
}

#[test]
fn insert_text() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .placeholder("Start typing...")
            .on_change(Some(on_change)),
    );
    app.input("Test");
    assert_eq!(app.last_frame(), support::frame(end("Test")));
    assert_eq!(&*changes.borrow(), &["Test"]);
}

#[test]
fn ignore_input_when_disabled() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .placeholder("Start typing...")
            .is_disabled(true)
            .on_change(Some(on_change)),
    );
    app.input("T");
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
        TextInput::default()
            .default_value("Test")
            .on_change(Some(on_change)),
    );
    app.input(DELETE);
    assert_eq!(app.last_frame(), support::frame(end("Tes")));
    assert_eq!(&*changes.borrow(), &["Tes"]);
}

#[test]
fn delete_all_text() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .placeholder("Start typing...")
            .default_value("Test")
            .on_change(Some(on_change)),
    );
    for _ in 0..4 {
        app.input(DELETE);
    }
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
    assert_eq!(&*changes.borrow(), &["Tes", "Te", "T", ""]);
}

#[test]
fn move_cursor_to_the_left() {
    let mut app = render(TextInput::default().default_value("Test"));
    for at in (0..4).rev() {
        app.input(LEFT);
        assert_eq!(app.last_frame(), support::frame(cursor_at("Test", at)));
    }
}

#[test]
fn cursor_cant_be_moved_left_when_at_the_beginning_of_the_input() {
    let mut app = render(TextInput::default().default_value("Test"));
    for _ in 0..5 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("Test", 0)));
}

#[test]
fn move_cursor_to_the_right() {
    let mut app = render(TextInput::default().default_value("Test"));
    for _ in 0..4 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("Test", 0)));
    for at in 1..4 {
        app.input(RIGHT);
        assert_eq!(app.last_frame(), support::frame(cursor_at("Test", at)));
    }
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(end("Test")));
}

#[test]
fn cursor_cant_be_moved_right_when_at_the_end_of_the_input() {
    let mut app = render(TextInput::default().default_value("Test"));
    assert_eq!(app.last_frame(), support::frame(end("Test")));
    app.input(RIGHT);
    assert_eq!(app.last_frame(), support::frame(end("Test")));
}

#[test]
fn insert_character_in_the_middle() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .default_value("Hllo")
            .on_change(Some(on_change)),
    );
    assert_eq!(app.last_frame(), support::frame(end("Hllo")));
    for _ in 0..3 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("Hllo", 1)));
    app.input("e");
    assert_eq!(app.last_frame(), support::frame(cursor_at("Hello", 2)));
    assert_eq!(&*changes.borrow(), &["Hello"]);
}

#[test]
fn insert_text_in_the_middle() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .default_value("Hllo")
            .on_change(Some(on_change)),
    );
    assert_eq!(app.last_frame(), support::frame(end("Hllo")));
    for _ in 0..3 {
        app.input(LEFT);
    }
    assert_eq!(app.last_frame(), support::frame(cursor_at("Hllo", 1)));
    app.input("eee");
    assert_eq!(app.last_frame(), support::frame(cursor_at("Heeello", 4)));
    assert_eq!(&*changes.borrow(), &["Heeello"]);
}

#[test]
fn autocomplete() {
    let (changes, on_change) = recorder();
    let (submissions, on_submit) = recorder();
    let mut app = render(
        TextInput::default()
            .placeholder("Start typing...")
            .on_change(Some(on_change))
            .on_submit(Some(on_submit))
            .suggestions(vec!["Abby".into(), "Angel".into(), "Annie".into()]),
    );
    assert_eq!(app.last_frame(), support::frame(placeholder_frame()));
    app.input("A");
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("A")
                .child(Text::new("b").inverse(true))
                .child(Text::new("by").dim_color(true))
        )
    );
    assert_eq!(&*changes.borrow(), &["A"]);
    app.input("b");
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("Ab")
                .child(Text::new("b").inverse(true))
                .child(Text::new("y").dim_color(true))
        )
    );
    assert_eq!(&*changes.borrow(), &["A", "Ab"]);
    app.input("b");
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("Abb")
                .child(Text::new("y").inverse(true))
                .child(Text::new("").dim_color(true))
        )
    );
    assert_eq!(&*changes.borrow(), &["A", "Ab", "Abb"]);
    app.input(ENTER);
    assert_eq!(app.last_frame(), support::frame(end("Abby")));
    assert_eq!(&*changes.borrow(), &["A", "Ab", "Abb", "Abby"]);
    assert_eq!(&*submissions.borrow(), &["Abby"]);
}

#[test]
fn submit_on_enter() {
    let (submissions, on_submit) = recorder();
    let mut app = render(
        TextInput::default()
            .default_value("Test")
            .on_submit(Some(on_submit)),
    );
    app.input(ENTER);
    assert_eq!(&*submissions.borrow(), &["Test"]);
}

#[test]
fn backspace_at_the_start() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .default_value("Test")
            .on_change(Some(on_change)),
    );
    app.input(LEFT.repeat(4));
    // Ink UI deletes the first character when the cursor is at the start.
    app.input(DELETE);
    assert_eq!(app.last_frame(), support::frame(cursor_at("Test", 0)));
    assert!(changes.borrow().is_empty());
}

#[test]
fn submit_suggestion_with_cursor_in_the_middle() {
    let (changes, on_change) = recorder();
    let (submissions, on_submit) = recorder();
    let mut app = render(
        TextInput::default()
            .default_value("Ab")
            .suggestions(vec!["Abby".into()])
            .on_change(Some(on_change))
            .on_submit(Some(on_submit)),
    );
    app.input(LEFT);
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("A")
                .child(Text::new("b").inverse(true))
                .child(Text::new("by").dim_color(true))
        )
    );
    // Ink UI inserts at the cursor but submits the completion at the end.
    app.input(ENTER);
    assert_eq!(app.last_frame(), support::frame(end("Abby")));
    assert_eq!(&*changes.borrow(), &["Abby"]);
    assert_eq!(&*submissions.borrow(), &["Abby"]);
}

#[test]
fn cursor_moves_over_grapheme_clusters() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .default_value("A\u{1f600}e\u{301}Z")
            .on_change(Some(on_change)),
    );
    // Ink UI mixes UTF-16 offsets with code-point rendering.
    app.input(LEFT.repeat(2));
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("A\u{1f600}")
                .child(Text::new("e\u{301}").inverse(true))
                .child("Z")
        )
    );
    app.input(LEFT);
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("A")
                .child(Text::new("\u{1f600}").inverse(true))
                .child("e\u{301}Z")
        )
    );
    app.input(RIGHT);
    app.input(DELETE);
    assert_eq!(
        app.last_frame(),
        support::frame(
            Text::new("A")
                .child(Text::new("e\u{301}").inverse(true))
                .child("Z")
        )
    );
    app.input(RIGHT);
    app.input(DELETE);
    assert_eq!(app.last_frame(), support::frame(cursor_at("AZ", 1)));
    assert_eq!(&*changes.borrow(), &["Ae\u{301}Z", "AZ"]);
}

#[test]
fn ctrl_letter_does_not_insert() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .default_value("Test")
            .on_change(Some(on_change)),
    );
    // Ink UI inserts the bare letter supplied with Ctrl+letter.
    app.input("\x01");
    assert_eq!(app.last_frame(), support::frame(end("Test")));
    assert!(changes.borrow().is_empty());
}

#[test]
fn paste_uses_the_paste_hook() {
    let (changes, on_change) = recorder();
    let mut app = render(
        TextInput::default()
            .default_value("AB")
            .on_change(Some(on_change)),
    );
    app.input(LEFT);
    // Ink UI has no paste hook; the kernel treats a bracketed paste as one edit.
    app.input("\x1b[200~hello world\x1b[201~");
    assert_eq!(
        app.last_frame(),
        support::frame(cursor_at("Ahello worldB", 12))
    );
    assert_eq!(&*changes.borrow(), &["Ahello worldB"]);

    let (changes, on_change) = recorder();
    let mut email = render(
        eink_rs::ui::EmailInput::default()
            .default_value("test@")
            .domains(Vec::new())
            .on_change(Some(on_change)),
    );
    email.input("\x1b[200~other@example.com\x1b[201~");
    assert_eq!(email.last_frame(), support::frame(end("test@")));
    assert!(changes.borrow().is_empty());
    email.input("\x1b[200~example.com\x1b[201~");
    assert_eq!(email.last_frame(), support::frame(end("test@example.com")));
    assert_eq!(&*changes.borrow(), &["test@example.com"]);
}
