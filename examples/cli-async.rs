//! A command that shows a spinner while waiting for asynchronous work.
//!
//! ```sh
//! cargo run --example cli-async --features tokio -- --seconds 2
//! ```

use std::process::ExitCode;
use std::time::Duration;

use eink_rs::{
    cli::{
        self,
        clap::{self, Parser},
    },
    ui::Spinner,
};
use ink_rs::{Element, Props, Scope};

#[derive(Parser, Clone, PartialEq, Props)]
#[command(version, about)]
#[props(component = wait)]
struct Cli {
    /// Seconds to wait before exiting
    #[arg(long, default_value_t = 2)]
    seconds: u64,
}

fn wait(cx: &mut Scope, props: &Cli) -> Element {
    let app = cx.use_app();
    let seconds = props.seconds;
    cx.use_future(seconds, move || async move {
        tokio::time::sleep(Duration::from_secs(seconds)).await;
        app.exit();
    });
    Spinner::default().label("Waiting for work").into()
}

#[tokio::main(flavor = "local")]
async fn main() -> ExitCode {
    cli::run_async::<Cli>().await
}
