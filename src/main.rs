mod commands;
pub mod github;
use anyhow::Result;
use commands::GramOpt;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<()> {
    GramOpt::from_args().handle().await
}
