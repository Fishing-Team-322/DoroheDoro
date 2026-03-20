use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "doro-agent", about = "DoroheDoro Rust log agent")]
pub struct Cli {
    #[arg(
        long,
        env = "AGENT_CONFIG",
        default_value = "/etc/doro-agent/config.yaml"
    )]
    pub config: PathBuf,
}
