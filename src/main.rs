use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use structopt::StructOpt;

static GRAM_USER_AGENT: &str = "gram";
static GITHUB_BASE_URL: &str = "https://api.github.com";
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
enum GramOpt {
    /// Diff actual settings with expected settings
    /// defined in a settings.toml
    DiffSettings {
        /// The owner of the repository
        #[structopt(short, long)]
        owner: String,

        /// The name of the repository
        #[structopt(short, long)]
        repo: String,

        #[structopt(short, long, help = SETTINGS_HELP)]
        settings: PathBuf,
    },
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
    match opt {
        GramOpt::DiffSettings {
            owner,
            repo,
            settings,
        } => {
            let settings = read_settings(&settings)?;
            let repo = Github::new().repository(&owner, &repo).await?;
            diff(repo, settings)?;
        }
    }
    Ok(())
}

fn read_settings(settings_location: &PathBuf) -> Result<Settings, Box<dyn Error>> {
    let settings_str = fs::read_to_string(settings_location)?;
    let settings = toml::from_str::<Settings>(&settings_str)?;
    Ok(settings)
}

struct Github {
    client: reqwest::Client,
}

impl Github {
    fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(GRAM_USER_AGENT)
            .build()
            .unwrap();
        Self { client }
    }

    async fn repository(&self, owner: &str, repo: &str) -> Result<Repository, Box<dyn Error>> {
        let repository = self
            .client
            .get(&format!("{}/repos/{}/{}", GITHUB_BASE_URL, owner, repo))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .send()
            .await?
            .json::<Repository>()
            .await?;
        Ok(repository)
    }
}

fn diff(repo: Repository, settings: Settings) -> Result<(), Box<dyn Error>> {
    if repo.description != settings.description {
        return Err(format!(
            "Current description [{}] does not match expected description [{}]",
            repo.description.unwrap_or_else(|| "null".to_owned()),
            settings.description.unwrap_or_else(|| "null".to_owned())
        )
        .into());
    }
    Ok(())
}

#[cfg(test)]
mod test {
    use super::{diff, Repository, Settings};

    #[test]
    fn diff_should_give_error_message_if_there_are_differences() {
        // arrange
        let repo_description = "test";
        let settings_description = "test2";
        let expected = format!(
            "Current description [{}] does not match expected description [{}]",
            repo_description, settings_description
        );
        let repo = Repository {
            description: Some(repo_description.to_owned()),
        };
        let settings = Settings {
            description: Some(settings_description.to_owned()),
        };

        // act
        let output = diff(repo, settings);
        assert!(output.is_err(), "expected an error but did not receive one");
        let actual = format!("{}", output.err().unwrap());
        assert_eq!(
            expected, actual,
            "expected [{}] to match [{}]",
            expected, actual
        );
    }

    #[test]
    fn diff_should_error_if_shows_differences() {
        // arrange
        let repo = Repository {
            description: Some("test".to_owned()),
        };
        let settings = Settings {
            description: Some("test2".to_owned()),
        };

        // act / assert
        assert!(diff(repo, settings).is_err());
    }
}
