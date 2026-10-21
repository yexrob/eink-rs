use eink_rs::{router::use_param, ui::use_theme};
use ink_rs::{Element, Props, Scope, Text};

use crate::{
    backend::use_backend,
    view::{self, ConsoleTheme},
};

/// A window-sized live log tail with a frozen snapshot when paused.
#[derive(Clone, Default, PartialEq, Props)]
#[props(component = logs)]
pub struct Logs {}

fn logs(cx: &mut Scope, _: &Logs) -> Element {
    let service = use_param::<super::Service>(cx);
    let backend = use_backend(cx);
    let window = cx.use_window_size();
    let theme = use_theme::<ConsoleTheme>(cx);
    let paused = cx.use_state(|| false);
    let frozen = cx.use_state(Vec::new);
    let count = window.rows.saturating_sub(8).max(1) as usize;
    let tail = backend.0.borrow().logs(&service.0, None, count);
    let snapshot = tail.clone();
    cx.use_input(true, move |input, _| {
        if input == " " {
            if !paused.get() {
                frozen.set(snapshot.clone());
            }
            paused.set(!paused.get());
        }
    });
    let lines = if paused.get() { frozen.get() } else { tail };
    view::column()
        .child(Text::new(if paused.get() { "PAUSED" } else { "Live tail" }))
        .children(
            lines
                .iter()
                .skip(lines.len().saturating_sub(count))
                .map(|line| view::log_line(line, &theme, window.columns as usize)),
        )
        .into()
}
