use crate::github::{GithubClient, Repository};
use serde::Deserialize;
use std::collections::HashMap;
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

/// Repository settings that `gram` is able to see.
///
/// Any settings that are not defined here will be ignored in all
/// `gram` commands.
#[derive(Debug, Deserialize)]
pub struct GramSettings {
    description: Option<String>,
}

/// Allowing easy conversion between [Repository](struct.Repository.html)
/// and [GramSettings](struct.GramSettings.html) simplifies actions like diff.
impl From<Repository> for GramSettings {
    fn from(repo: Repository) -> Self {
        Self {
            description: repo.description,
        }
    }
}

static DESCRIPTION_KEY: &str = "description";

// TODO: it would be nicer to use a macro/proc-macro to generate this
// instance. Then the keys can be taken directly from the field names.
impl<'a> From<&'a GramSettings> for HashMap<&'a str, &'a str> {
    fn from(settings: &'a GramSettings) -> Self {
        let GramSettings { description } = settings;
        let mut hm = Self::new();
        description
            .as_ref()
            .map(|val| hm.insert(DESCRIPTION_KEY, val));
        hm
    }
}

impl GramSettings {
    /// Get the diff between two [GramSettings](commands.struct.GramSettings.html).
    fn diff(&self, other: &GramSettings) -> Vec<String> {
        let hm = HashMap::from(self);
        let other_hm = HashMap::from(other);
        hm.iter()
            .map(|(key, expected_val)| {
                let other_val = other_hm.get(key);
                if other_val == None {
                    return Some(format!(
                        "For setting [{}]: expected [{}] but it has no value",
                        key, expected_val
                    ));
                }
                other_val.and_then(|other_val| {
                    if expected_val != other_val {
                        Some(format!(
                            "For setting [{}]: expected [{}] got [{}]",
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

/// Supported commands and options.
///
/// # Diff settings
/// `gram` supports diffing known settings defined in a settings toml file
/// against the current repository settings. e.g.
///
/// ```shell
/// $ gram diff-settings -t ${TOKEN} -o ${OWNER} -r ${REPO} -s ${PATH_TO_TOML}
/// ```
#[derive(Debug, StructOpt)]
pub struct GramOpt {
    /// Github token to use.
    ///
    /// This is a Personal Access token that gram can use to authenticate with
    /// github. It can also be set as an environment variable called GITHUB_TOKEN.
    #[structopt(long, short, env = "GITHUB_TOKEN")]
    token: String,

    /// Subcommands
    #[structopt(subcommand)]
    command: GramOptCommand,
}

/// Supported commands.
#[derive(Debug, StructOpt)]
pub enum GramOptCommand {
    /// Diff actual settings with expected settings
    /// defined in a settings toml file.
    ///
    /// gram will only diff settings defined in the given toml file. It
    /// will not mention any settings which are not defined in that file.
    DiffSettings {
        /// The owner of the repository.
        #[structopt(short, long)]
        owner: String,

        /// The name of the repository.
        #[structopt(short, long)]
        repo: String,

        /// Path to the settings TOML file.
        #[structopt(short, long, help = SETTINGS_HELP)]
        settings: PathBuf,
    },
}

impl GramOpt {
    /// Handle the command and args given to `gram`.
    pub async fn handle<G, F>(self, github: G, reader: F) -> Result<(), Box<dyn Error>>
    where
        G: GithubClient,
        F: FileReader,
    {
        match self.command {
            GramOptCommand::DiffSettings {
                owner,
                repo,
                settings,
            } => {
                let settings = reader.read_settings(&settings)?;
                let repo = github.repository(&owner, &repo).await?;
                let actual_settings = GramSettings::from(repo);
                let diffs = settings.diff(&actual_settings);
                match diffs.as_slice() {
                    [] => Ok(()),
                    [..] => Err(diffs.iter().fold(String::new(), |mut acc, diff| {
                        acc.push_str(diff);
                        acc
                    }))?,
                }
            }
        }
    }
}

pub trait FileReader {
    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String, std::io::Error>;

    fn read_settings(&self, settings_location: &PathBuf) -> Result<GramSettings, Box<dyn Error>> {
        let settings_str = self.read_to_string(settings_location)?;
        let settings = toml::from_str::<GramSettings>(&settings_str)?;
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
mod test {
    use super::{FileReader, GramOpt, GramOptCommand, GramSettings};
    use crate::github::{GithubClient, Repository};
    use async_trait::async_trait;
    use std::clone::Clone;
    use std::collections::HashMap;
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

    #[derive(Default)]
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

    #[test]
    fn from_gram_settings_for_hashmap_should_convert_all_fields_with_non_none_values() {
        // arrange
        let settings = GramSettings {
            description: Some("description".to_owned()),
        };
        // Destructure so the compiler will give out if there are unused fields on
        // the left hand side. Prevents mistakes where the value is added to `settings`
        // in the definition above, but never used after that.
        let GramSettings { description } = &settings;

        // act
        let hm = HashMap::from(&settings);

        // assert
        assert_eq!(hm.get("description").map(|s| *s), description.as_deref());
    }

    #[tokio::test]
    async fn handle_it_should_error_if_settings_toml_has_a_value_but_the_repo_does_not() {
        // arrange
        let github = FakeGithubRepo::default();
        let settings = FakeFileReader {
            file_as_str: r#"
                description = "test"
            "#
            .to_owned(),
        };
        let command = GramOptCommand::DiffSettings {
            owner: "wayofthepie".to_owned(),
            repo: "gram".to_owned(),
            settings: PathBuf::new(),
        };
        let opt = GramOpt {
            token: "".to_owned(),
            command,
        };

        // act
        let result = opt.handle(github, settings).await;

        // arrange
        assert!(result.is_err(), "expected an error");
        let err = result.err();
        assert!(err.is_some());
        assert_eq!(
            format!("{}", err.unwrap()),
            "For setting [description]: expected [test] but it has no value"
        );
    }

    #[tokio::test]
    async fn handle_it_should_error_if_settings_and_repo_have_different_description() {
        // arrange
        let mut github = FakeGithubRepo::default();
        github.description = Some("something else".to_owned());
        let settings = FakeFileReader {
            file_as_str: r#"
                description = "test"
            "#
            .to_owned(),
        };
        let command = GramOptCommand::DiffSettings {
            owner: "wayofthepie".to_owned(),
            repo: "gram".to_owned(),
            settings: PathBuf::new(),
        };
        let opt = GramOpt {
            token: "".to_owned(),
            command,
        };

        // act
        let result = opt.handle(github, settings).await;

        // arrange
        assert!(result.is_err(), "expected an error");
        let err = result.err();
        assert!(err.is_some());
        assert_eq!(
            format!("{}", err.unwrap()),
            "For setting [description]: expected [test] got [something else]"
        );
    }
}
