use std::process::ExitCode;

use eink_rs::cli;
use file_routes::Cli;

fn main() -> ExitCode {
    cli::run::<Cli>()
}
