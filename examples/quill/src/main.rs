use std::process::ExitCode;

use ink_rs::Options;
use quill::Cli;

fn main() -> ExitCode {
    eink_rs::cli::run_with::<Cli>(|cli| Options {
        exit_on_ctrl_c: cli.command.is_some(),
        ..Options::default()
    })
}
