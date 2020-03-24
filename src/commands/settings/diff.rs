use super::{GramSettings, Options};
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

struct DiffableSettings<'a>(&'a GramSettings);

impl Diff {
    pub async fn handle<F, G>(self, reader: F, github: G) -> Result<()>
    where
        F: FileReader,
        G: GithubClient,
    {
        let configured_settings = reader.read_settings(&self.settings_file)?;
        let actual_settings = self.get_actual_settings(&github).await?;
        let mut diffs = Diff::diff(
            DiffableSettings(&configured_settings),
            DiffableSettings(&actual_settings),
        );
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

    async fn get_actual_settings<G: GithubClient>(self, github: &G) -> Result<GramSettings> {
        let repo = github
            .get::<Repository>(&format!("/repos/{}/{}", &self.owner, &self.repo))
            .await?;
        let settings = GramSettings {
            description: repo.description,
            options: Some(Options {
                allow_squash_merge: Some(repo.allow_squash_merge),
                allow_merge_commit: Some(repo.allow_merge_commit),
                allow_rebase_merge: Some(repo.allow_rebase_merge),
                delete_branch_on_merge: Some(repo.delete_branch_on_merge),
            }),
        };
        Ok(settings)
    }

    /// Get the diff between two [GramSettings](commands.struct.GramSettings.html).
    fn diff(left: DiffableSettings, right: DiffableSettings) -> Vec<String> {
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

static DESCRIPTION_KEY: &str = "description";
static OPTIONS_ALLOW_SQUASH_MERGE_KEY: &str = "options.allow-squash-merge";
static OPTIONS_ALLOW_MERGE_COMMIT_KEY: &str = "options.allow-merge-commit";
static OPTIONS_ALLOW_REBASE_MERGE_KEY: &str = "options.allow-rebase-merge";
static OPTIONS_DELETE_BRANCH_ON_MERGE_KEY: &str = "options.delete-branch-on-merge";

// TODO: it would be nicer to use a macro/proc-macro to generate this
// instance. Then the keys can be taken directly from the field names.
//
// Tell clippy to ignore the implicit hasher here. We want to use the default.
#[allow(clippy::implicit_hasher)]
impl<'a> From<DiffableSettings<'a>> for HashMap<&'a str, String> {
    fn from(settings: DiffableSettings) -> Self {
        let GramSettings {
            description,
            options,
        } = settings.0;
        let mut hm = Self::new();
        description
            .as_ref()
            .map(|val| hm.insert(DESCRIPTION_KEY, val.to_owned()));
        if let Some(opts) = options.as_ref() {
            let Options {
                allow_squash_merge,
                allow_merge_commit,
                allow_rebase_merge,
                delete_branch_on_merge,
            } = opts;
            allow_squash_merge
                .map(|allow| hm.insert(OPTIONS_ALLOW_SQUASH_MERGE_KEY, allow.to_string()));
            allow_merge_commit
                .map(|allow| hm.insert(OPTIONS_ALLOW_MERGE_COMMIT_KEY, allow.to_string()));
            allow_rebase_merge
                .map(|allow| hm.insert(OPTIONS_ALLOW_REBASE_MERGE_KEY, allow.to_string()));
            delete_branch_on_merge
                .map(|delete| hm.insert(OPTIONS_DELETE_BRANCH_ON_MERGE_KEY, delete.to_string()));
        }
        hm
    }
}

#[cfg(test)]
mod test {
    use super::{Diff, FileReader, GithubClient};
    use super::{GramSettings, Options};
    use crate::github::Repository;
    use anyhow::anyhow;
    use async_trait::async_trait;
    use serde::{de::DeserializeOwned, Serialize};
    use std::clone::Clone;
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};

    struct FakeGithub<'a, R> {
        error: Option<String>,
        result: &'a R,
    }

    #[async_trait]
    impl<'a, R> GithubClient for FakeGithub<'a, R>
    where
        R: Clone + Serialize + Sync,
    {
        async fn get<T>(&self, _: &str) -> anyhow::Result<T>
        where
            T: DeserializeOwned,
        {
            if self.error.is_some() {
                Err(anyhow!(self.error.clone().unwrap()))
            } else {
                // TODO: figure out a better way of faking this generic function ...
                let json = serde_json::to_string(&self.result.clone()).unwrap();
                Ok(serde_json::from_str::<T>(&json)?)
            }
        }
    }

