mod backend;
mod command_line;
mod components;
mod main_loop;
mod pages;
mod quadra;
mod single_instance;
mod state;
mod utils;

use command_line::Invocation;
use single_instance::Claim;
use std::process::ExitCode;

const APP_ID: &str = "org.quadrahq.Quadra";

fn main() -> ExitCode {
    let invocation = Invocation::of_this_process();
    if let Some(exit_code) = command_line::run_report_if_requested(&invocation) {
        return exit_code;
    }

    if let Claim::Forwarded = single_instance::claim_or_forward(&invocation) {
        return ExitCode::SUCCESS;
    }

    match quadra::run(invocation) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Quadra couldn't start its window: {error}");
            ExitCode::FAILURE
        }
    }
}
