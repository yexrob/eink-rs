mod support;

use std::{cell::RefCell, rc::Rc, time::Duration};

use eink_rs::ui::{
    Caret, Composer, ComposerAction as Action, ComposerState, Direction, Edit, composer_caret,
    reduce_composer,
};
use ink_rs::{
    Callback, ColorLevel, Element, FlexDirection, Options, RenderOptions, Scope, Styled, Text,
    View, component,
    testing::{TestTerminal, mount, render, render_with},
};

fn at(value: &str, cursor: usize) -> ComposerState {
    ComposerState {
        value: value.into(),
        cursor,
    }
}

fn edit(state: &ComposerState, action: Action) -> ComposerState {
    reduce_composer(state, &action, 4, "").state
}

#[test]
fn grapheme_insertion_movement_and_deletion() {
    let family = "\u{1f469}\u{200d}\u{1f4bb}";
    let value = format!("A\u{4e2d}{family}Z");
    let state = ComposerState::new(&value);
    let left = edit(&state, Action::Left);
    assert_eq!(left.cursor, value.len() - 1);
    let left = edit(&left, Action::Left);
    assert_eq!(left.cursor, 4);
    assert_eq!(edit(&left, Action::Right).cursor, value.len() - 1);
    assert_eq!(edit(&left, Action::Delete), at("A\u{4e2d}Z", 4));
    assert_eq!(
        edit(&left, Action::Backspace),
        at(&format!("A{family}Z"), 1)
    );
    assert_eq!(
        edit(&at("eZ", 1), Action::Insert("\u{301}".into())),
        at("e\u{301}Z", 3)
    );
    assert_eq!(edit(&at("ab", 1), Action::Newline), at("a\nb", 2));
}

#[test]
fn words_and_logical_line_boundaries() {
    let state = ComposerState::new("one, two!\nthree");
    assert_eq!(edit(&state, Action::WordLeft).cursor, 10);
    assert_eq!(edit(&at(&state.value, 10), Action::WordLeft).cursor, 5);
    assert_eq!(edit(&at(&state.value, 3), Action::WordRight).cursor, 8);
    assert_eq!(edit(&at(&state.value, 12), Action::LineStart).cursor, 10);
    assert_eq!(edit(&at(&state.value, 5), Action::LineEnd).cursor, 9);
    assert_eq!(
        edit(&ComposerState::new("\u{4e2d}\u{6587}"), Action::WordLeft).cursor,
        3
    );
}

#[test]
fn kills_and_yank() {
    for (state, action, after, kill) in [
        (
            at("one, two!", 8),
            Action::DeleteWord,
            at("one, !", 5),
            "two",
        ),
        (at("a\nbcd", 4), Action::KillToStart, at("a\nd", 2), "bc"),
        (at("abc\ndef", 1), Action::KillToEnd, at("a\ndef", 1), "bc"),
        (at("abc\ndef", 3), Action::KillToEnd, at("abcdef", 3), "\n"),
    ] {
        let reduced = reduce_composer(&state, &action, 4, "old");
        assert_eq!(reduced.state, after);
        assert_eq!(reduced.kill.as_deref(), Some(kill));
        let restored = reduce_composer(&after, &Action::Yank, 4, kill);
        assert_eq!(restored.state.value, state.value);
    }
    assert_eq!(
        reduce_composer(&at("", 0), &Action::KillToStart, 4, "old").kill,
        None
    );
}

#[test]
fn vertical_movement_and_history_boundaries_follow_wrapping() {
    let state = ComposerState::new("ab\u{4e2d}cd\u{1f469}\u{200d}\u{1f4bb}ef");
    assert_eq!(composer_caret(&state, 4), (3, Caret { row: 2, column: 2 }));
    let up = edit(&state, Action::Up);
    assert_eq!(up.cursor, 7);
    assert_eq!(composer_caret(&up, 4).1, Caret { row: 1, column: 2 });
    let up = edit(&up, Action::Up);
    assert_eq!(up.cursor, 2);
    assert_eq!(edit(&up, Action::Down).cursor, 7);
    let boundary = reduce_composer(&up, &Action::Up, 4, "");
    assert_eq!(boundary.state, up);
    assert_eq!(boundary.boundary, Some(Direction::Up));
    let boundary = reduce_composer(&state, &Action::Down, 4, "");
    assert_eq!(boundary.state, state);
    assert_eq!(boundary.boundary, Some(Direction::Down));
}

#[derive(Clone, PartialEq)]
struct Host {
    initial: ComposerState,
    rewrite: bool,
    ignore: bool,
    log: Callback<Edit>,
}

