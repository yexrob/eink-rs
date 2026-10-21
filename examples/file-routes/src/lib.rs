//! In-app routes generated from files, with layouts and typed parameters.
//!
//! Routes: `/`, `/wallet`, `/market`, `/market/:pair`, and `/market/:pair/book`.
//! Keys: m opens ETH, w opens the wallet, 1/2 select chart/book, c increments
//! the pair layout counter, t changes the pair, Esc goes back, and q exits.
//!
//! ```sh
//! cargo run -p file-routes -- --help
//! cargo run -p file-routes
//! cargo run -p file-routes -- market
//! cargo run -p file-routes -- market ETH
//! cargo run -p file-routes -- market ETH book
//! cargo run -p file-routes -- wallet
//! ```

#![forbid(unsafe_code)]

use eink_rs::cli::clap::{self, Parser};
use eink_rs::router::{RouteArgs, Router};
use ink_rs::{Element, Props, Scope};

/// The routes discovered under `src/routes`.
pub mod routes {
    eink_rs::router::routes!("src/routes");
}

/// A market desk with routes generated from files.
#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = app)]
pub struct Cli {
    #[command(flatten)]
    route: RouteArgs<routes::Route>,
}

fn app(_: &mut Scope, props: &Cli) -> Element {
    Router::new(props.route.route.clone()).into()
}
