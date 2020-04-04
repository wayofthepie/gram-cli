pub mod retrieve;
use super::{GramSettings, Options};
use crate::commands::FileReader;
use anyhow::{anyhow, Result};
use retrieve::Retrieve;
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
    pub async fn handle<F, R>(self, reader: F, retriever: R) -> Result<()>
    where
        F: FileReader,
        R: Retrieve,
    {
        let configured_settings = reader.read_settings(&self.settings_file)?;
        let actual_settings = retriever.retrieve(&self.owner, &self.repo).await?;
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
static PROTECTED: &str = "protected";

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
            protected,
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
        if let Some(branches) = protected {
            let branches_str = branches
                .iter()
                .fold(String::new(), |mut acc, branch| {
                    acc.push_str(&branch.name);
                    acc.push_str(" ");
                    acc
                })
                .trim()
                .to_owned();
            hm.insert(PROTECTED, branches_str);
        }
        hm
    }
}

#[cfg(test)]
mod test {
    use super::{retrieve::Retrieve, Diff, FileReader};
    use crate::commands::settings::{GramSettings, Options, ProtectedBranch};
    use anyhow::anyhow;
    use async_trait::async_trait;

    use std::clone::Clone;

    use std::path::{Path, PathBuf};

    struct SucceedingFileReader<'a> {
        settings: &'a GramSettings,
    }

    struct FailingFileReader {}

    impl<'a> FileReader for SucceedingFileReader<'a> {
        fn read_to_string<P: AsRef<Path>>(&self, _: P) -> Result<String, std::io::Error> {
            unimplemented!()
        }

        fn read_settings(&self, _: &PathBuf) -> anyhow::Result<GramSettings> {
            Ok(self.settings.clone())
        }
    }

    impl<'a> FileReader for FailingFileReader {
        fn read_to_string<P: AsRef<Path>>(
            &self,
            _path: P,
        ) -> anyhow::Result<String, std::io::Error> {
            unimplemented!()
        }

        fn read_settings(&self, _settings_location: &PathBuf) -> anyhow::Result<GramSettings> {
            Err(anyhow!(""))
        }
    }

    struct FakeRetriever {
        settings: Option<GramSettings>,
    }

    #[async_trait]
    impl Retrieve for FakeRetriever {
        async fn retrieve(&self, _owner: &str, _repo: &str) -> anyhow::Result<GramSettings> {
            match &self.settings {
                Some(settings) => Ok(settings.clone()),
                None => Err(anyhow!("")),
            }
        }
    }

    fn local_settings() -> GramSettings {
        GramSettings {
            description: Some("a".to_owned()),
            options: Some(Options {
                allow_squash_merge: Some(true),
                allow_merge_commit: Some(true),
                allow_rebase_merge: Some(true),
                delete_branch_on_merge: Some(true),
            }),
            protected: Some(vec![
                ProtectedBranch {
                    name: "a".to_owned(),
                },
                ProtectedBranch {
                    name: "b".to_owned(),
                },
            ]),
        }
    }

    fn repo_settings() -> GramSettings {
        GramSettings {
            description: Some("b".to_owned()),
            options: Some(Options {
                allow_squash_merge: Some(false),
                allow_merge_commit: Some(false),
                allow_rebase_merge: Some(false),
                delete_branch_on_merge: Some(false),
            }),
            protected: Some(vec![ProtectedBranch {
                name: "b".to_owned(),
            }]),
        }
    }

    fn default_diff() -> Diff {
        Diff {
            owner: "".to_owned(),
            repo: "".to_owned(),
            settings_file: PathBuf::new(),
        }
    }

    #[tokio::test]
    async fn diff_error_for_differing_settings_should_contain_a_line_per_error_when_values_differ()
    {
        let local_settings = local_settings();
        let repo_settings = repo_settings();
        let diff = default_diff();

        let reader = SucceedingFileReader {
            settings: &local_settings.clone(),
        };
        let retriever = FakeRetriever {
            settings: Some(repo_settings.clone()),
        };

        // act
        let result = diff.handle(reader, retriever).await;

        // assert
        assert!(result.is_err());
        let diffs = format!("{}", result.err().unwrap())
            .trim()
            .split("\n")
            .skip(1)
            .map(|s| s.to_owned())
            .collect::<Vec<String>>();

        let description_err = format!(
            "[description]: expected [{}] got [{}]",
            local_settings.description.unwrap(),
            repo_settings.description.unwrap()
        );
        let local_options = local_settings.options.unwrap();
        let repo_options = repo_settings.options.unwrap();
        let allow_merge_commit_error = format!(
            "[options.allow-merge-commit]: expected [{}] got [{}]",
            local_options.allow_merge_commit.unwrap(),
            repo_options.allow_merge_commit.unwrap()
        );
        let allow_rebase_merge_err = format!(
            "[options.allow-rebase-merge]: expected [{}] got [{}]",
            local_options.allow_rebase_merge.unwrap(),
            repo_options.allow_rebase_merge.unwrap()
        );
        let allow_squash_merge_error = format!(
            "[options.allow-squash-merge]: expected [{}] got [{}]",
            local_options.allow_squash_merge.unwrap(),
            repo_options.allow_squash_merge.unwrap(),
        );
        let delete_branch_on_merge_error = format!(
            "[options.delete-branch-on-merge]: expected [{}] got [{}]",
            local_options.delete_branch_on_merge.unwrap(),
            repo_options.delete_branch_on_merge.unwrap()
        );
        let protected_branch_master_error = format!(
            "[protected]: expected [{}] got [{}]",
            local_settings.protected.unwrap()[0].name,
            repo_settings.protected.unwrap()[0].name
        );
        assert_eq!(description_err, diffs[0]);
        assert_eq!(allow_merge_commit_error, diffs[1]);
        assert_eq!(allow_rebase_merge_err, diffs[2]);
        assert_eq!(allow_squash_merge_error, diffs[3]);
        assert_eq!(delete_branch_on_merge_error, diffs[4]);
        assert_eq!(protected_branch_master_error, diffs[5]);
    }
}
