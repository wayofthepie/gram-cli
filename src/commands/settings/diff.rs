use super::GramSettings;
use crate::commands::FileReader;
use crate::github::{GithubClient, Repository};
use anyhow::{anyhow, Result};
use std::collections::HashMap;
use std::path::PathBuf;
use structopt::StructOpt;

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
    #[structopt(name = "file", short, long)]
    pub settings_file: PathBuf,
}

impl Diff {
    pub async fn handle<F, G>(self, reader: F, github: G) -> Result<()>
    where
        F: FileReader,
        G: GithubClient,
    {
        let configured_settings = reader.read_settings(&self.settings_file)?;
        let repo = github
            .get(&format!("/repos/{}/{}", &self.owner, &self.repo))
            .await?
            .json::<Repository>()
            .await?;
        let actual_settings = GramSettings::from(repo);
        let mut diffs = Diff::diff(&configured_settings, &actual_settings);
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

    /// Get the diff between two [GramSettings](commands.struct.GramSettings.html).
    fn diff(left: &GramSettings, right: &GramSettings) -> Vec<String> {
        let hm = HashMap::from(left);
        let other_hm = HashMap::from(right);
        hm.iter()
            .map(|(key, expected_val)| {
                let other_val = other_hm.get(key);
                if other_val == None {
                    return Some(format!(
                        "[{}]: expected [{}] but it has no value",
                        key, expected_val
                    ));
                }
                other_val.and_then(|other_val| {
                    if expected_val != other_val {
                        Some(format!(
                            "[{}]: expected [{}] got [{}]",
                            key, expected_val, other_val
                        ))
                    } else {
                        None
                    }
                })
            })
            .flatten()
            .collect::<Vec<String>>()
    }
}
