//! Quill demonstrates inline Static scrollback, streaming cells, approval Selects,
//! headless completion lists and a multi-line composer built on use_input_state.
//! Nothing reaches an external service, changes a file, or executes a command.
//!
//! Enter sends; Ctrl-J adds a line; Left/Right and Backspace edit; pasted text
//! keeps its newlines. Up/Down recall submissions. `/` opens commands, `@` opens
//! file mentions, and `?` on an empty composer opens shortcuts. Esc interrupts;
//! Ctrl-C interrupts a turn, clears text, or asks for a second press to quit.
//! Approval keys: y proceeds, a remembers the command or file, n or Esc declines.
//!
//! `test`/`fix` reproduce and repair a test; `explain`/`what`/`how` explore;
//! `refactor`/`plan` update a plan and two files; `fail` breaks the stream;
//! other prompts get a short answer. Matching uses this order, except a lone
//! failure request takes priority. Approvals pause the simulated clock.
//! The resume hint is fictional: there is no persistence or resume command.
//!
//! ```sh
//! cargo run -p quill
//! cargo run -p quill -- -a untrusted "fix the failing test"
//! cargo run -p quill -- "explain the parser"
//! cargo run -p quill -- "plan a refactor"
//! cargo run -p quill -- exec "fix the failing test"
//! cargo run -p quill -- e fail
//! cargo run -p quill -- completions zsh
//! ```

#![forbid(unsafe_code)]

use std::{cell::RefCell, rc::Rc, time::Duration};

use eink_rs::{
    cli::clap::{self, Parser, Subcommand},
    ui::{Select, SelectOption, Spinner},
};
use ink_rs::{AnimationOptions, Element, JustifyContent, Props, Scope, Static, Styled, Text, View};

use agent::{Action, AgentHandle, Answer};
use cells::{Cell, column};
use composer::{Composer, Intent};
use session::Session;

mod agent;
mod cells;
mod composer;
mod markdown;
mod script;
mod session;

pub use agent::ApprovalMode;

/// Arguments for the inline assistant or its non-interactive command.
#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = app)]
pub struct Cli {
    /// First prompt to send in the interactive session.
    prompt: Option<String>,
    /// Model listed by the simulated agent.
    #[arg(short, long, default_value = "quill-1", global = true)]
    model: String,
    /// Commands that require permission.
    #[arg(
        short = 'a',
        long = "ask-for-approval",
        value_enum,
        default_value = "on-request",
        global = true
    )]
    approvals: ApprovalMode,
    /// Run one turn and exit; absence opens the inline composer.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Non-interactive commands; shell completion is supplied by eink-rs.
#[derive(Subcommand, Clone, PartialEq)]
pub enum Command {
    /// Run a simulated turn without approvals and print its cells.
    #[command(alias = "e")]
    Exec {
        /// Task for the simulated agent.
        prompt: String,
    },
}

#[derive(Clone, PartialEq)]
enum Picker {
    Model,
    Approvals,
}

fn app(cx: &mut Scope, props: &Cli) -> Element {
    let agent = cx.use_memo((), || {
        AgentHandle(Rc::new(RefCell::new(script::ScriptedAgent::new())))
    });
    cx.provide(agent);
    Console { cli: props.clone() }.into()
}

#[derive(Clone, PartialEq, Props)]
#[props(component = console)]
struct Console {
    cli: Cli,
}

