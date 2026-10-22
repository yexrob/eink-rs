use eink_rs::ui::{
    Composer as Prompt, ComposerAction, ComposerState, Direction, Edit, SelectOption,
    use_select_state,
};
use ink_rs::{Callback, Color, Element, KeyEventType, Props, Scope, Styled, Text, View};

use crate::cells::column;

const COMMANDS: [(&str, &str); 6] = [
    ("/model", "choose model and reasoning effort"),
    ("/approvals", "choose when to ask permission"),
    ("/status", "show session configuration"),
    ("/diff", "show this session's changes"),
    ("/new", "start a new session"),
    ("/quit", "leave Quill"),
];

/// Composer intents handled by the session owner.
#[derive(Clone)]
pub(crate) enum Intent {
    Submit(String),
    Interrupt,
    Shortcuts(bool),
    ControlC { had_text: bool },
}

/// A controlled prompt with history and completion popups.
#[derive(Clone, PartialEq, Props)]
#[props(component = composer)]
pub(crate) struct Composer {
    pub disabled: bool,
    pub busy: bool,
    pub shortcuts: bool,
    pub files: Vec<String>,
    pub first_prompt: Option<String>,
    pub on_intent: Callback<Intent>,
}

fn mention(state: &ComposerState) -> Option<(usize, usize, String)> {
    let before = &state.value[..state.cursor];
    let start = before
        .rfind(char::is_whitespace)
        .map_or(0, |at| at + before[at..].chars().next().unwrap().len_utf8());
    let query = before[start..].strip_prefix('@')?;
    let end = state.value[before.len()..]
        .find(char::is_whitespace)
        .map_or(state.value.len(), |at| before.len() + at);
    Some((start, end, query.into()))
}

