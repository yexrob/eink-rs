use eink_rs::ui::{Composer, ComposerState, Direction, Edit};
use ink_rs::{
    Callback, Element, FlexDirection, Options, Scope, Static, Styled, Text, View, component,
};

fn example(cx: &mut Scope, _: &()) -> Element {
    let input = cx.use_state(ComposerState::default);
    let history = cx.use_state(Vec::<String>::new);
    let recalled = cx.use_state(|| None::<usize>);
    let app = cx.use_app();
    let current = input.get();
    View::new()
        .flex_direction(FlexDirection::Column)
        .child(Static::new(history.get(), |value, _| {
            Text::new(value).into()
        }))
        .child(
            Composer::default()
                .prompt("> ")
                .placeholder("Enter submits; Alt+Enter adds a line; Ctrl+C exits")
                .max_rows(5)
                .value(current.value)
                .cursor(current.cursor)
                .on_edit(Some(Callback::new(move |edit: Edit| {
                    if edit.before.value != edit.after.value {
                        recalled.set(None);
                    }
                    input.set(edit.after);
                })))
                .on_submit(Some(Callback::new(move |value| {
                    history.update(|entries| entries.push(value));
                    input.set(ComposerState::default());
                    recalled.set(None);
                })))
                .on_history(Some(Callback::new(move |direction| {
                    let entries = history.get();
                    let index = recalled.get().unwrap_or(entries.len());
                    let next = match direction {
                        Direction::Up => index.saturating_sub(1),
                        Direction::Down => (index + 1).min(entries.len()),
                    };
                    input.set(ComposerState::new(
                        entries.get(next).cloned().unwrap_or_default(),
                    ));
                    recalled.set((next < entries.len()).then_some(next));
                })))
                .on_interrupt(Some(Callback::new(move |()| app.exit()))),
        )
        .into()
}

fn main() -> Result<(), ink_rs::Error> {
    ink_rs::render(
        component(example, ()),
        Options {
            exit_on_ctrl_c: false,
            ..Options::default()
        },
    )
    .wait_until_exit()
    .map(|_| ())
}
