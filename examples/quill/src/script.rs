use std::{collections::VecDeque, time::Duration};

use crate::agent::{Action, Agent, Answer, ApprovalMode, Diff, Event, Model, Turn};

enum Step {
    Event(Event),
    Ask { action: Action, escalation: bool },
}

// "Don't ask again" covers a command line, or every edit to one file.
fn subject(action: &Action) -> &str {
    match action {
        Action::Run(name) => name,
        Action::Edit(diff) => &diff.path,
    }
}

fn start(action: Action) -> Event {
    match action {
        Action::Run(name) => Event::CommandBegin(name),
        Action::Edit(diff) => Event::FileChange(diff),
    }
}

/// A local simulation; no files are written and no commands are executed.
pub(crate) struct ScriptedAgent {
    steps: VecDeque<(Duration, Step)>,
    pending: Option<(Action, Duration)>,
    allowed: Vec<String>,
    mode: ApprovalMode,
    next: Duration,
}

impl ScriptedAgent {
    /// Creates an idle agent with no remembered command permissions.
    pub fn new() -> Self {
        Self {
            steps: VecDeque::new(),
            pending: None,
            allowed: vec![],
            mode: ApprovalMode::OnRequest,
            next: Duration::ZERO,
        }
    }

    fn push(&mut self, step: Step) {
        self.next += Duration::from_millis(180);
        self.steps.push_back((self.next, step));
    }

    fn event(&mut self, event: Event) {
        self.push(Step::Event(event));
    }

    fn message(&mut self, text: &str) {
        for chunk in text.split_inclusive(' ') {
            self.event(Event::MessageDelta(chunk.into()));
        }
    }

    fn command(&mut self, name: &str, escalation: bool, output: &str, exit: i32) {
        self.push(Step::Ask {
            action: Action::Run(name.into()),
            escalation,
        });
        for line in output.lines() {
            self.event(Event::CommandOutput(line.into()));
        }
        self.event(Event::CommandEnd(exit));
    }

    fn plan(&mut self, done: usize) {
        self.event(Event::Plan(
            [
                "Inspect the parser",
                "Extract parsing helpers",
                "Check the crate",
            ]
            .into_iter()
            .enumerate()
            .map(|(index, label)| (label.into(), index < done))
            .collect(),
        ));
    }

    fn edit(&mut self, path: &str, removed: &[&str], added: &[&str], escalation: bool) {
        self.push(Step::Ask {
            action: Action::Edit(Diff {
                path: path.into(),
                line: 12,
                removed: removed.iter().map(|line| (*line).into()).collect(),
                added: added.iter().map(|line| (*line).into()).collect(),
            }),
            escalation,
        });
    }
}

impl Agent for ScriptedAgent {
    fn models(&self) -> Vec<Model> {
        vec![
            Model {
                name: "quill-1".into(),
                efforts: vec!["high".into(), "medium".into(), "low".into()],
            },
            Model {
                name: "quill-mini".into(),
                efforts: vec!["medium".into(), "low".into()],
            },
        ]
    }

    fn files(&self) -> Vec<String> {
        [
            "Cargo.toml",
            "src/lib.rs",
            "src/parse.rs",
            "tests/parse.rs",
            "README.md",
        ]
        .map(str::to_owned)
        .to_vec()
    }

