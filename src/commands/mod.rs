mod settings;
use crate::github::{Github, GithubClient, GITHUB_BASE_URL};
use anyhow::Result;
use settings::{FileReader, SettingsCmd, SettingsReader};
use structopt::StructOpt;

/// Supported commands and options.  
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
        let github = Github::new(self.token.clone(), GITHUB_BASE_URL);
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

