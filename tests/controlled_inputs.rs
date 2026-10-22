mod support;

use std::{cell::RefCell, rc::Rc};

use eink_rs::ui::{MultiSelect, Select, SelectOption, TextInput};
use ink_rs::{
    Callback, Color, Element, FlexDirection, Scope, Styled, Text, View, component, testing::render,
};

fn record<T: 'static>(calls: &Rc<RefCell<Vec<T>>>) -> Callback<T> {
    let calls = calls.clone();
    Callback::new(move |value| calls.borrow_mut().push(value))
}

fn end(value: &str) -> Text {
    Text::new(value).child(Text::new(" ").inverse(true))
}

#[test]
fn ignored_text_changes_keep_host_value_and_each_batch_starts_from_it() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        TextInput::default()
            .value(Some("AB".into()))
            .default_value("ignored")
            .on_change(Some(record(&calls))),
    );
    app.input("\x1b[D");
    for _ in 0..3 {
        app.input("x");
        assert_eq!(
            app.last_frame(),
            support::frame(Text::new("A").child(Text::new("B").inverse(true)))
        );
    }
    assert_eq!(*calls.borrow(), vec!["AxB", "AxB", "AxB"]);
}

#[test]
fn keys_in_one_batch_build_on_the_previous_proposal() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        TextInput::default()
            .value(Some("A".into()))
            .on_change(Some(record(&calls))),
    );
    // Kitty key sequences create two events instead of a single text run.
    app.input("\x1b[120u\x1b[121u");
    assert_eq!(*calls.borrow(), vec!["Ax", "Axy"]);
    assert_eq!(app.last_frame(), support::frame(end("A")));
}

#[test]
fn host_replacement_moves_cursor_to_end_and_acceptance_preserves_it() {
    fn host(cx: &mut Scope, _: &()) -> Element {
        let value = cx.use_state(|| "AB".to_owned());
        TextInput::default()
            .value(Some(value.get()))
            .on_change(Some(Callback::new(move |next| value.set(next))))
            .into()
    }
    let calls = Rc::new(RefCell::new(Vec::new()));
    let tree = |value: &str| {
        TextInput::default()
            .value(Some(value.into()))
            .on_change(Some(record(&calls)))
    };
    let mut app = render(tree("AB"));
    app.input("\x1b[D");
    app.rerender(tree("replacement"));
    assert_eq!(app.last_frame(), support::frame(end("replacement")));
    app.input("x");
    assert_eq!(calls.borrow().last().unwrap(), "replacementx");
    app.rerender(tree("replacementx"));
    app.input("\x1b[D");
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("replacement").child(Text::new("x").inverse(true)))
    );
    let mut accepting = render(component(host, ()));
    accepting.input("\x1b[D");
    accepting.input("x");
    assert_eq!(
        accepting.last_frame(),
        support::frame(Text::new("Ax").child(Text::new("B").inverse(true)))
    );
}

#[test]
fn select_enter_confirms_current_value_and_ignored_changes_keep_the_mark() {
    let changes = Rc::new(RefCell::new(Vec::new()));
    let selections = Rc::new(RefCell::new(Vec::new()));
    let tree = |value| {
        Select::default()
            .value(Some(value))
            .default_value(Some(1))
            .options(vec![
                SelectOption::new("one", 1),
                SelectOption::new("two", 2),
            ])
            .on_change(Some(record(&changes)))
            .on_select(Some(record(&selections)))
    };
    let mut app = render(tree(2));
    app.input("\r\r");
    assert!(changes.borrow().is_empty());
    assert_eq!(*selections.borrow(), vec![2, 2]);
    app.input("\x1b[A\r");
    assert_eq!(*changes.borrow(), vec![1]);
    assert_eq!(*selections.borrow(), vec![2, 2, 1]);
    let expected = View::new()
        .flex_direction(FlexDirection::Column)
        .child(
            View::new()
                .gap(1)
                .child(Text::new("❯").color(Color::Blue))
                .child(Text::new("one").color(Color::Blue)),
        )
        .child(
            View::new()
                .padding_left(2)
                .gap(1)
                .child(Text::new("two").color(Color::Green))
                .child(Text::new("✔").color(Color::Green)),
        );
    assert_eq!(app.last_frame(), support::frame(expected));
    app.rerender(tree(1));
    app.input("\r");
    assert_eq!(*changes.borrow(), vec![1]);
    assert_eq!(selections.borrow().last(), Some(&1));
}

#[test]
fn number_hotkeys_focus_select_and_scroll_with_theme_colored_prefixes() {
    let selections = Rc::new(RefCell::new(Vec::new()));
    let options = (1..=10)
        .map(|n| SelectOption::new(format!("option {n}"), n))
        .collect::<Vec<_>>();
    let mut app = render(
        Select::default()
            .options(options.clone())
            .number_hotkeys(true)
            .visible_option_count(1)
            .on_select(Some(record(&selections))),
    );
    app.input("9");
    app.input("\r");
    assert_eq!(*selections.borrow(), vec![9, 9]);
    assert_eq!(
        app.last_frame(),
        support::frame(
            View::new()
                .gap(1)
                .child(Text::new("❯").color(Color::Blue))
                .child(Text::new("9.").color(Color::Blue))
                .child(Text::new("option 9").color(Color::Blue))
                .child(Text::new("✔").color(Color::Green))
        )
    );
    app.rerender(
        Select::default()
            .options(options)
            .on_select(Some(record(&selections))),
    );
    app.input("1");
    assert_eq!(*selections.borrow(), vec![9, 9]);
}

#[test]
fn descriptions_render_under_labels_and_windows_count_options() {
    let options = vec![
        SelectOption::new("one", 1).description("first description"),
        SelectOption::new("two", 2).description("second description"),
    ];
    let expected = |label: &str, description: &str| {
        support::frame(
            View::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    View::new()
                        .gap(1)
                        .child(Text::new("❯").color(Color::Blue))
                        .child(Text::new(label).color(Color::Blue)),
                )
                .child(
                    View::new()
                        .padding_left(2)
                        .child(Text::new(description).dim_color(true)),
                ),
        )
    };
    let mut app = render(
        Select::default()
            .options(options.clone())
            .visible_option_count(1),
    );
    assert_eq!(app.last_frame(), expected("one", "first description"));
    app.input("\x1b[B");
    assert_eq!(app.last_frame(), expected("two", "second description"));
    let mut multi = render(
        MultiSelect::default()
            .options(options)
            .visible_option_count(1),
    );
    assert_eq!(multi.last_frame(), expected("one", "first description"));
    multi.input("\x1b[B");
    assert_eq!(multi.last_frame(), expected("two", "second description"));
}
