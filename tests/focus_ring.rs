mod support;

use std::{cell::RefCell, rc::Rc, time::Duration};

use eink_rs::ui::{
    Button, Claims, FocusRing, RingFocus, RingItem, Select, SelectOption, TextInput, use_ring_focus,
};
use ink_rs::{
    Callback, Color, Element, FlexDirection, Scope, Styled, Text, View, component, testing::render,
};

const TAB: &str = "\t";
const BACK_TAB: &str = "\x1b[Z";
const DOWN: &str = "\x1b[B";
const UP: &str = "\x1b[A";
const ENTER: &str = "\r";

fn record<T: 'static>(calls: &Rc<RefCell<Vec<T>>>) -> Callback<T> {
    let calls = calls.clone();
    Callback::new(move |value| calls.borrow_mut().push(value))
}

fn button(id: &str) -> Element {
    Element::from(Button::default().id(id).label(id)).key(id)
}

fn stops(ids: &[&str]) -> View {
    View::new()
        .flex_direction(FlexDirection::Column)
        .children(ids.iter().map(|id| button(id)))
}

#[test]
fn tab_and_shift_tab_wrap() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        FocusRing::default()
            .on_focus(Some(record(&calls)))
            .child(stops(&["a", "b", "c"])),
    );
    assert!(calls.borrow().is_empty());
    for key in [TAB, TAB, TAB, TAB, BACK_TAB, BACK_TAB, BACK_TAB, BACK_TAB] {
        app.input(key);
    }
    assert_eq!(
        *calls.borrow(),
        ["a", "b", "c", "a", "c", "b", "a", "c"].map(|key| Some(key.into()))
    );
}

#[test]
fn inactive_ring_ignores_keys_and_escape_runs_when_active() {
    let moves = Rc::new(RefCell::new(Vec::new()));
    let presses = Rc::new(RefCell::new(Vec::new()));
    let escapes = Rc::new(RefCell::new(Vec::new()));
    let tree = |active| {
        FocusRing::default()
            .is_active(active)
            .arrows(true)
            .on_focus(Some(record(&moves)))
            .on_escape(Some(record(&escapes)))
            .child(
                Button::default()
                    .id("a")
                    .label("a")
                    .auto_focus(true)
                    .hotkey(Some('y'))
                    .on_press(Some(record(&presses))),
            )
    };
    let mut app = render(tree(false));
    app.input("\t\x1b[B\ry");
    app.input("\x1b");
    app.advance(Duration::from_millis(20));
    assert!(moves.borrow().is_empty());
    assert!(presses.borrow().is_empty());
    assert!(escapes.borrow().is_empty());
    app.rerender(tree(true));
    assert_eq!(*moves.borrow(), vec![Some("a".into())]);
    app.input("\x1b");
    app.advance(Duration::from_millis(20));
    assert_eq!(escapes.borrow().len(), 1);
}

#[test]
fn controlled_focus_waits_for_host_and_allows_redirection() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let tree = |focused| {
        FocusRing::default()
            .focused(focused)
            .on_focus(Some(record(&calls)))
            .child(stops(&["a", "b", "c"]))
    };
    let mut app = render(tree(RingFocus::Key("a".into())));
    let before = app.last_frame().to_owned();
    app.input(DOWN);
    assert!(calls.borrow().is_empty());
    app.input(TAB);
    app.input(TAB);
    assert_eq!(app.last_frame(), before);
    assert_eq!(*calls.borrow(), vec![Some("b".into()), Some("b".into())]);
    app.rerender(tree(RingFocus::Key("c".into())));
    app.input(TAB);
    assert_eq!(calls.borrow().last(), Some(&Some("a".into())));
    app.rerender(tree(RingFocus::None));
    let before = app.last_frame().to_owned();
    app.input(TAB);
    assert_eq!(app.last_frame(), before);
    assert_eq!(calls.borrow().last(), Some(&Some("a".into())));
}

