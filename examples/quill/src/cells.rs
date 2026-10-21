use ink_rs::{BorderStyle, Color, Element, FlexDirection, Styled, Text, View};

use crate::{agent::Diff, markdown};

/// A conversation item shared by scrollback, the live turn and exec.
#[derive(Clone, PartialEq)]
pub(crate) enum Cell {
    Card {
        directory: String,
        model: String,
        effort: String,
        approvals: String,
    },
    User(String),
    Message(String),
    Reasoning(String),
    Exploration(Vec<String>),
    Command {
        name: String,
        output: Vec<String>,
        exit: Option<i32>,
    },
    Diff(Diff),
    Plan(Vec<(String, bool)>),
    Note(String),
    Interrupted,
    Failure(String),
}

/// Creates the vertical layout shared by conversation elements.
pub(crate) fn column() -> View {
    View::new().flex_direction(FlexDirection::Column)
}

/// Renders one cell without state or input handlers.
pub(crate) fn render(cell: &Cell) -> Element {
    let content: Element = match cell {
        Cell::Card {
            directory,
            model,
            effort,
            approvals,
        } => column()
            .border_style(BorderStyle::Round)
            .border_color(Color::Gray)
            .padding_x(1)
            .child(Text::new(">_ Quill (v0.1.0)").bold(true))
            .child(Text::new(directory.as_str()).dim_color(true))
            .child(Text::new(format!(
                "{model} · {effort} · approvals: {approvals}"
            )))
            .child(
                Text::new("Simulated workspace · no commands or writes reach your machine")
                    .dim_color(true),
            )
            .child(Text::new(
                "Describe a task, or try /model, /status, /diff, /new",
            ))
            .into(),
        Cell::User(message) => View::new()
            .background_color(Color::Ansi256(236))
            .child(Text::new("› "))
            .child(Text::new(message.as_str()))
            .into(),
        Cell::Message(message) => View::new()
            .child(Text::new("• "))
            .child(markdown::render(message))
            .into(),
        Cell::Reasoning(summary) => Text::new(summary.as_str())
            .dim_color(true)
            .italic(true)
            .into(),
        Cell::Exploration(lines) => column()
            .child(Text::new("• Explored").bold(true))
            .children(lines.iter().enumerate().map(|(index, line)| {
                Text::new(format!("  {} {line}", if index == 0 { "└" } else { " " }))
            }))
            .into(),
        Cell::Command { name, output, exit } => {
            let mut body = column().child(
                Text::new(format!(
                    "• {} {name}",
                    if exit.is_some() { "Ran" } else { "Running" }
                ))
                .bold(true),
            );
            for (index, line) in output.iter().take(4).enumerate() {
                body = body.child(
                    Text::new(format!("  {} {line}", if index == 0 { "└" } else { " " }))
                        .dim_color(true),
                );
            }
            if output.len() > 4 {
                body = body
                    .child(Text::new(format!("    … +{} lines", output.len() - 4)).dim_color(true));
            }
            if let Some(code) = exit {
                body = body.child(Text::new(format!("    exit {code}")).color(if *code == 0 {
                    Color::Green
                } else {
                    Color::Red
                }));
            }
            body.into()
        }
        Cell::Diff(diff) => {
            let mut body = column().child(
                Text::new(format!(
                    "• Edited {} (+{} -{})",
                    diff.path,
                    diff.added.len(),
                    diff.removed.len()
                ))
                .bold(true),
            );
            for (index, line) in diff.removed.iter().enumerate() {
                body = body.child(
                    Text::new(format!("  {:>3} - {line}", diff.line + index)).color(Color::Red),
                );
            }
            for (index, line) in diff.added.iter().enumerate() {
                body = body.child(
                    Text::new(format!("  {:>3} + {line}", diff.line + index)).color(Color::Green),
                );
            }
            body.into()
        }
        Cell::Plan(steps) => column()
            .child(Text::new("• Updated Plan").bold(true))
            .children(steps.iter().map(|(step, done)| {
                Text::new(format!("  {} {step}", if *done { "✔" } else { "□" })).dim_color(*done)
            }))
            .into(),
        Cell::Note(note) => Text::new(note.as_str()).dim_color(true).into(),
        Cell::Interrupted => Text::new("■ Conversation interrupted")
            .color(Color::Yellow)
            .into(),
        Cell::Failure(error) => Text::new(format!("■ {error}")).color(Color::Red).into(),
    };
    column().margin_bottom(1).child(content).into()
}
