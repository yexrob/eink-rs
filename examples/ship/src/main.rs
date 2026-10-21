use std::process::ExitCode;

use ink_rs::Options;
use ship::Cli;

fn main() -> ExitCode {
    eink_rs::cli::run_with::<Cli>(|cli| Options {
        alternate_screen: cli.command.is_none(),
        ..Options::default()
    })
}