#[test]
fn side_by_side_rings_have_separate_members_and_focus() {
    let left = Rc::new(RefCell::new(Vec::new()));
    let right = Rc::new(RefCell::new(Vec::new()));
    let tree = |active| {
        View::new()
            .child(
                FocusRing::default()
                    .is_active(active)
                    .on_focus(Some(record(&left)))
                    .child(stops(&["a", "b"])),
            )
            .child(
                FocusRing::default()
                    .is_active(!active)
                    .on_focus(Some(record(&right)))
                    .child(stops(&["a", "b"])),
            )
    };
    let mut app = render(tree(true));
    app.input(TAB);
    app.input(TAB);
    assert_eq!(*left.borrow(), vec![Some("a".into()), Some("b".into())]);
    assert!(right.borrow().is_empty());
    app.rerender(tree(false));
    app.input(TAB);
    assert_eq!(*right.borrow(), vec![Some("a".into())]);
    app.rerender(tree(true));
    app.input(BACK_TAB);
    assert_eq!(left.borrow().last(), Some(&Some("a".into())));
}

#[test]
fn insertion_uses_screen_order_and_removal_proposes_neighbours() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let tree = |ids: &[&str]| {
        FocusRing::default()
            .on_focus(Some(record(&calls)))
            .child(stops(ids))
    };
    let mut app = render(tree(&["a", "c"]));
    app.input(TAB);
    app.rerender(tree(&["a", "b", "c"]));
    app.input(TAB);
    assert_eq!(calls.borrow().last(), Some(&Some("b".into())));
    app.rerender(tree(&["a", "c"]));
    assert_eq!(calls.borrow().last(), Some(&Some("c".into())));
    app.rerender(tree(&["a"]));
    assert_eq!(calls.borrow().last(), Some(&Some("a".into())));
    app.rerender(tree(&[]));
    assert_eq!(calls.borrow().last(), Some(&None));
}

#[test]
fn no_auto_focus_does_not_press_and_activation_uses_screen_order() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let presses = Rc::new(RefCell::new(Vec::new()));
    let tree = |active, auto| {
        FocusRing::default()
            .is_active(active)
            .on_focus(Some(record(&calls)))
            .child(
                View::new()
                    .flex_direction(FlexDirection::RowReverse)
                    .child(
                        Button::default()
                            .id("a")
                            .label("a")
                            .auto_focus(auto)
                            .on_press(Some(record(&presses))),
                    )
                    .child(
                        Button::default()
                            .id("b")
                            .label("b")
                            .auto_focus(auto)
                            .on_press(Some(record(&presses))),
                    ),
            )
    };
    let mut app = render(tree(true, false));
    app.input(ENTER);
    assert!(calls.borrow().is_empty());
    assert!(presses.borrow().is_empty());
    app.rerender(tree(false, true));
    app.rerender(tree(true, true));
    assert_eq!(*calls.borrow(), vec![Some("b".into())]);
    app.input(ENTER);
    assert_eq!(presses.borrow().len(), 1);
}

#[test]
fn auto_focus_on_late_mount_proposes_once_when_host_ignores_it() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let tree = |show: bool| {
        FocusRing::default()
            .focused(RingFocus::None)
            .on_focus(Some(record(&calls)))
            .child(show.then(|| Button::default().id("a").label("a").auto_focus(true)))
    };
    let mut app = render(tree(false));
    app.rerender(tree(true));
    app.rerender(tree(true));
    assert_eq!(*calls.borrow(), vec![Some("a".into())]);
}

#[test]
fn arrows_walk_buttons_and_stay_with_select() {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let picks = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        FocusRing::default()
            .arrows(true)
            .on_focus(Some(record(&calls)))
            .child(
                View::new()
                    .flex_direction(FlexDirection::Column)
                    .child(Button::default().id("a").label("a").auto_focus(true))
                    .child(
                        Select::default()
                            .id(Some("s".into()))
                            .options(vec![
                                SelectOption::new("one", 1),
                                SelectOption::new("two", 2),
                            ])
                            .on_select(Some(record(&picks))),
                    )
                    .child(button("b")),
            ),
    );
    app.input(DOWN);
    app.input(DOWN);
    app.input(ENTER);
    assert_eq!(*calls.borrow(), vec![Some("a".into()), Some("s".into())]);
    assert_eq!(*picks.borrow(), vec![2]);
    app.input(UP);
    app.input(ENTER);
    assert_eq!(*picks.borrow(), vec![2, 1]);
    app.input(TAB);
    app.input(DOWN);
    assert_eq!(calls.borrow().last(), Some(&Some("a".into())));
}

