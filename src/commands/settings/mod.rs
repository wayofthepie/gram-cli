mod diff;
use crate::github::Repository;
pub use diff::Diff;
use serde::Deserialize;
use std::collections::HashMap;
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
#[derive(Debug, Deserialize)]
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

/// Allowing easy conversion between [Repository](struct.Repository.html)
/// and [GramSettings](struct.GramSettings.html) simplifies actions like diff.
impl From<Repository> for GramSettings {
    fn from(repo: Repository) -> Self {
        let mut options: Options = Options::default();
        options.allow_squash_merge = Some(repo.allow_squash_merge);
        options.allow_merge_commit = Some(repo.allow_merge_commit);
        options.allow_rebase_merge = Some(repo.allow_rebase_merge);
        options.delete_branch_on_merge = Some(repo.delete_branch_on_merge);
        Self {
            description: repo.description,
            options: Some(options),
        }
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

impl GramSettings {
    /// Get the diff between two [GramSettings](commands.struct.GramSettings.html).
    pub fn diff(&self, other: &GramSettings) -> Vec<String> {
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

#[cfg(test)]
mod test {
    use super::{
        GramSettings, Options, DESCRIPTION_KEY, OPTIONS_ALLOW_MERGE_COMMIT_KEY,
        OPTIONS_ALLOW_REBASE_MERGE_KEY, OPTIONS_ALLOW_SQUASH_MERGE_KEY,
        OPTIONS_DELETE_BRANCH_ON_MERGE_KEY,
    };
    use crate::github::Repository;
    use std::clone::Clone;
    use std::collections::HashMap;

    fn default_repo() -> Repository {
        Repository {
            description: Some("".to_owned()),
            allow_merge_commit: false,
            allow_squash_merge: false,
            allow_rebase_merge: false,
            delete_branch_on_merge: false,
        }
    }

    #[test]
    fn from_repository_for_gram_settings_should_construct_all_repo_fields() {
        // arrange
        let mut repo = default_repo();
        repo.description = Some("description".to_owned());
        repo.allow_merge_commit = true;
        repo.allow_squash_merge = true;
        repo.allow_rebase_merge = true;
        repo.delete_branch_on_merge = true;
        let r = repo.clone();

        // act
        let settings = GramSettings::from(repo);

        // assert
        assert_eq!(r.description, settings.description);

        let options = settings.options;
        assert!(
            options.is_some(),
            "expected options to be set, but it is None"
        );
        let options = options.unwrap();
        assert_eq!(options.allow_squash_merge.unwrap(), r.allow_squash_merge);
        assert_eq!(options.allow_merge_commit.unwrap(), r.allow_merge_commit);
        assert_eq!(options.allow_rebase_merge.unwrap(), r.allow_rebase_merge);
        assert_eq!(
            options.delete_branch_on_merge.unwrap(),
            r.delete_branch_on_merge
        );
    }

    #[test]
    fn from_gram_settings_for_hashmap_should_convert_all_fields_with_non_none_values() {
        // arrange
        let settings = GramSettings {
            description: Some("description".to_owned()),
            options: Some(Options {
                allow_squash_merge: Some(true),
                allow_merge_commit: Some(true),
                allow_rebase_merge: Some(true),
                delete_branch_on_merge: Some(true),
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
            delete_branch_on_merge,
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
        assert_eq!(
            hm.get(OPTIONS_DELETE_BRANCH_ON_MERGE_KEY)
                .map(|s| s.to_owned()),
            delete_branch_on_merge.map(|b| b.to_string())
        );
    }
}
