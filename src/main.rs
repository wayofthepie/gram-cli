mod commands;
pub mod github;
use commands::{GramOpt, SettingsReader};
use github::Github;
use std::error::Error;
use structopt::StructOpt;

//
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let github = Github::new();
    let reader = SettingsReader::new();
    GramOpt::from_args().handle(github, reader).await
}
