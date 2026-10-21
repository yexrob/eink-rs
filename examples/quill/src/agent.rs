use std::{cell::RefCell, rc::Rc, time::Duration};

use eink_rs::cli::clap::{self, ValueEnum};

/// Which simulated commands and edits need permission.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum ApprovalMode {
    /// Ask before every command and edit.
    Untrusted,
    /// Ask for actions marked as needing escalation.
    #[default]
    OnRequest,
    /// Run without asking.
    Never,
}

impl ApprovalMode {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Untrusted => "untrusted",
            Self::OnRequest => "on-request",
            Self::Never => "never",
        }
    }
}

/// A model and the reasoning efforts it accepts.
#[derive(Clone, PartialEq)]
pub(crate) struct Model {
    pub name: String,
    pub efforts: Vec<String>,
}

/// Settings captured when a turn starts.
#[derive(Clone, PartialEq)]
pub(crate) struct Turn {
    pub prompt: String,
    pub model: String,
    pub effort: String,
    pub approvals: ApprovalMode,
}

/// An approval response, with optional session-wide permission.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Answer {
    Yes,
    Always,
    No,
}

/// A numbered replacement in a simulated source file.
#[derive(Clone, PartialEq)]
pub(crate) struct Diff {
    pub path: String,
    pub line: usize,
    pub removed: Vec<String>,
    pub added: Vec<String>,
}

/// A command or an edit that may wait for the user's approval.
#[derive(Clone, PartialEq)]
pub(crate) enum Action {
    Run(String),
    Edit(Diff),
}

/// Events from one turn, in presentation order.
#[derive(Clone, PartialEq)]
pub(crate) enum Event {
    Reasoning(String),
    MessageDelta(String),
    Exploration(Vec<String>),
    CommandBegin(String),
    CommandOutput(String),
    CommandEnd(i32),
    FileChange(Diff),
    Plan(Vec<(String, bool)>),
    Approval(Action),
    Usage { input: u64, output: u64 },
    Complete,
    Failed(String),
    Interrupted,
}

/// A single-threaded event source driven by the application's elapsed clock.
pub(crate) trait Agent {
    /// Models and efforts available to the session.
    fn models(&self) -> Vec<Model>;
    /// Paths available for mention completion.
    fn files(&self) -> Vec<String>;
    /// Starts a turn, replacing any previous stream.
    fn submit(&mut self, turn: Turn, now: Duration);
    /// Drains events due by `now`, stopping at an unanswered approval.
    fn poll(&mut self, now: Duration) -> Vec<Event>;
    /// Answers the current request and resumes its clock.
    fn answer(&mut self, answer: Answer, now: Duration);
    /// Cancels the current stream; the next poll yields Interrupted.
    fn interrupt(&mut self);
    /// Clears session-scoped permissions.
    fn new_session(&mut self);
}

/// A context handle whose equality compares allocations, not agent contents.
#[derive(Clone)]
pub(crate) struct AgentHandle(pub Rc<RefCell<dyn Agent>>);

impl PartialEq for AgentHandle {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
}