    struct FakeFileReader<'a> {
        error: bool,
        settings: &'a GramSettings,
    }

    impl<'a> FileReader for FakeFileReader<'a> {
        fn read_to_string<P: AsRef<Path>>(&self, _: P) -> Result<String, std::io::Error> {
            if self.error {
                Err(std::io::Error::new(ErrorKind::Other, ""))
            } else {
                Ok("".to_owned())
            }
        }

        fn read_settings(&self, _: &PathBuf) -> anyhow::Result<GramSettings> {
            Ok(self.settings.clone())
        }
    }

    static SETTINGS_DESCRIPTION: &'static str = "description";
    static REPO_DESCRIPTION: &'static str = "different";

    fn opposing_settings_and_repo() -> (GramSettings, Repository) {
        let settings = GramSettings {
            description: Some(SETTINGS_DESCRIPTION.to_owned()),
            options: Some(Options {
                delete_branch_on_merge: Some(true),
                allow_rebase_merge: Some(true),
                allow_squash_merge: Some(true),
                allow_merge_commit: Some(false),
            }),
        };
        let repo = Repository {
            description: Some(REPO_DESCRIPTION.to_owned()),
            allow_merge_commit: true,
            allow_squash_merge: false,
            allow_rebase_merge: false,
            delete_branch_on_merge: false,
        };
        (settings, repo)
    }

    fn passing_reader<'a>(settings: &'a GramSettings) -> FakeFileReader<'a> {
        FakeFileReader {
            error: false,
            settings,
        }
    }

    fn default_diff() -> Diff {
        Diff {
            owner: "".to_owned(),
            repo: "".to_owned(),
            settings_file: PathBuf::new().to_owned(),
        }
    }

    async fn setup_diff_error_test() -> (GramSettings, Repository, Vec<String>) {
        // arrange
        let (settings, repo) = opposing_settings_and_repo();
        let diff = default_diff();
        let github = FakeGithub::<Repository> {
            error: None,
            result: &repo,
        };
        // act
        let result = diff.handle(passing_reader(&settings), github).await;

        // assert
        assert!(result.is_err());
        let err = format!("{}", result.err().unwrap());
        let err = err.trim();
        let diffs = err
            .split("\n")
            .skip(1)
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();
        (settings, repo, diffs)
    }

    #[tokio::test]
    async fn diff_error_for_differing_settings_should_contain_a_line_per_failed_setting() {
        let (settings, repo, diffs) = setup_diff_error_test().await;
        let options = settings.options.unwrap();
        let description_err = format!(
            "[description]: expected [{}] got [{}]",
            SETTINGS_DESCRIPTION, REPO_DESCRIPTION
        );
        let allow_merge_commit_error = format!(
            "[options.allow-merge-commit]: expected [{}] got [{}]",
            options.allow_merge_commit.clone().unwrap(),
            repo.allow_merge_commit
        );
        let allow_rebase_merge_err = format!(
            "[options.allow-rebase-merge]: expected [{}] got [{}]",
            options.allow_rebase_merge.clone().unwrap(),
            repo.allow_rebase_merge
        );
        let allow_squash_merge_error = format!(
            "[options.allow-squash-merge]: expected [{}] got [{}]",
            options.allow_squash_merge.clone().unwrap(),
            repo.allow_squash_merge
        );

        assert_eq!(description_err, diffs[0]);
        assert_eq!(allow_merge_commit_error, diffs[1]);
        assert_eq!(allow_rebase_merge_err, diffs[2]);
        assert_eq!(allow_squash_merge_error, diffs[3]);
    }

    #[tokio::test]
    async fn diff_should_error_with_differences_if_repo_and_settings_differ() {
        // arrange
        let mut settings = GramSettings::default();
        settings.description = Some("blah".to_owned());
        let reader = FakeFileReader {
            error: false,
            settings: &settings,
        };
        let mut repo = Repository::default();
        repo.description = Some("different".to_owned());
        let github = FakeGithub::<Repository> {
            error: None,
            result: &repo,
        };
        let diff = Diff {
            owner: "".to_owned(),
            repo: "".to_owned(),
            settings_file: PathBuf::new().to_owned(),
        };

        // act
        let result = diff.handle(reader, github).await;

        // assert
        println!("{:#?}", result);
        assert!(result.is_err());
        assert!(
            format!("{}", result.err().unwrap()).contains("Actual settings differ from expected!")
        );
    }

    #[tokio::test]
    async fn diff_should_error_if_get_for_repo_fails() {
        // arrange
        let reader = FakeFileReader {
            error: false,
            settings: &GramSettings::default(),
        };
        let github = FakeGithub::<String> {
            error: None,
            result: &"error".to_owned(),
        };

        let diff = Diff {
            owner: "".to_owned(),
            repo: "".to_owned(),
            settings_file: PathBuf::new().to_owned(),
        };

        // act
        let result = diff.handle(reader, github).await;

        // assert
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn diff_should_error_if_reading_settings_file_fails() {
        // arrange
        let failing_reader = FakeFileReader {
            error: true,
            settings: &GramSettings::default(),
        };
        let diff = Diff {
            owner: "".to_owned(),
            repo: "".to_owned(),
            settings_file: PathBuf::new(),
        };

        let github = FakeGithub::<String> {
            error: Some("".to_owned()),
            result: &"".to_owned(),
        };

        // act
        let result = diff.handle(failing_reader, github).await;

        // assert
        assert!(result.is_err());
    }
}