#[test]
fn text_focus_blocks_other_hotkeys() {
    let presses = Rc::new(RefCell::new(Vec::new()));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let mut app = render(
        FocusRing::default()
            .arrows(true)
            .child(
                TextInput::default()
                    .id(Some("text".into()))
                    .auto_focus(true)
                    .on_change(Some(record(&changes))),
            )
            .child(
                Button::default()
                    .id("yes")
                    .label("yes")
                    .hotkey(Some('y'))
                    .on_press(Some(record(&presses))),
            ),
    );
    app.input("y");
    assert_eq!(*changes.borrow(), vec!["y"]);
    assert!(presses.borrow().is_empty());
    app.input("\x1b[D");
    app.input("y");
    assert_eq!(changes.borrow().last().unwrap(), "yy");
    app.input(TAB);
    app.input("y");
    assert_eq!(presses.borrow().len(), 1);
}

#[test]
fn custom_stops_and_generated_keys_survive_redraws() {
    fn stop(cx: &mut Scope, label: &String) -> Element {
        let node = cx.use_node_ref();
        let focused = use_ring_focus(
            cx,
            RingItem {
                key: None,
                node,
                auto_focus: false,
                claims: Claims::None,
            },
        );
        View::new()
            .node_ref(node)
            .child(Text::new(label).inverse(focused))
            .into()
    }
    let calls = Rc::new(RefCell::new(Vec::new()));
    let tree = |label: &str| {
        FocusRing::default()
            .on_focus(Some(record(&calls)))
            .child(component(stop, label.to_owned()))
            .child(button("b"))
    };
    let mut app = render(tree("close"));
    app.input(TAB);
    let key = calls.borrow()[0].clone();
    app.rerender(tree("Close"));
    app.input(TAB);
    app.input(TAB);
    assert_eq!(calls.borrow().last(), Some(&key));
}

#[test]
fn bingo_controls_share_ring_and_host_values_redraw() {
    let moves = Rc::new(RefCell::new(Vec::new()));
    let presses = Rc::new(RefCell::new(Vec::new()));
    let changes = Rc::new(RefCell::new(Vec::new()));
    let tree = |text: &str, value| {
        FocusRing::default().on_focus(Some(record(&moves))).child(
            View::new()
                .flex_direction(FlexDirection::Column)
                .child(
                    Button::default()
                        .id("yes")
                        .label("yes")
                        .hotkey(Some('y'))
                        .on_press(Some(record(&presses))),
                )
                .child(button("no"))
                .child(
                    TextInput::default()
                        .id(Some("text".into()))
                        .value(Some(text.into()))
                        .on_change(Some(record(&changes))),
                )
                .child(
                    Select::default()
                        .id(Some("select".into()))
                        .value(Some(value))
                        .options(vec![
                            SelectOption::new("one", 1),
                            SelectOption::new("two", 2),
                        ]),
                ),
        )
    };
    let mut app = render(tree("old", 1));
    app.input("y");
    assert_eq!(presses.borrow().len(), 1);
    for _ in 0..3 {
        app.input(TAB);
    }
    app.input("x");
    assert_eq!(*changes.borrow(), vec!["oldx"]);
    app.input(TAB);
    assert_eq!(
        *moves.borrow(),
        ["yes", "no", "text", "select"].map(|key| Some(key.into()))
    );
    app.rerender(tree("new", 2));
    assert!(app.last_frame().contains("new"));
    // Selection follows the host while the existing focused option stays internal.
    let expected_select = support::frame(
        View::new()
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
            ),
    );
    assert!(app.last_frame().ends_with(&expected_select));
}
