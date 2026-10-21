use eink_rs::ui::{
    InputAction, InputState, SelectOption, reduce_input, use_input_state, use_select_state,
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

/// A headless-input composer with history and completion popups.
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

// Derive byte boundaries through the editing kernel so grapheme semantics stay
// with eink-rs, including combining characters and emoji sequences.
fn prefix(state: &InputState) -> String {
    let mut left = InputState::new(&state.value);
    while left.cursor > state.cursor {
        left = reduce_input(&left, InputAction::Delete);
    }
    left.value
}

fn mention(state: &InputState) -> Option<(usize, usize, String)> {
    let before = prefix(state);
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
    let input = use_input_state(cx, "");
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
    cx.use_input(active, move |text, key| {
        if key.event_type == Some(KeyEventType::Release) {
            return;
        }
        let current = input.get();
        if key.ctrl && text == "c" {
            intent.call(Intent::ControlC {
                had_text: !current.value.is_empty(),
            });
            if !busy {
                input.set(InputState::default());
                recalled.set(None);
                intent.call(Intent::Shortcuts(false));
            }
            return;
        }
        if shortcuts {
            if key.escape || text == "?" {
                intent.call(Intent::Shortcuts(false));
            }
            return;
        }
        if text == "?" && current.value.is_empty() {
            intent.call(Intent::Shortcuts(true));
            return;
        }
        if key.escape {
            if busy {
                intent.call(Intent::Interrupt);
            } else if escaped.get() {
                if let Some(last) = history.get().last() {
                    input.set(InputState::new(last));
                }
                escaped.set(false);
            } else {
                escaped.set(true);
            }
            return;
        }
        escaped.set(false);
        if !options.is_empty() && (key.up_arrow || key.down_arrow) {
            selection.set(if key.up_arrow {
                selection.get().focus_previous_option()
            } else {
                selection.get().focus_next_option()
            });
            return;
        }
        if (key.tab || key.enter) && !options.is_empty() {
            let selected = selection
                .get()
                .focused_index()
                .and_then(|index| options.get(index));
            if let Some(selected) = selected {
                if command_popup {
                    input.set(InputState::new(&selected.value));
                    if key.enter {
                        intent.call(Intent::Submit(selected.value.clone()));
                        input.set(InputState::default());
                    }
                } else if let Some((start, end, _)) = &file_word {
                    let mut replacement = current.value[..*start].to_string();
                    replacement.push_str(&selected.value);
                    replacement.push(' ');
                    let cursor = InputState::new(&replacement).cursor;
                    replacement.push_str(&current.value[*end..]);
                    input.set(InputState {
                        value: replacement,
                        cursor,
                    });
                }
            }
            return;
        }
        if key.enter {
            if !current.value.trim().is_empty() {
                history.update(|entries| entries.push(current.value.clone()));
                intent.call(Intent::Submit(current.value));
                input.set(InputState::default());
                recalled.set(None);
            }
            return;
        }
        if (key.up_arrow || key.down_arrow)
            && (current.value.is_empty() || recalled.get().is_some())
        {
            let entries = history.get();
            let index = recalled.get().unwrap_or(entries.len());
            let next = if key.up_arrow {
                index.saturating_sub(1)
            } else {
                (index + 1).min(entries.len())
            };
            input.set(InputState::new(
                entries.get(next).cloned().unwrap_or_default(),
            ));
            recalled.set((next < entries.len()).then_some(next));
            return;
        }
        let action = if (key.ctrl && text == "j") || text == "\n" {
            Some(InputAction::Insert("\n"))
        } else if key.left_arrow {
            Some(InputAction::MoveLeft)
        } else if key.right_arrow {
            Some(InputAction::MoveRight)
        } else if key.backspace || key.delete {
            Some(InputAction::Delete)
        } else if !key.ctrl && !key.meta && !key.tab && !text.is_empty() {
            Some(InputAction::Insert(text))
        } else {
            None
        };
        if let Some(action) = action {
            input.set(reduce_input(&current, action));
            if !key.left_arrow && !key.right_arrow {
                recalled.set(None);
            }
        }
    });
    cx.use_paste(active, move |text| {
        if !shortcuts {
            input.set(reduce_input(&input.get(), InputAction::Insert(text)));
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
    let mut text = Text::default();
    if value.value.is_empty() {
        text = text
            .child(Text::new("A").inverse(true))
            .child(Text::new("sk Quill to do anything").dim_color(true));
    } else {
        let left = prefix(&value);
        let right = prefix(&reduce_input(&value, InputAction::MoveRight));
        let cursor = &value.value[left.len()..right.len()];
        text = text.child(left.as_str()).child(
            Text::new(if cursor.is_empty() || cursor == "\n" {
                " "
            } else {
                cursor
            })
            .inverse(true),
        );
        if cursor == "\n" {
            text = text.child("\n");
        }
        text = text.child(&value.value[right.len()..]);
    }
    // A row keeps continuation lines aligned after the prompt marker.
    body.child(View::new().child(Text::new("› ")).child(text))
        .into()
}
