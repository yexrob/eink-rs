// Port of Ink UI 2.0.0 test/confirm-input.tsx.
mod support;
use eink_rs::ui::{ConfirmChoice, ConfirmInput};
use ink_rs::testing::render;
use ink_rs::{Callback, Text};
use std::cell::RefCell;
use std::rc::Rc;
fn recorder() -> (Rc<RefCell<Vec<()>>>, Callback) {
    let calls = Rc::new(RefCell::new(Vec::new()));
    let log = calls.clone();
    (
        calls,
        Callback::new(move |value| log.borrow_mut().push(value)),
    )
}

#[test]
fn confirm_via_uppercase_y() {
    let (confirmed, on_confirm) = recorder();
    let (cancelled, on_cancel) = recorder();
    let mut app = render(ConfirmInput::new(on_confirm, on_cancel));
    assert_eq!(app.last_frame(), support::frame(Text::new("Y/n")));
    app.input("Y");
    assert_eq!(confirmed.borrow().len(), 1);
    assert_eq!(cancelled.borrow().len(), 0);
}

#[test]
fn confirm_via_lowercase_y() {
    let (confirmed, on_confirm) = recorder();
    let (cancelled, on_cancel) = recorder();
    let mut app = render(ConfirmInput::new(on_confirm, on_cancel));
    assert_eq!(app.last_frame(), support::frame(Text::new("Y/n")));
    app.input("y");
    assert_eq!(confirmed.borrow().len(), 1);
    assert_eq!(cancelled.borrow().len(), 0);
}

#[test]
fn confirm_via_enter() {
    let (confirmed, on_confirm) = recorder();
    let (cancelled, on_cancel) = recorder();
    let mut app = render(ConfirmInput::new(on_confirm, on_cancel));
    assert_eq!(app.last_frame(), support::frame(Text::new("Y/n")));
    app.input("\r");
    assert_eq!(confirmed.borrow().len(), 1);
    assert_eq!(cancelled.borrow().len(), 0);
}

#[test]
fn confirm_with_disabled_enter() {
    let (confirmed, on_confirm) = recorder();
    let (cancelled, on_cancel) = recorder();
    let mut app = render(ConfirmInput::new(on_confirm, on_cancel).submit_on_enter(false));
    assert_eq!(app.last_frame(), support::frame(Text::new("Y/n")));
    app.input("\r");
    assert!(confirmed.borrow().is_empty());
    assert!(cancelled.borrow().is_empty());
    app.input("y");
    assert_eq!(confirmed.borrow().len(), 1);
    assert_eq!(cancelled.borrow().len(), 0);
}

#[test]
fn cancel_via_uppercase_n() {
    let (confirmed, on_confirm) = recorder();
    let (cancelled, on_cancel) = recorder();
    let mut app = render(ConfirmInput::new(on_confirm, on_cancel));
    assert_eq!(app.last_frame(), support::frame(Text::new("Y/n")));
    app.input("N");
    assert_eq!(confirmed.borrow().len(), 0);
    assert_eq!(cancelled.borrow().len(), 1);
}

#[test]
fn cancel_via_lowercase_n() {
    let (confirmed, on_confirm) = recorder();
    let (cancelled, on_cancel) = recorder();
    let mut app = render(ConfirmInput::new(on_confirm, on_cancel));
    assert_eq!(app.last_frame(), support::frame(Text::new("Y/n")));
    app.input("n");
    assert_eq!(confirmed.borrow().len(), 0);
    assert_eq!(cancelled.borrow().len(), 1);
}

#[test]
fn cancel_via_enter() {
    let (confirmed, on_confirm) = recorder();
    let (cancelled, on_cancel) = recorder();
    let mut app =
        render(ConfirmInput::new(on_confirm, on_cancel).default_choice(ConfirmChoice::Cancel));
    assert_eq!(app.last_frame(), support::frame(Text::new("y/N")));
    app.input("\r");
    assert_eq!(confirmed.borrow().len(), 0);
    assert_eq!(cancelled.borrow().len(), 1);
}

#[test]
fn cancel_with_disabled_enter() {
    let (confirmed, on_confirm) = recorder();
    let (cancelled, on_cancel) = recorder();
    let mut app = render(
        ConfirmInput::new(on_confirm, on_cancel)
            .default_choice(ConfirmChoice::Cancel)
            .submit_on_enter(false),
    );
    assert_eq!(app.last_frame(), support::frame(Text::new("y/N")));
    app.input("\r");
    assert!(confirmed.borrow().is_empty());
    assert!(cancelled.borrow().is_empty());
    app.input("n");
    assert_eq!(confirmed.borrow().len(), 0);
    assert_eq!(cancelled.borrow().len(), 1);
}
