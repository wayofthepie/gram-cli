use crate::github::{Github, GithubClient, Repository};
use anyhow::{anyhow, Result};
use serde::Deserialize;
use std::collections::HashMap;
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
    /// Handle arguments passed to `gram`.
    ///
    /// This is the first place we have access to our arguments so we don't expose
    /// the github client or settings reader on its contract. The github client may
    /// be used with a token, and this is the first place we can access that token.
    /// This does lead to having to test the [handle_internal](struct.GramOpt.html#method.handle_internal)
    /// function instead, this is ok in this case.
    pub async fn handle(&self) -> Result<()> {
        let token = &self.token;
        let github = Github::new(token);
        let reader = SettingsReader::new();
        self.handle_internal(github, reader).await
    }

    /// Handle the command and args given to `gram`.
    async fn handle_internal<G, F>(&self, github: G, reader: F) -> Result<()>
    where
        G: GithubClient,
        F: FileReader,
    {
        match &self.command {
            GramOptCommand::DiffSettings {
                owner,
                repo,
                settings,
            } => {
                let settings = reader.read_settings(&settings)?;
                let repo = github.repository(&owner, &repo).await?;
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
    }
}

/// Repository settings that `gram` is able to see.
///
/// Any settings that are not defined here will be ignored in all
/// `gram` commands.
#[derive(Debug, Deserialize)]
pub struct GramSettings {
    description: Option<String>,
    options: Option<Options>,
}

/// Represents settings that appear under a repositories Settings -> Options section.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Options {
    #[serde(rename = "allow-squash-merge")]
    allow_squash_merge: Option<bool>,
    #[serde(rename = "allow-merge-commit")]
    allow_merge_commit: Option<bool>,
    #[serde(rename = "allow-rebase-merge")]
    allow_rebase_merge: Option<bool>,
}

impl Copy for Options {}

/// Allowing easy conversion between [Repository](struct.Repository.html)
/// and [GramSettings](struct.GramSettings.html) simplifies actions like diff.
impl From<Repository> for GramSettings {
    fn from(repo: Repository) -> Self {
        let mut has_option = false;
        let mut options: Options = Options::default();
        if repo.allow_squash_merge.is_some() {
            has_option = true;
            options.allow_squash_merge = repo.allow_squash_merge;
        }
        if repo.allow_merge_commit.is_some() {
            has_option = true;
            options.allow_merge_commit = repo.allow_merge_commit;
        }
        if repo.allow_rebase_merge.is_some() {
            has_option = true;
            options.allow_rebase_merge = repo.allow_rebase_merge;
        }
        Self {
            description: repo.description,
            options: if has_option { Some(options) } else { None },
        }
    }
}

static DESCRIPTION_KEY: &str = "description";
static OPTIONS_ALLOW_SQUASH_MERGE_KEY: &str = "options.allow-squash-merge";
static OPTIONS_ALLOW_MERGE_COMMIT_KEY: &str = "options.allow-merge-commit";
static OPTIONS_ALLOW_REBASE_MERGE_KEY: &str = "options.allow-rebase-merge";

