mod diff;
use anyhow::Result;
pub use diff::Diff;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

/// Supported settings subcommands.
#[derive(Debug, StructOpt)]
pub enum SettingsCmd {
    Diff(Diff),
}

/// Repository settings that `gram` is able to see.
///
/// Any settings that are not defined here will be ignored in all
/// `gram` commands.
#[derive(Debug, Default, Clone, Deserialize)]
pub struct GramSettings {
    pub description: Option<String>,
    pub options: Option<Options>,
}

/// Represents settings that appear under a repositories Settings -> Options section.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Options {
    #[serde(rename = "allow-squash-merge")]
    pub allow_squash_merge: Option<bool>,
    #[serde(rename = "allow-merge-commit")]
    pub allow_merge_commit: Option<bool>,
    #[serde(rename = "allow-rebase-merge")]
    pub allow_rebase_merge: Option<bool>,
    #[serde(rename = "delete-branch-on-merge")]
    pub delete_branch_on_merge: Option<bool>,
}

impl Copy for Options {}

pub struct SettingsReader;

impl SettingsReader {
    pub fn new() -> Self {
        Self {}
    }
}

pub trait FileReader {
    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String, std::io::Error>;

    fn read_settings(&self, settings_location: &PathBuf) -> Result<GramSettings> {
        let settings_str = self.read_to_string(settings_location)?;
        let settings = toml::from_str::<GramSettings>(&settings_str)?;
        Ok(settings)
    }
}

impl FileReader for SettingsReader {
    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String, std::io::Error> {
        fs::read_to_string(path)
    }
}