fn host(cx: &mut Scope, props: &Host) -> Element {
    let state = cx.use_state(|| props.initial.clone());
    let current = state.get();
    let rewrite = props.rewrite;
    let ignore = props.ignore;
    let log = props.log.clone();
    Composer::default()
        .value(current.value)
        .cursor(current.cursor)
        .on_edit(Some(Callback::new(move |edit: Edit| {
            log.call(edit.clone());
            if !ignore {
                state.set(if rewrite {
                    ComposerState::new(edit.after.value.to_uppercase())
                } else {
                    edit.after
                });
            }
        })))
        .into()
}

fn recorded_host(rewrite: bool, ignore: bool) -> (Element, Rc<RefCell<Vec<Edit>>>) {
    let log = Rc::new(RefCell::new(Vec::new()));
    let edits = log.clone();
    (
        component(
            host,
            Host {
                initial: ComposerState::default(),
                rewrite,
                ignore,
                log: Callback::new(move |edit| edits.borrow_mut().push(edit)),
            },
        ),
        log,
    )
}

#[test]
fn host_rewrite_wins_on_the_next_render() {
    let (host, log) = recorded_host(true, false);
    let mut app = render(host);
    app.input("a");
    app.input("b");
    assert_eq!(log.borrow()[1].before.value, "A");
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("AB").child(Text::new(" ").inverse(true)))
    );
}

#[test]
fn input_batch_builds_on_each_proposal_in_order() {
    let (host, log) = recorded_host(false, false);
    let mut app = render(host);
    // Kitty events force two keys inside one stdin read rather than one text run.
    app.input("\x1b[97u\x1b[98u");
    let edits = log.borrow();
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].after.value, "a");
    assert_eq!(edits[1].before, edits[0].after);
    assert_eq!(edits[1].after.value, "ab");
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new("ab").child(Text::new(" ").inverse(true)))
    );
}

#[test]
fn ignored_edits_leave_props_and_the_next_batch_unchanged() {
    let (host, log) = recorded_host(false, true);
    let mut app = render(host);
    app.input("a");
    app.input("b");
    assert_eq!(log.borrow()[1].before, ComposerState::default());
    assert_eq!(
        app.last_frame(),
        support::frame(Text::new(" ").inverse(true))
    );
}

#[test]
fn multiline_paste_is_one_normalized_edit() {
    let (host, log) = recorded_host(false, false);
    let mut app = render(host);
    app.input("\x1b[200~a\r\nb\rc\x1b[201~");
    let edits = log.borrow();
    assert_eq!(edits.len(), 1);
    assert_eq!(edits[0].action, Action::Insert("a\nb\nc".into()));
    assert_eq!(edits[0].after, ComposerState::new("a\nb\nc"));
}

#[test]
fn enter_replaces_backslash_and_other_newline_keys_preserve_it() {
    let (host, log) = recorded_host(false, false);
    let mut app = render(host);
    app.input("\\");
    app.input("\r");
    assert_eq!(log.borrow()[1].before.value, "\\");
    assert_eq!(log.borrow()[1].after, ComposerState::new("\n"));
    assert_eq!(log.borrow()[1].action, Action::Newline);
    for key in ["\n", "\x1b\r", "\x1b[13;2u"] {
        app.input("\\");
        app.input(key);
        assert!(log.borrow().last().unwrap().after.value.ends_with("\\\n"));
    }
}

#[test]
fn key_groups_dispatch_edits_history_and_callbacks() {
    let (host, log) = recorded_host(false, false);
    let mut app = render(host);
    app.input("one two\nabc");
    for (key, action) in [
        ("\x1b[D", Action::Left),
        ("\x1b[C", Action::Right),
        ("\x1bb", Action::WordLeft),
        ("\x1bf", Action::WordRight),
        ("\x1b[1;5D", Action::WordLeft),
        ("\x1b[1;5C", Action::WordRight),
        ("\x01", Action::LineStart),
        ("\x05", Action::LineEnd),
        ("\x1b[H", Action::LineStart),
        ("\x1b[F", Action::LineEnd),
        ("\x7f", Action::Backspace),
        ("\x01", Action::LineStart),
        ("\x1b[3~", Action::Delete),
        ("\x05", Action::LineEnd),
        ("\x17", Action::DeleteWord),
        ("\x19", Action::Yank),
        ("\x15", Action::KillToStart),
        ("\x19", Action::Yank),
        ("\x01", Action::LineStart),
        ("\x0b", Action::KillToEnd),
        ("\x19", Action::Yank),
        ("\x1b[A", Action::Up),
        ("\x1b[B", Action::Down),
    ] {
        let before = log.borrow().len();
        app.input(key);
        assert_eq!(log.borrow().len(), before + 1, "{action:?}");
        assert_eq!(log.borrow().last().unwrap().action, action);
    }

    let calls = Rc::new(RefCell::new(Vec::<String>::new()));
    let submit = calls.clone();
    let history = calls.clone();
    let escape = calls.clone();
    let interrupt = calls.clone();
    let terminal = TestTerminal::new(30, 10);
    let mut app = mount(
        Composer::default()
            .value("x")
            .cursor(1)
            .on_submit(Some(Callback::new(move |value| {
                submit.borrow_mut().push(value)
            })))
            .on_history(Some(Callback::new(move |direction| {
                history.borrow_mut().push(format!("{direction:?}"))
            })))
            .on_escape(Some(Callback::new(move |()| {
                escape.borrow_mut().push("escape".into())
            })))
            .on_interrupt(Some(Callback::new(move |()| {
                interrupt.borrow_mut().push("interrupt".into())
            }))),
        Options {
            exit_on_ctrl_c: false,
            max_fps: 0,
            ..Options::default()
        },
        &terminal,
    );
    app.input("\r");
    app.input("\x1b[A\x1b[B");
    app.input("\x1b");
    app.advance(Duration::from_millis(25));
    app.input("\x1b[99;5:1u\x1b[99;5:3u");
    assert_eq!(*calls.borrow(), ["x", "Up", "Down", "escape", "interrupt"]);
}

