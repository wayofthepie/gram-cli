mod commands;
pub mod github;
use commands::GramOpt;
use std::error::Error;
use structopt::StructOpt;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    GramOpt::from_args().handle().await
}