// TODO: it would be nicer to use a macro/proc-macro to generate this
// instance. Then the keys can be taken directly from the field names.
//
// Tell clippy to ignore the implicit hasher here. We want to used the default.
#[allow(clippy::implicit_hasher)]
impl<'a> From<&'a GramSettings> for HashMap<&'a str, String> {
    fn from(settings: &'a GramSettings) -> Self {
        let GramSettings {
            description,
            options,
        } = settings;
        let mut hm = Self::new();
        description
            .as_ref()
            .map(|val| hm.insert(DESCRIPTION_KEY, val.to_owned()));
        options.as_ref().map(|opts| {
            opts.allow_squash_merge
                .map(|allow| hm.insert(OPTIONS_ALLOW_SQUASH_MERGE_KEY, allow.to_string()));
            opts.allow_merge_commit
                .map(|allow| hm.insert(OPTIONS_ALLOW_MERGE_COMMIT_KEY, allow.to_string()));
            opts.allow_rebase_merge
                .map(|allow| hm.insert(OPTIONS_ALLOW_REBASE_MERGE_KEY, allow.to_string()));
        });
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

pub trait FileReader {
    fn read_to_string<P: AsRef<Path>>(&self, path: P) -> Result<String, std::io::Error>;

    fn read_settings(&self, settings_location: &PathBuf) -> Result<GramSettings> {
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
    use super::{
        FileReader, GramOpt, GramOptCommand, GramSettings, Options, DESCRIPTION_KEY,
        OPTIONS_ALLOW_MERGE_COMMIT_KEY, OPTIONS_ALLOW_REBASE_MERGE_KEY,
        OPTIONS_ALLOW_SQUASH_MERGE_KEY,
    };
    use crate::github::{GithubClient, Repository};
    use anyhow::Result;
    use async_trait::async_trait;
    use std::clone::Clone;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use tokio;

    #[derive(Default)]
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
        async fn repository(&self, _: &str, _: &str) -> Result<Repository> {
            Ok(Repository {
                description: self.description.to_owned(),
                allow_squash_merge: None,
                allow_merge_commit: None,
                allow_rebase_merge: None,
            })
        }
    }

    fn default_command() -> GramOptCommand {
        GramOptCommand::DiffSettings {
            owner: "".to_owned(),
            repo: "".to_owned(),
            settings: PathBuf::new(),
        }
    }

    fn default_opt() -> GramOpt {
        GramOpt {
            token: "".to_owned(),
            command: default_command(),
        }
    }

    impl Default for Repository {
        fn default() -> Self {
            Repository {
                description: None,
                allow_merge_commit: None,
                allow_squash_merge: None,
                allow_rebase_merge: None,
            }
        }
    }

    #[test]
    fn from_repository_for_gram_settings_should_construct_all_repo_fields() {
        // arrange
        let mut repo = Repository::default();
        repo.description = Some("description".to_owned());
        repo.allow_merge_commit = Some(true);
        repo.allow_squash_merge = Some(true);
        repo.allow_rebase_merge = Some(true);
        let r = repo.clone();

        // act
        let settings = GramSettings::from(repo);

        // assert
        assert_eq!(settings.description, r.description);

        let options = settings.options;
        assert!(
            options.is_some(),
            "expected options to be set, but it is None"
        );
        assert_eq!(options.unwrap().allow_squash_merge, r.allow_squash_merge);
        assert_eq!(options.unwrap().allow_merge_commit, r.allow_merge_commit);
        assert_eq!(options.unwrap().allow_rebase_merge, r.allow_rebase_merge);
    }

    #[test]
    fn from_gram_settings_for_hashmap_should_convert_all_fields_with_non_none_values() {
        // arrange
        let settings = GramSettings {
            description: Some("description".to_owned()),
            options: Some(Options {
                allow_squash_merge: Some(true),
                allow_merge_commit: None,
                allow_rebase_merge: None,
            }),
        };
        // Destructure so the compiler will give out if there are unused fields on
        // the left hand side. Prevents mistakes where the value is added to `settings`
        // in the definition above, but never used after that.
        let GramSettings {
            description,
            options,
        } = &settings;
        let Options {
            allow_squash_merge,
            allow_merge_commit,
            allow_rebase_merge,
        } = options.unwrap();

        // act
        let hm = HashMap::from(&settings);

        // assert
        assert_eq!(hm.get(DESCRIPTION_KEY).map(|s| s.to_owned()), *description);
        assert_eq!(
            hm.get(OPTIONS_ALLOW_SQUASH_MERGE_KEY).map(|s| s.to_owned()),
            allow_squash_merge.map(|b| b.to_string())
        );
        assert_eq!(
            hm.get(OPTIONS_ALLOW_MERGE_COMMIT_KEY).map(|s| s.to_owned()),
            allow_merge_commit.map(|b| b.to_string())
        );
        assert_eq!(
            hm.get(OPTIONS_ALLOW_REBASE_MERGE_KEY).map(|s| s.to_owned()),
            allow_rebase_merge.map(|b| b.to_string())
        );
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
        let opt = default_opt();

        // act
        let result = opt.handle_internal(github, settings).await;

        // arrange
        assert_diff_error(
            result,
            vec![
                "Actual settings differ from expected!",
                "[description]: expected [test] but it has no value",
            ],
        );
    }

    #[tokio::test]
    async fn handle_it_should_error_if_settings_toml_and_repo_have_different_description() {
        // arrange
        let mut github = FakeGithubRepo::default();
        github.description = Some("something else".to_owned());
        let mut reader = FakeFileReader::default();
        reader.file_as_str = r#"
                description = "test"
            "#
        .to_owned();
        let opt = default_opt();

        // act
        let result = opt.handle_internal(github, reader).await;

        // arrange
        assert_diff_error(
            result,
            vec![
                "Actual settings differ from expected!",
                "[description]: expected [test] got [something else]",
            ],
        );
    }

    fn assert_diff_error(result: Result<()>, expected_lines: Vec<&str>) {
        assert!(result.is_err(), "expected an error, got {:#?}", result);

        let err = result.err();
        assert!(err.is_some(), "expected error to have an 'err' value");

        let err_str = format!("{}", err.unwrap());
        let lines = err_str.trim().split('\n').collect::<Vec<&str>>();

        assert!(
            lines.len() == expected_lines.len(),
            concat!(
                "the number of lines in the error does not match expected",
                ", got:\n{:#?}\nexpected\n{:#?}\n",
            ),
            lines,
            expected_lines
        );
        expected_lines
            .iter()
            .enumerate()
            .for_each(|(index, &expected)| {
                assert_eq!(
                    lines[index], expected,
                    "line {} has unexpected value",
                    index
                )
            });
    }
}
