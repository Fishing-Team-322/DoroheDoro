mod app;
mod batching;
mod cli;
mod config;
#[path = "doctor_v2.rs"]
mod doctor;
mod error;
#[path = "health_v2.rs"]
mod health;
mod logging;
mod metadata;
mod ops;
mod policy;
mod proto;
mod runtime;
mod security;
mod sources;
mod state;
#[cfg(test)]
mod test_support;
mod transport;

use clap::Parser;

use crate::{
    app::App,
    cli::{Cli, CliCommand, OutputFormatArgs},
    error::AppResult,
};

const EXIT_RUNTIME_ERROR: i32 = 1;
const EXIT_PREFLIGHT_FAILURE: i32 = 2;
const EXIT_HEALTH_FAILURE: i32 = 3;

#[tokio::main]
async fn main() {
    install_panic_hook();
    let cli = Cli::parse();
    let exit_code = match dispatch(cli).await {
        Ok(code) => code,
        Err((code, error)) => {
            eprintln!("doro-agent failed: {error}");
            code
        }
    };
    std::process::exit(exit_code);
}

async fn dispatch(cli: Cli) -> Result<i32, (i32, crate::error::AppError)> {
    match cli.command {
        None | Some(CliCommand::Run) => {
            let app = App::load(cli.config.clone())
                .await
                .map_err(|error| (EXIT_RUNTIME_ERROR, error))?;
            app.run()
                .await
                .map(|_| 0)
                .map_err(|error| (EXIT_RUNTIME_ERROR, error))
        }
        Some(CliCommand::Doctor(args)) => {
            let report = doctor::run(&cli.config);
            print_report(&report, &args).map_err(|error| (EXIT_PREFLIGHT_FAILURE, error))?;
            if report.has_failures() {
                return Ok(EXIT_PREFLIGHT_FAILURE);
            }
            Ok(0)
        }
        Some(CliCommand::Preflight(args)) => {
            let report = doctor::run(&cli.config);
            print_report(&report, &args).map_err(|error| (EXIT_PREFLIGHT_FAILURE, error))?;
            if report.has_failures() {
                return Ok(EXIT_PREFLIGHT_FAILURE);
            }
            Ok(0)
        }
        Some(CliCommand::Health(args)) => {
            let report = health::run(&cli.config);
            print_report(&report, &args).map_err(|error| (EXIT_HEALTH_FAILURE, error))?;
            if report.has_failures() {
                return Ok(EXIT_HEALTH_FAILURE);
            }
            Ok(0)
        }
    }
}

fn print_report(report: &crate::ops::OperationalReport, args: &OutputFormatArgs) -> AppResult<()> {
    if args.json {
        report.print_json()?;
    } else {
        report.print_text();
    }
    Ok(())
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("doro-agent panic: {panic_info}");
    }));
}