fn composer(cx: &mut Scope, props: &Composer) -> Element {
    let input = cx.use_state(ComposerState::default);
    let history = cx.use_state(|| {
        props
            .first_prompt
            .clone()
            .into_iter()
            .collect::<Vec<String>>()
    });
    let recalled = cx.use_state(|| None::<usize>);
    let escaped = cx.use_state(|| false);
    let value = input.get();
    let command_popup = value.value.starts_with('/') && !value.value.contains(char::is_whitespace);
    let file_word = mention(&value);
    let options: Vec<_> = if command_popup {
        COMMANDS
            .iter()
            .filter(|(name, _)| name.starts_with(&value.value))
            .map(|(name, description)| {
                SelectOption::new(format!("{name:<12} {description}"), name.to_string())
            })
            .collect()
    } else if let Some((_, _, query)) = &file_word {
        props
            .files
            .iter()
            .filter(|file| file.contains(query))
            .map(|file| SelectOption::new(file, file.clone()))
            .collect()
    } else {
        vec![]
    };
    let selection = use_select_state(cx, &options, None, 6);
    let intent = props.on_intent.clone();
    let busy = props.busy;
    let shortcuts = props.shortcuts;
    let active = !props.disabled;
    let has_options = !options.is_empty();
    let interrupt = Callback::new(move |()| {
        intent.call(Intent::ControlC {
            had_text: !input.get().value.is_empty(),
        });
        if !busy {
            input.set(ComposerState::default());
            recalled.set(None);
            intent.call(Intent::Shortcuts(false));
        }
    });
    let intent = props.on_intent.clone();
    let escape = Callback::new(move |()| {
        if busy {
            intent.call(Intent::Interrupt);
        } else if escaped.get() {
            if let Some(last) = history.get().last() {
                input.set(ComposerState::new(last));
            }
            escaped.set(false);
        } else {
            escaped.set(true);
        }
    });
    let navigate = Callback::new(move |direction| {
        escaped.set(false);
        if has_options {
            selection.set(if direction == Direction::Up {
                selection.get().focus_previous_option()
            } else {
                selection.get().focus_next_option()
            });
        } else if input.get().value.is_empty() || recalled.get().is_some() {
            let entries = history.get();
            let index = recalled.get().unwrap_or(entries.len());
            let next = if direction == Direction::Up {
                index.saturating_sub(1)
            } else {
                (index + 1).min(entries.len())
            };
            input.set(ComposerState::new(
                entries.get(next).cloned().unwrap_or_default(),
            ));
            recalled.set((next < entries.len()).then_some(next));
        }
    });
    let intent = props.on_intent.clone();
    let complete = Callback::new(move |submit| {
        escaped.set(false);
        let selected = selection
            .get()
            .focused_index()
            .and_then(|index| options.get(index));
        if let Some(selected) = selected {
            if command_popup {
                input.set(ComposerState::new(&selected.value));
                if submit {
                    intent.call(Intent::Submit(selected.value.clone()));
                    input.set(ComposerState::default());
                }
            } else if let Some((start, end, _)) = &file_word {
                let current = input.get();
                let mut replacement = current.value[..*start].to_string();
                replacement.push_str(&selected.value);
                replacement.push(' ');
                let cursor = replacement.len();
                replacement.push_str(&current.value[*end..]);
                input.set(ComposerState {
                    value: replacement,
                    cursor,
                });
            }
        }
    });
    let intent = props.on_intent.clone();
    let tab_complete = complete.clone();
    let shortcuts_interrupt = interrupt.clone();
    cx.use_input(active, move |text, key| {
        if key.event_type == Some(KeyEventType::Release) {
            return;
        }
        if shortcuts {
            if key.ctrl && text == "c" {
                shortcuts_interrupt.call(());
            } else if key.escape || text == "?" {
                intent.call(Intent::Shortcuts(false));
            }
        } else if key.tab && has_options {
            tab_complete.call(false);
        }
    });
    let intent = props.on_intent.clone();
    let edit_navigation = navigate.clone();
    let on_edit = Callback::new(move |edit: Edit| {
        escaped.set(false);
        if (has_options || recalled.get().is_some())
            && matches!(edit.action, ComposerAction::Up | ComposerAction::Down)
        {
            edit_navigation.call(if edit.action == ComposerAction::Up {
                Direction::Up
            } else {
                Direction::Down
            });
        } else if edit.before.value.is_empty() && edit.action == ComposerAction::Insert("?".into())
        {
            intent.call(Intent::Shortcuts(true));
        } else {
            if edit.before.value != edit.after.value {
                recalled.set(None);
            }
            input.set(edit.after);
        }
    });
    let intent = props.on_intent.clone();
    let on_submit = Callback::new(move |value: String| {
        escaped.set(false);
        if has_options {
            complete.call(true);
        } else if !value.trim().is_empty() {
            history.update(|entries| entries.push(value.clone()));
            intent.call(Intent::Submit(value));
            input.set(ComposerState::default());
            recalled.set(None);
        }
    });
    if props.disabled {
        return Element::empty();
    }
    let mut body = column();
    if shortcuts {
        body = body.child(
            View::new()
                .gap(3)
                .child(
                    column()
                        .child(Text::new("Compose").bold(true))
                        .child(Text::new("/ Commands\n@ Mention files\n⌃j New line")),
                )
                .child(
                    column()
                        .child(Text::new("Session").bold(true))
                        .child(Text::new("esc Interrupt\n⌃c Quit\n/model Models")),
                )
                .child(
                    column()
                        .child(Text::new("Transcript").bold(true))
                        .child(Text::new(
                            "↑ / ↓ Recall\nesc esc Edit last message\n/diff Changes",
                        )),
                ),
        );
    } else if has_options {
        let selected = selection.get();
        body = body.child(
            column().children(selected.visible_options().iter().enumerate().map(
                |(index, option)| {
                    let focused =
                        selected.focused_index() == Some(selected.visible_range().start + index);
                    Text::new(format!(
                        "{} {}",
                        if focused { "›" } else { " " },
                        option.label
                    ))
                    .color(if focused { Some(Color::Cyan) } else { None })
                },
            )),
        );
    }
    body.child(
        Prompt::default()
            .value(value.value)
            .cursor(value.cursor)
            .prompt("› ")
            .placeholder("Ask Quill to do anything")
            .max_rows(5)
            .is_disabled(shortcuts)
            .on_edit(Some(on_edit))
            .on_submit(Some(on_submit))
            .on_history(Some(navigate))
            .on_escape(Some(escape))
            .on_interrupt(Some(interrupt)),
    )
    .into()
}
