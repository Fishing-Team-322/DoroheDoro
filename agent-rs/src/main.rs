mod app;
mod batching;
mod cli;
mod config;
mod error;
mod logging;
mod proto;
mod runtime;
mod sources;
mod state;
mod transport;

use clap::Parser;

use crate::{app::App, cli::Cli, error::AppResult};

#[tokio::main]
async fn main() -> AppResult<()> {
    let cli = Cli::parse();
    let app = App::load(cli).await?;
    app.run().await
}
