use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

static SETTINGS_HELP: &str = concat!(
    "Path to the settings file",
    r#"

This is a toml file. For example:
-----------------------------------------
description = "This is a test repository"

[settings]
merge.allow-squash = false
-----------------------------------------
"#
);

#[derive(Debug, StructOpt)]
struct GramOpt {
    /// The owner of the repository
    #[structopt(short, long)]
    owner: String,

    /// The name of the repository
    #[structopt(short, long)]
    repo: String,

    #[structopt(short, long, help = SETTINGS_HELP)]
    settings: PathBuf,
}

#[derive(Debug, Deserialize)]
struct Repository {
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct Settings {
    description: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let opt = GramOpt::from_args();
    let settings = read_settings(&opt.settings)?;
    let repo = get_repo(&opt.owner, &opt.repo).await?;
    diff(repo, settings)?;
    Ok(())
}

fn read_settings(settings_location: &PathBuf) -> Result<Settings, Box<dyn Error>> {
    let settings_str = fs::read_to_string(settings_location)?;
    let settings = toml::from_str::<Settings>(&settings_str)?;
    Ok(settings)
}

async fn get_repo(owner: &str, repo: &str) -> Result<Repository, Box<dyn Error>> {
    let client = reqwest::Client::builder()
        .user_agent("github-repo-automater")
        .build()
        .unwrap();
    let body = client
        .get(&format!("https://api.github.com/repos/{}/{}", owner, repo))
        .header(reqwest::header::CONTENT_TYPE, "application/json")
        .send()
        .await;
    let state = body.unwrap().json::<Repository>().await.unwrap();
    Ok(state)
}

fn diff(repo: Repository, settings: Settings) -> Result<(), Box<dyn Error>> {
    if repo.description != settings.description {
        return Err(format!(
            "Current description {:?} does not match expected description {:?}",
            repo.description.unwrap_or("null".to_owned()),
            settings.description.unwrap_or("null".to_owned())
        ))?;
    }
    Ok(())
}
