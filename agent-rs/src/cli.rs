use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "doro-agent", about = "DoroheDoro Rust log agent")]
pub struct Cli {
    #[arg(
        long,
        env = "AGENT_CONFIG",
        default_value = "/etc/doro-agent/config.yaml",
        global = true
    )]
    pub config: PathBuf,
    #[command(subcommand)]
    pub command: Option<CliCommand>,
}

#[derive(Debug, Clone, clap::Subcommand)]
pub enum CliCommand {
    Run,
    Doctor,
}

impl Cli {
    pub fn command_or_run(&self) -> CliCommand {
        self.command.clone().unwrap_or(CliCommand::Run)
    }
}
