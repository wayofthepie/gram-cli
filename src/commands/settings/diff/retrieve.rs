use crate::{
    commands::settings::{GramSettings, Options, ProtectedBranch},
    github::GithubClient,
};
use anyhow::Result;
use async_trait::async_trait;

#[async_trait]
pub trait Retrieve {
    async fn retrieve(&self, owner: &str, repo: &str) -> Result<GramSettings>;
}

pub struct RetrieveSettings<'a, C> {
    client: &'a C,
}

impl<'a, C> RetrieveSettings<'a, C>
where
    C: GithubClient + Send + Sync,
{
    pub fn new(client: &'a C) -> Self {
        RetrieveSettings { client }
    }
}

#[async_trait]
impl<C> Retrieve for RetrieveSettings<'_, C>
where
    C: GithubClient + Send + Sync,
{
    async fn retrieve(&self, owner: &str, repo: &str) -> Result<GramSettings> {
        let repository = self.client.repository(owner, repo).await?;
        let protected_branches = self.client.protected_branches(owner, repo).await?;
        let protected = protected_branches
            .into_iter()
            .map(|branch| ProtectedBranch { name: branch.name })
            .collect::<Vec<ProtectedBranch>>();
        let protected = if protected.is_empty() {
            None
        } else {
            Some(protected)
        };
        Ok(GramSettings {
            description: repository.description,
            options: Some(Options {
                allow_squash_merge: Some(repository.allow_squash_merge),
                allow_merge_commit: Some(repository.allow_merge_commit),
                allow_rebase_merge: Some(repository.allow_rebase_merge),
                delete_branch_on_merge: Some(repository.delete_branch_on_merge),
            }),
            protected,
        })
    }
}

#[cfg(test)]
mod test {
    use super::{Retrieve, RetrieveSettings};
    use crate::{
        commands::settings::GramSettings,
        github::{Branch, GithubClient, Repository},
    };
    use anyhow::{anyhow, Result};
    use async_trait::async_trait;

    struct FakeClient {
        protected_branches: Option<Vec<Branch>>,
        repository: Option<Repository>,
    }

    #[async_trait]
    impl GithubClient for FakeClient {
        async fn repository(&self, _: &str, _: &str) -> Result<Repository> {
            match &self.repository {
                Some(repo) => Ok(repo.clone()),
                None => Err(anyhow!("error")),
            }
        }
        async fn protected_branches(&self, _: &str, _: &str) -> Result<Vec<Branch>> {
            match &self.protected_branches {
                Some(branches) => Ok(branches.clone()),
                None => Err(anyhow!("error")),
            }
        }
    }

    fn default_repository() -> Repository {
        Repository {
            description: Some("description".to_owned()),
            allow_squash_merge: true,
            allow_merge_commit: false,
            allow_rebase_merge: true,
            delete_branch_on_merge: false,
        }
    }

    fn default_protected_branches() -> Vec<Branch> {
        vec![Branch {
            name: "master".to_owned(),
        }]
    }

    async fn actual_settings(repo: Repository, branches: Vec<Branch>) -> Result<GramSettings> {
        let client = FakeClient {
            repository: Some(repo),
            protected_branches: Some(branches),
        };
        let retrieve = RetrieveSettings { client: &client };
        retrieve.retrieve("", "").await
    }

    #[tokio::test]
    async fn should_return_settings_with_no_protected_branches_if_there_are_none() {
        // arrange
        let repo = default_repository();

        // act
        let opt_settings = actual_settings(repo.clone(), Vec::new()).await;

        // assert
        assert!(opt_settings.is_ok());
        let settings = opt_settings.unwrap();
        assert!(settings.protected.is_none());
    }

    #[tokio::test]
    async fn should_return_settings_protected_branches() {
        // arrange
        let repo = default_repository();
        let branches = default_protected_branches();

        // act
        let opt_settings = actual_settings(repo.clone(), branches.clone()).await;

        // assert
        assert!(opt_settings.is_ok());
        let settings = opt_settings.unwrap();
        assert!(settings.protected.is_some());
        let protected_branches = settings.protected.unwrap();
        assert_eq!(branches[0].name, protected_branches[0].name);
    }

    #[tokio::test]
    async fn should_return_settings_description() {
        // arrange
        let repo = default_repository();
        let branches = default_protected_branches();

        // act
        let opt_settings = actual_settings(repo.clone(), branches.clone()).await;

        // assert
        assert!(opt_settings.is_ok());
        assert_eq!(repo.description, opt_settings.unwrap().description);
    }

    #[tokio::test]
    async fn should_return_error_if_protected_branch_fetch_fails() {
        // arrange
        let client = FakeClient {
            repository: Some(Repository::default()),
            protected_branches: None,
        };
        let retriever = RetrieveSettings { client: &client };

        // act
        let opt_settings = retriever.retrieve("", "").await;

        // assert
        assert!(opt_settings.is_err());
    }

    #[tokio::test]
    async fn should_return_settings() {
        // arrange
        let client = FakeClient {
            repository: Some(Repository::default()),
            protected_branches: Some(vec![Branch::default()]),
        };
        let retriever = RetrieveSettings { client: &client };

        // act
        let opt_settings = retriever.retrieve("", "").await;

        // assert
        assert!(opt_settings.is_ok());
    }

    #[tokio::test]
    async fn should_return_error_if_repo_fetch_fails() {
        // arrange
        let client = FakeClient {
            repository: None,
            protected_branches: Some(vec![Branch::default()]),
        };
        let retriever = RetrieveSettings { client: &client };

        // act
        let opt_settings = retriever.retrieve("", "").await;

        // assert
        assert!(opt_settings.is_err());
    }
}