fn console(cx: &mut Scope, props: &Console) -> Element {
    let agent = cx
        .use_context::<AgentHandle>()
        .expect("quill agent provider");
    let models = agent.0.borrow().models();
    let model = models.iter().find(|model| model.name == props.cli.model);
    let error = model.is_none().then(|| {
        format!(
            "unknown model '{}'; known models: {}",
            props.cli.model,
            models
                .iter()
                .map(|model| model.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        )
    });
    let interactive = props.cli.command.is_none();
    let initial = match &props.cli.command {
        Some(Command::Exec { prompt }) => Some(prompt.clone()),
        None => props.cli.prompt.clone(),
    };
    let session = cx.use_state(|| {
        Session::new(
            props.cli.model.clone(),
            model.map_or("high".into(), |model| model.efforts[0].clone()),
            if interactive {
                props.cli.approvals
            } else {
                ApprovalMode::Never
            },
            interactive,
        )
    });
    let picker = cx.use_state(|| None::<Picker>);
    let shortcuts = cx.use_state(|| false);
    let clock = cx.use_animation(AnimationOptions {
        interval: Duration::from_millis(100),
        active: error.is_none() && !session.get().quit,
    });
    let initializing = agent.clone();
    let valid = error.is_none();
    cx.use_layout_effect((), move || {
        if valid && let Some(prompt) = initial {
            session.update(|state| state.submit(prompt, &initializing, clock.time));
        }
    });
    let ticking = agent.clone();
    cx.use_layout_effect(clock.time, move || {
        session.update(|state| state.poll(&ticking, clock.time));
    });
    let app = cx.use_app();
    let current = session.get();
    let finished = !interactive && !current.busy;
    let invalid_model = error.is_some();
    let failure = error.or(current.failure.clone().filter(|_| !interactive));
    let exit = current.quit || finished || failure.is_some();
    cx.use_effect((exit, failure.clone()), move || {
        if let Some(error) = failure {
            app.exit_with_error(error);
        } else if exit {
            app.exit();
        }
    });
    let responding = agent.clone();
    let answer = cx.use_callback(move |answer: Answer| {
        if session.get().pending.is_some() {
            responding.0.borrow_mut().answer(answer, clock.time);
            session.update(|state| {
                state.pending = None;
                state.poll(&responding, clock.time);
            });
        }
    });
    let answering = answer.clone();
    let interrupting = agent.clone();
    cx.use_input(
        interactive && current.pending.is_some(),
        move |input, key| {
            if key.ctrl && input == "c" {
                interrupting.0.borrow_mut().interrupt();
                session.update(|state| state.poll(&interrupting, clock.time));
            } else if input == "y" || input == "1" {
                answering.call(Answer::Yes);
            } else if input == "a" || input == "2" {
                answering.call(Answer::Always);
            } else if input == "n" || input == "3" || key.escape {
                answering.call(Answer::No);
            }
        },
    );
    cx.use_input(
        interactive && picker.get().is_some() && current.pending.is_none(),
        move |input, key| {
            if key.escape || (key.ctrl && input == "c") {
                picker.set(None);
            }
        },
    );
    let acting = agent.clone();
    let on_intent = cx.use_callback(move |intent| {
        session.update(|state| match intent {
            Intent::Shortcuts(open) => shortcuts.set(open),
            Intent::Interrupt => {
                if state.busy {
                    acting.0.borrow_mut().interrupt();
                    state.poll(&acting, clock.time);
                }
            }
            Intent::ControlC { had_text } => {
                if state.busy {
                    acting.0.borrow_mut().interrupt();
                    state.poll(&acting, clock.time);
                } else if had_text {
                    state.cancel_at = None;
                } else if state
                    .cancel_at
                    .is_some_and(|at| clock.time.saturating_sub(at) < Duration::from_secs(2))
                {
                    state.quit = true;
                } else {
                    state.cancel_at = Some(clock.time);
                }
            }
            Intent::Submit(prompt) => match prompt.trim() {
                "/model" => picker.set(Some(Picker::Model)),
                "/approvals" => picker.set(Some(Picker::Approvals)),
                "/status" => state.history.push(Cell::Note(state.status())),
                "/diff" => {
                    if state.diffs.is_empty() {
                        state
                            .history
                            .push(Cell::Note("No changes in this session.".into()));
                    } else {
                        state
                            .history
                            .extend(state.diffs.iter().cloned().map(Cell::Diff));
                    }
                }
                "/new" => {
                    if state.busy {
                        state.history.push(Cell::Note(
                            "Interrupt or finish the turn before /new.".into(),
                        ));
                    } else {
                        state.number += 1;
                        state.input_tokens = 0;
                        state.output_tokens = 0;
                        state.diffs.clear();
                        state.failure = None;
                        state.cancel_at = None;
                        acting.0.borrow_mut().new_session();
                        state
                            .history
                            .push(Cell::Note(format!("──── New session {} ────", state.id())));
                    }
                }
                "/quit" => {
                    state.queue.clear();
                    if state.busy {
                        acting.0.borrow_mut().interrupt();
                        state.poll(&acting, clock.time);
                    }
                    state.quit = true;
                }
                command if command.starts_with('/') => state
                    .history
                    .push(Cell::Note(format!("Unknown command: {command}"))),
                _ => state.submit(prompt, &acting, clock.time),
            },
        });
    });
    let on_model = cx.use_callback(move |(model, effort)| {
        session.update(|state| {
            state.model = model;
            state.effort = effort;
        });
        picker.set(None);
    });
    let on_mode = cx.use_callback(move |mode| {
        session.update(|state| state.approvals = mode);
        picker.set(None);
    });
    if invalid_model {
        return Element::empty();
    }
    let mut body = column().child(Static::new(current.history.clone(), |cell, _| {
        cells::render(cell)
    }));
    if current.quit {
        return body
            .child(Text::new(current.usage()).dim_color(true))
            .child(Text::new(format!(
                "To continue this session, run quill resume {}",
                current.id()
            )))
            .into();
    }
    if let Some(command) = &current.command {
        body = body.child(cells::render(command));
    }
    if !current.message.is_empty() {
        body = body.child(cells::render(&Cell::Message(current.message.clone())));
    }
    if !interactive {
        return body.into();
    }
    if current.busy {
        body = body.child(Spinner::default().label(format!(
            "Working ({}s • esc to interrupt)",
            clock.time.saturating_sub(current.started).as_secs()
        )));
    }
    body = body.children(current.queue.iter().map(|prompt| {
        Text::new(format!("↳ queued: {}", prompt.replace('\n', " "))).dim_color(true)
    }));
    if let Some(action) = &current.pending {
        let (question, detail, always) = match action {
            Action::Run(command) => (
                "Would you like to run the following command?",
                Text::new(format!("$ {command}")).into(),
                "2. Yes, and don't ask again for this command (a)",
            ),
            Action::Edit(diff) => (
                "Would you like to make the following edits?",
                cells::render(&Cell::Diff(diff.clone())),
                "2. Yes, and don't ask again for this file (a)",
            ),
        };
        body = body.child(
            column()
                .child(Text::new(question).bold(true))
                .child(detail)
                .child(
                    Select::default()
                        .options(vec![
                            SelectOption::new("1. Yes, proceed (y)", Answer::Yes),
                            SelectOption::new(always, Answer::Always),
                            SelectOption::new(
                                "3. No, and tell Quill what to do differently (esc)",
                                Answer::No,
                            ),
                        ])
                        .on_change(Some(answer)),
                )
                .key("approval"),
        );
    } else {
        match picker.get() {
            Some(Picker::Model) => {
                body = body.child(
                    column()
                        .child(Text::new("Choose model and effort · esc to cancel"))
                        .child(
                            Select::default()
                                .options(
                                    models
                                        .iter()
                                        .flat_map(|model| {
                                            model.efforts.iter().map(|effort| {
                                                SelectOption::new(
                                                    format!("{} · {effort}", model.name),
                                                    (model.name.clone(), effort.clone()),
                                                )
                                            })
                                        })
                                        .collect(),
                                )
                                .on_change(Some(on_model)),
                        )
                        .key("models"),
                );
            }
            Some(Picker::Approvals) => {
                body = body.child(
                    column()
                        .child(Text::new(
                            "Choose approvals · applies to later turns · esc to cancel",
                        ))
                        .child(
                            Select::default()
                                .options(
                                    [
                                        ApprovalMode::Untrusted,
                                        ApprovalMode::OnRequest,
                                        ApprovalMode::Never,
                                    ]
                                    .map(|mode| SelectOption::new(mode.label(), mode))
                                    .to_vec(),
                                )
                                .on_change(Some(on_mode)),
                        )
                        .key("modes"),
                );
            }
            None => {}
        }
    }
    body = body.child(
        Element::from(Composer {
            disabled: current.pending.is_some() || picker.get().is_some(),
            busy: current.busy,
            shortcuts: shortcuts.get(),
            files: agent.0.borrow().files(),
            first_prompt: props.cli.prompt.clone(),
            on_intent,
        })
        .key("composer"),
    );
    let hint = if shortcuts.get() && current.pending.is_none() && picker.get().is_none() {
        "? / esc close"
    } else if current
        .cancel_at
        .is_some_and(|at| clock.time.saturating_sub(at) < Duration::from_secs(2))
    {
        "ctrl+c again to quit"
    } else {
        "? for shortcuts"
    };
    body.child(
        View::new()
            .justify_content(JustifyContent::SpaceBetween)
            .child(
                Text::new(format!(
                    "{} · {} · Context {}% used",
                    current.model,
                    current.effort,
                    ((current.input_tokens + current.output_tokens) * 100 / 32000).min(100)
                ))
                .dim_color(true),
            )
            .child(Text::new(hint).dim_color(true)),
    )
    .into()
}
