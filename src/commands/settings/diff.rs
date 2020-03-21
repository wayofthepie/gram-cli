use super::GramSettings;
use crate::commands::FileReader;
use crate::github::GithubClient;
use anyhow::{anyhow, Result};
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

/// Diff actual settings with expected settings
/// defined in a settings toml file.
///
/// gram will only diff settings defined in the given toml file. It
/// will not mention any settings which are not defined in that file.
#[derive(Debug, StructOpt)]
pub struct Diff {
    /// The owner of the repository.
    #[structopt(short, long)]
    pub owner: String,

    /// The name of the repository.
    #[structopt(short, long)]
    pub repo: String,

    /// Path to the settings TOML file.
    #[structopt(short, long, help = SETTINGS_HELP)]
    pub settings: PathBuf,
}

impl Diff {
    pub async fn handle<F, G>(self, reader: F, github: G) -> Result<()>
    where
        F: FileReader,
        G: GithubClient,
    {
        let settings = reader.read_settings(&self.settings)?;
        let repo = github.repository(&self.owner, &self.repo).await?;
        let actual_settings = GramSettings::from(repo);
        let mut diffs = settings.diff(&actual_settings);
        match diffs.as_slice() {
            [] => Ok(()),
            [..] => {
                diffs.sort();
                let errors = diffs.iter().fold(String::new(), |mut acc, diff| {
                    acc.push_str(diff);
                    acc.push('\n');
                    acc
                });
                Err(anyhow!("Actual settings differ from expected!\n{}", errors))
            }
        }
    }
}