    fn submit(&mut self, turn: Turn, now: Duration) {
        self.steps.clear();
        self.pending = None;
        self.mode = turn.approvals;
        self.next = now;
        let prompt = turn.prompt.to_lowercase();
        if prompt.contains("fail") && !prompt.contains("test") && !prompt.contains("fix") {
            self.event(Event::Reasoning(
                "Checking the simulated event stream.".into(),
            ));
            self.message("I started inspecting the workspace, but");
            self.event(Event::Failed("simulated agent stream failed".into()));
            return;
        }
        if prompt.contains("test") || prompt.contains("fix") {
            self.event(Event::Reasoning(
                "I will reproduce the parser failure and check the boundary case.".into(),
            ));
            self.event(Event::Exploration(vec![
                "Read src/lib.rs".into(),
                "Read src/parse.rs".into(),
                "Search failing in tests/parse.rs".into(),
            ]));
            self.command("cargo test", true, "Compiling tiny-parser v0.1.0\nRunning tests/parse.rs\nrunning 3 tests\ntest parses_number ... ok\ntest trims_input ... FAILED\ntest rejects_empty ... ok\nfailures:\n---- trims_input stdout ----\nthread 'trims_input' panicked at tests/parse.rs:18\nassertion left == right failed\n  left: Err(InvalidDigit)\n right: Ok(42)\nnote: whitespace reached integer parsing\nfailures: trims_input\ntest result: FAILED. 2 passed; 1 failed\nerror: test failed, to rerun pass --test parse", 101);
            self.event(Event::Plan(vec![
                ("Reproduce the failure".into(), true),
                ("Trim input before parsing".into(), false),
                ("Run the tests".into(), false),
            ]));
            self.edit(
                "src/parse.rs",
                &["    input.parse()"],
                &["    let trimmed = input.trim();", "    trimmed.parse()"],
                true,
            );
            self.command(
                "cargo test",
                true,
                "Running tests/parse.rs\nrunning 3 tests\ntest result: ok. 3 passed; 0 failed",
                0,
            );
            self.event(Event::Plan(vec![
                ("Reproduce the failure".into(), true),
                ("Trim input before parsing".into(), true),
                ("Run the tests".into(), true),
            ]));
            self.message("Fixed the parser boundary case. **All 3 tests pass.** Input is trimmed before parsing.");
        } else if ["explain", "what", "how"]
            .iter()
            .any(|word| prompt.contains(word))
        {
            self.event(Event::Exploration(vec![
                "Read src/lib.rs".into(),
                "Read src/parse.rs".into(),
                "Read README.md".into(),
            ]));
            self.message("The crate exposes a **small integer parser**.\n\n- `src/lib.rs` exports the public API.\n- `src/parse.rs` converts text to a number.\n1. Accept a string slice.\n2. Return a value or a parse error.\n\n```rust\nlet value = tiny_parser::parse(\"42\")?;\nassert_eq!(value, 42);\n```\n\nNo files were changed.");
        } else if prompt.contains("refactor") || prompt.contains("plan") {
            self.plan(0);
            self.event(Event::Exploration(vec![
                "Read src/parse.rs".into(),
                "Read src/lib.rs".into(),
            ]));
            self.plan(1);
            self.edit(
                "src/parse.rs",
                &["    input.parse()"],
                &["    parse_number(input.trim())"],
                false,
            );
            self.edit(
                "src/lib.rs",
                &["mod parse;"],
                &["mod parse;", "pub use parse::parse_number;"],
                false,
            );
            self.plan(2);
            self.command(
                "cargo check",
                true,
                "Checking tiny-parser v0.1.0\nFinished dev profile",
                0,
            );
            self.plan(3);
            self.message(
                "Extracted the parsing helper and updated the public export. `cargo check` passes.",
            );
        } else {
            self.event(Event::Reasoning(format!(
                "Using {} with {} effort in the simulated workspace.",
                turn.model, turn.effort
            )));
            self.message("I can help with this Rust crate. Try **fix the failing test**, **explain the parser**, or **plan a refactor**.");
        }
        self.event(Event::Usage {
            input: turn.prompt.len() as u64 / 4 + 120,
            output: 240,
        });
        self.event(Event::Complete);
    }

    fn poll(&mut self, now: Duration) -> Vec<Event> {
        let mut events = vec![];
        if self.pending.is_some() {
            return events;
        }
        while self.steps.front().is_some_and(|(due, _)| *due <= now) {
            let (due, step) = self.steps.pop_front().expect("due step");
            match step {
                Step::Event(event) => events.push(event),
                Step::Ask { action, escalation } => {
                    let ask = match self.mode {
                        ApprovalMode::Untrusted => true,
                        ApprovalMode::OnRequest => escalation,
                        ApprovalMode::Never => false,
                    } && !self
                        .allowed
                        .iter()
                        .any(|allowed| allowed == subject(&action));
                    if ask {
                        self.pending = Some((action.clone(), due));
                        events.push(Event::Approval(action));
                        break;
                    }
                    events.push(start(action));
                }
            }
        }
        events
    }

    fn answer(&mut self, answer: Answer, now: Duration) {
        let Some((action, paused)) = self.pending.take() else {
            return;
        };
        if answer == Answer::No {
            self.steps.clear();
            self.next = now;
            let declined = match &action {
                Action::Run(name) => format!("`{name}` was not run."),
                Action::Edit(diff) => format!("The edit to `{}` was not applied.", diff.path),
            };
            self.message(&format!(
                "{declined} The turn stopped at your request; tell Quill what to do differently."
            ));
            self.event(Event::Complete);
        } else {
            if answer == Answer::Always {
                self.allowed.push(subject(&action).into());
            }
            for (due, _) in &mut self.steps {
                *due += now.saturating_sub(paused);
            }
            self.steps.push_front((now, Step::Event(start(action))));
        }
    }

    fn interrupt(&mut self) {
        self.pending = None;
        self.steps.clear();
        self.steps
            .push_back((Duration::ZERO, Step::Event(Event::Interrupted)));
    }

    fn new_session(&mut self) {
        self.allowed.clear();
    }
}
