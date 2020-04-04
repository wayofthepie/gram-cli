use crate::{
    commands::settings::{GramSettings, Options},
    github::GithubClient,
};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Retrieve {
    async fn retrieve(&self, owner: &str, repo: &str) -> Result<GramSettings>;
}

pub struct RetrieveSettings<'a, C: GithubClient> {
    client: &'a C,
}

#[async_trait]
impl<C> Retrieve for RetrieveSettings<'_, C>
where
    C: GithubClient + Send + Sync,
{
    async fn retrieve(&self, owner: &str, repo: &str) -> Result<GramSettings> {
        let repository = self.client.repository(owner, repo).await?;
        Ok(GramSettings {
            description: repository.description,
            options: Some(Options {
                allow_squash_merge: Some(repository.allow_squash_merge),
                allow_merge_commit: Some(repository.allow_merge_commit),
                allow_rebase_merge: Some(repository.allow_rebase_merge),
                delete_branch_on_merge: Some(repository.delete_branch_on_merge),
            }),
        })
    }
}

#[cfg(test)]
mod test {
    use super::{Retrieve, RetrieveSettings};
    use crate::github::{GithubClient, Repository};
    use anyhow::anyhow;
    use async_trait::async_trait;

    struct FailingClient {}
    struct SucceedingClient {}

    #[async_trait]
    impl GithubClient for FailingClient {
        async fn repository(&self, _: &str, _: &str) -> anyhow::Result<Repository> {
            Err(anyhow!("error"))
        }
    }

    #[async_trait]
    impl GithubClient for SucceedingClient {
        async fn repository(&self, _: &str, _: &str) -> anyhow::Result<Repository> {
            Ok(Repository::default())
        }
    }

    #[tokio::test]
    async fn should_return_settings() {
        // arrange
        let client = SucceedingClient {};
        let retriever = RetrieveSettings { client: &client };

        // act
        let opt_settings = retriever.retrieve("", "").await;

        // assert
        assert!(opt_settings.is_ok());
    }

    #[tokio::test]
    async fn should_return_error_if_repo_fetch_fails() {
        // arrange
        let client = FailingClient {};
        let retriever = RetrieveSettings { client: &client };

        // act
        let opt_settings = retriever.retrieve("", "").await;

        // assert
        assert!(opt_settings.is_err());
    }
}
