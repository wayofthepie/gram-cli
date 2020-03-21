mod settings;
use crate::github::{Github, GithubClient};
use anyhow::Result;
use settings::{GramSettings, SettingsCmd};
use std::fs;
use std::path::{Path, PathBuf};
use structopt::StructOpt;

/// Supported commands and options.
///
/// # Diff settings
/// `gram` supports diffing known settings defined in a settings toml file
/// against the current repository settings.
#[derive(Debug, StructOpt)]
#[structopt(name = "gram")]
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
    /// Interactions for repository settings.
    Settings {
        #[structopt(flatten)]
        cmd: SettingsCmd,
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
    pub async fn handle(self) -> Result<()> {
        let token = &self.token;
        let github = Github::new(token);
        let reader = SettingsReader::new();
        self.handle_internal(github, reader).await
    }

    /// Handle the command and args given to `gram`.
    async fn handle_internal<G, F>(self, github: G, reader: F) -> Result<()>
    where
        G: GithubClient,
        F: FileReader,
    {
        match self.command {
            GramOptCommand::Settings { cmd } => match cmd {
                SettingsCmd::Diff(diff) => diff.handle(reader, github).await,
            },
        }
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
    use super::{FileReader, GramOpt, GramOptCommand};
    use crate::commands::settings::{Diff, SettingsCmd};
    use crate::github::{GithubClient, Repository};
    use anyhow::Result;
    use async_trait::async_trait;
    use std::clone::Clone;
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
        let diff = Diff {
            owner: "".to_owned(),
            repo: "".to_owned(),
            settings: PathBuf::new(),
        };
        GramOptCommand::Settings {
            cmd: SettingsCmd::Diff(diff),
        }
    }

    fn default_opt() -> GramOpt {
        GramOpt {
            token: "".to_owned(),
            command: default_command(),
        }
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
