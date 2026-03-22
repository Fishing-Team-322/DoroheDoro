use std::path::PathBuf;

use clap::{Args, Parser};

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
    #[command(visible_alias = "check-config")]
    Doctor(OutputFormatArgs),
    Preflight(OutputFormatArgs),
    Health(OutputFormatArgs),
}

#[derive(Debug, Clone, Args, Default)]
pub struct OutputFormatArgs {
    #[arg(long)]
    pub json: bool,
}
