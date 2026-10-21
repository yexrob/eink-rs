use std::{collections::VecDeque, time::Duration};

use crate::{
    agent::{Action, AgentHandle, ApprovalMode, Diff, Event, Turn},
    cells::Cell,
};

/// Session state independent of terminal layout and key bindings.
#[derive(Clone)]
pub(crate) struct Session {
    pub history: Vec<Cell>,
    pub message: String,
    pub command: Option<Cell>,
    pub pending: Option<Action>,
    pub queue: VecDeque<String>,
    pub busy: bool,
    pub started: Duration,
    pub model: String,
    pub effort: String,
    pub approvals: ApprovalMode,
    pub directory: String,
    pub number: u64,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub diffs: Vec<Diff>,
    pub failure: Option<String>,
    pub quit: bool,
    pub cancel_at: Option<Duration>,
}

impl Session {
    /// Captures the process directory and the first session card.
    pub fn new(model: String, effort: String, approvals: ApprovalMode, interactive: bool) -> Self {
        let directory = std::env::current_dir().map_or_else(
            |_| "<unavailable>".into(),
            |path| path.display().to_string(),
        );
        let history = if interactive {
            vec![Cell::Card {
                directory: directory.clone(),
                model: model.clone(),
                effort: effort.clone(),
                approvals: approvals.label().into(),
            }]
        } else {
            vec![]
        };
        Self {
            history,
            message: String::new(),
            command: None,
            pending: None,
            queue: VecDeque::new(),
            busy: false,
            started: Duration::ZERO,
            model,
            effort,
            approvals,
            directory,
            number: 1,
            input_tokens: 0,
            output_tokens: 0,
            diffs: vec![],
            failure: None,
            quit: false,
            cancel_at: None,
        }
    }

    /// A process-local identifier for the fictional resume hint.
    pub fn id(&self) -> String {
        format!("demo-{:04}", self.number)
    }

    /// Formats the accumulated usage reported by completed turns.
    pub fn usage(&self) -> String {
        format!(
            "Token usage: {} input · {} output · {} total",
            self.input_tokens,
            self.output_tokens,
            self.input_tokens + self.output_tokens
        )
    }

    /// Starts an idle turn or appends a prompt to the FIFO queue.
    pub fn submit(&mut self, prompt: String, agent: &AgentHandle, now: Duration) {
        self.cancel_at = None;
        if self.busy {
            self.queue.push_back(prompt);
            return;
        }
        self.history.push(Cell::User(prompt.clone()));
        self.busy = true;
        self.failure = None;
        self.started = now;
        agent.0.borrow_mut().submit(
            Turn {
                prompt,
                model: self.model.clone(),
                effort: self.effort.clone(),
                approvals: self.approvals,
            },
            now,
        );
    }

    /// Moves finished steps into history and starts the next queued turn.
    pub fn poll(&mut self, agent: &AgentHandle, now: Duration) {
        let events = agent.0.borrow_mut().poll(now);
        for event in events {
            match event {
                Event::Reasoning(text) => self.history.push(Cell::Reasoning(text)),
                Event::MessageDelta(text) => self.message.push_str(&text),
                Event::Exploration(lines) => self.history.push(Cell::Exploration(lines)),
                Event::CommandBegin(name) => {
                    self.command = Some(Cell::Command {
                        name,
                        output: vec![],
                        exit: None,
                    })
                }
                Event::CommandOutput(line) => {
                    if let Some(Cell::Command { output, .. }) = &mut self.command {
                        output.push(line);
                    }
                }
                Event::CommandEnd(code) => {
                    if let Some(Cell::Command { exit, .. }) = &mut self.command {
                        *exit = Some(code);
                    }
                    if let Some(command) = self.command.take() {
                        self.history.push(command);
                    }
                }
                Event::FileChange(diff) => {
                    self.diffs.push(diff.clone());
                    self.history.push(Cell::Diff(diff));
                }
                Event::Plan(steps) => self.history.push(Cell::Plan(steps)),
                Event::Approval(action) => self.pending = Some(action),
                Event::Usage { input, output } => {
                    self.input_tokens += input;
                    self.output_tokens += output;
                }
                Event::Complete | Event::Failed(_) | Event::Interrupted => {
                    if let Some(command) = self.command.take() {
                        self.history.push(command);
                    }
                    if !self.message.is_empty() {
                        self.history
                            .push(Cell::Message(std::mem::take(&mut self.message)));
                    }
                    match event {
                        Event::Failed(error) => {
                            self.failure = Some(error.clone());
                            self.history.push(Cell::Failure(error));
                        }
                        Event::Interrupted => self.history.push(Cell::Interrupted),
                        _ => {}
                    }
                    self.pending = None;
                    self.busy = false;
                }
            }
        }
        if !self.busy
            && !self.quit
            && let Some(prompt) = self.queue.pop_front()
        {
            self.submit(prompt, agent, now);
        }
    }

    /// Formats the configuration snapshot printed by /status.
    pub fn status(&self) -> String {
        format!(
            "Model: {} · {}\nApprovals: {}\nDirectory: {}\nSession: {}\n{}",
            self.model,
            self.effort,
            self.approvals.label(),
            self.directory,
            self.id(),
            self.usage()
        )
    }
}
