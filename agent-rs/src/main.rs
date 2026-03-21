mod app;
mod batching;
mod cli;
mod config;
mod doctor;
mod error;
mod logging;
mod metadata;
mod policy;
mod proto;
mod runtime;
mod sources;
mod state;
mod transport;

use clap::Parser;

use crate::{
    app::App,
    cli::{Cli, CliCommand},
    error::AppResult,
};

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();
    match cli.command_or_run() {
        CliCommand::Run => {
            let app = App::load(cli.config.clone()).await?;
            app.run().await
        }
        CliCommand::Doctor => {
            let report = doctor::run(&cli.config)?;
            report.print();
            if report.has_failures() {
                std::process::exit(2);
            }
            Ok(())
        }
    }
}