#[test]
fn growth_scroll_and_caret_use_measured_width() {
    let carets = Rc::new(RefCell::new(Vec::new()));
    let log = carets.clone();
    let callback = Callback::new(move |caret| log.borrow_mut().push(caret));
    let tree = |value: &str, cursor: usize| {
        View::new().width(7).child(
            Composer::default()
                .value(value)
                .cursor(cursor)
                .prompt("> ")
                .max_rows(2)
                .on_caret(Some(callback.clone())),
        )
    };
    let mut app = render_with(
        tree("abc", 3),
        RenderOptions {
            columns: 30,
            color_level: ColorLevel::None,
        },
    );
    assert_eq!(app.last_frame(), "> abc");
    app.rerender(tree("abcdef", 6));
    assert_eq!(app.last_frame(), "> abcd\n  ef");
    app.rerender(tree("abcdefghij", 10));
    assert_eq!(app.last_frame(), "  efgh\n  ij");
    assert_eq!(carets.borrow().last(), Some(&Caret { row: 1, column: 4 }));
    app.rerender(tree("abcdefghij", 0));
    assert_eq!(app.last_frame(), "> abcd\n  efgh");
    assert_eq!(carets.borrow().last(), Some(&Caret { row: 0, column: 2 }));
}

#[test]
fn wide_grapheme_moves_to_next_row_without_splitting() {
    let value = "abc\u{4e2d}";
    let app = render_with(
        Composer::default()
            .value(value)
            .cursor(value.len())
            .prompt("> "),
        RenderOptions {
            columns: 7,
            color_level: ColorLevel::None,
        },
    );
    assert_eq!(app.last_frame(), "> abc\n  \u{4e2d}");
    assert_eq!(
        composer_caret(&ComposerState::new(value), 4),
        (2, Caret { row: 1, column: 2 })
    );
}

#[test]
fn mounted_cursor_follows_growth_above_the_box() {
    fn tree(cx: &mut Scope, _: &()) -> Element {
        let expanded = cx.use_state(|| false);
        cx.use_input(true, move |_, key| {
            if key.tab {
                expanded.set(true);
            }
        });
        View::new()
            .flex_direction(FlexDirection::Column)
            .child(Text::new(if expanded.get() { "a\nb" } else { "a" }))
            .child(Composer::default().value("xy").cursor(1).prompt("> "))
            .child(Text::new("footer"))
            .into()
    }
    let terminal = TestTerminal::new(30, 10);
    let mut app = mount(
        component(tree, ()),
        Options {
            max_fps: 0,
            ..Options::default()
        },
        &terminal,
    );
    // Inline paint ends below the frame, then moves back to the caret row.
    let first = terminal.writes().join("");
    assert!(first.contains("\x1b[4G"), "{first:?}");
    assert!(first.contains("\x1b[2A"), "{first:?}");
    assert!(app.live_frame().contains("\x1b[7my\x1b[27m"));
    let before = terminal.writes().len();
    app.input("\t");
    let update = terminal.writes()[before..].join("");
    assert!(update.contains("\x1b[4G"), "{update:?}");
    assert!(update.contains("\x1b[2A"), "{update:?}");
    assert_eq!(app.live_frame().lines().count(), 4);
    let before = terminal.writes().len();
    app.rerender(Composer::default().value("xy").cursor(1).is_disabled(true));
    assert_eq!(app.live_frame(), "xy");
    assert!(terminal.writes()[before..].join("").contains("\x1b[?25l"));
}
