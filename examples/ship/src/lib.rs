//! Ship is a simulated deploy console: Ink UI components and palettes, command
//! components, and file routes with shared layouts and typed parameters.
//!
//! Routes: `/`, `/services`, `/services/:service`, and its `logs`, `scale`,
//! `rollback` tabs; `/deploy`, `/deploy/:service`, and its `regions`, `review`,
//! `run` steps; `/settings` and `/settings/account`.
//!
//! Esc returns or exits at the history root; Ctrl-C exits. Lists use Up/Down
//! and Enter. Service tabs use Tab/Shift-Tab, Ctrl-P pins their header, d opens deploy.
//! Logs use Space to pause. Regions use Space to toggle and Enter to continue.
//! Settings uses a to open account; account uses Tab to move and Enter to save.
//! Scale and review use y/n to confirm. Theme changes affect the whole console.
//!
//! The simulator is the only backend and never contacts a service. Its clock
//! advances in one-second steps from seed 0x5eed. Deploying worker v2.0.0 fails
//! at the health check; other releases finish. Data lasts for this process.
//!
//! ```sh
//! cargo run -p ship
//! cargo run -p ship -- services api logs
//! cargo run -p ship -- deploy api
//! cargo run -p ship -- deploy worker
//! cargo run -p ship -- settings account
//! cargo run -p ship -- status
//! cargo run -p ship -- status api
//! cargo run -p ship -- logs api -n 5
//! cargo run -p ship -- logs api -f
//! cargo run -p ship -- completions zsh
//! ```

#![forbid(unsafe_code)]

use std::{cell::RefCell, rc::Rc, time::Duration};

use eink_rs::{
    cli::{
        Commands,
        clap::{self, Parser, Subcommand},
    },
    router::{RouteArgs, Router},
};
use ink_rs::{AnimationOptions, Element, Props, Scope};

mod backend;
mod commands;
mod sim;
mod view;

/// The pages and layouts discovered from the route directory.
pub mod routes {
    eink_rs::router::routes!("src/routes");
}

/// Deploy console arguments, sharing route words with print commands.
#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = app)]
pub struct Cli {
    #[command(flatten)]
    route: RouteArgs<routes::Route>,
    /// A normal-screen command; absence opens the alternate-screen console.
    #[command(subcommand)]
    pub command: Option<Command>,
}

/// Commands whose argument types are their component props.
#[derive(Subcommand, Clone, PartialEq, Commands)]
pub enum Command {
    /// Print service status and exit.
    Status(commands::Status),
    /// Print service logs, or follow until q, Esc or Ctrl-C.
    Logs(commands::Logs),
}

fn app(cx: &mut Scope, props: &Cli) -> Element {
    let backend = cx.use_memo((), || {
        backend::BackendHandle(Rc::new(RefCell::new(sim::Simulator::new())))
    });
    let live = props
        .command
        .as_ref()
        .is_none_or(|command| matches!(command, Command::Logs(logs) if logs.follow));
    let clock = cx.use_animation(AnimationOptions {
        interval: Duration::from_millis(250),
        active: live,
    });
    let revision = cx.use_state(|| Duration::ZERO);
    let ticking = backend.clone();
    cx.use_layout_effect(clock.time, move || {
        ticking.0.borrow_mut().refresh(clock.time);
        revision.set(clock.time);
    });
    cx.provide(backend);
    cx.provide(backend::Revision(revision.get()));
    match props.command.clone() {
        Some(command) => command.into(),
        None => Router::new(props.route.route.clone()).into(),
    }
}
