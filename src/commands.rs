use crate::github::{GithubClient, Repository};
use serde::Deserialize;
use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
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

#[derive(Debug, Deserialize)]
pub struct Settings {
    description: Option<String>,
}

/// Commands supported by `gram`.
#[derive(Debug, StructOpt)]
pub enum GramOpt {
    /// Diff actual settings with expected settings
    /// defined in a settings toml file.
    ///
    /// gram will only diff settings defined in the given toml file. It
    /// will not mention any settings which are not defined in that file.
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

impl GramOpt {
    pub async fn handle<G, F>(self, github: G, reader: F) -> Result<(), Box<dyn Error>>
    where
        G: GithubClient,
        F: FileReader,
    {
        match self {
            GramOpt::DiffSettings {
                owner,
                repo,
                settings,
            } => {
                let settings = reader.read_settings(&settings)?;
                let repo = github.repository(&owner, &repo).await?;
                diff(repo, settings)?;
            }
        }
        Ok(())
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

pub trait FileReader {
    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String, std::io::Error>;

    fn read_settings(&self, settings_location: &PathBuf) -> Result<Settings, Box<dyn Error>> {
        let settings_str = self.read_to_string(settings_location)?;
        let settings = toml::from_str::<Settings>(&settings_str)?;
        Ok(settings)
    }
}

pub struct SettingsReader;

impl SettingsReader {
    pub fn new() -> Self {
        Self {}
    }
}

impl FileReader for SettingsReader {
    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String, std::io::Error> {
        fs::read_to_string(path)
    }
}

#[cfg(test)]
mod diff {
    use super::{FileReader, GramOpt, Repository};
    use crate::github::GithubClient;
    use async_trait::async_trait;
    use std::clone::Clone;
    use std::error::Error;
    use std::path::{Path, PathBuf};
    use tokio;

    struct FakeFileReader {
        file_as_str: String,
    }

    impl FileReader for FakeFileReader {
        fn read_to_string<P: AsRef<Path>>(&self, _: P) -> Result<String, std::io::Error> {
            Ok(self.file_as_str.clone())
        }
    }

    struct FakeGithubRepo {
        description: Option<String>,
    }

    #[async_trait]
    impl GithubClient for FakeGithubRepo {
        async fn repository(&self, _: &str, _: &str) -> Result<Repository, Box<dyn Error>> {
            Ok(Repository {
                description: self.description.to_owned(),
            })
        }
    }

    #[tokio::test]
    async fn it_should_error_if_settings_and_repo_have_different_description() {
        // arrange
        let github = FakeGithubRepo {
            description: Some("something".to_owned()),
        };
        let settings = FakeFileReader {
            file_as_str: r#"description = "test""#.to_owned(),
        };
        let opt = GramOpt::DiffSettings {
            owner: "wayofthepie".to_owned(),
            repo: "gram".to_owned(),
            settings: PathBuf::new(),
        };

        // act
        let result = opt.handle(github, settings).await;

        // arrange
        assert!(result.is_err(), "expected an error");
        let err = result.err();
        assert!(err.is_some());
        assert_eq!(
            format!("{}", err.unwrap()),
            "Current description [something] does not match expected description [test]"
        );
    }
}
