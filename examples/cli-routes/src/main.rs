use std::process::ExitCode;

use cli_routes::Cli;
use eink_rs::cli;

fn main() -> ExitCode {
    cli::run::<Cli>()
}
