use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::{
    header,
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use structopt::clap::{crate_name, crate_version};

pub static GITHUB_BASE_URL: &str = "https://api.github.com";
static GRAM_USER_AGENT: &str = concat!(crate_name!(), " ", crate_version!());

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Repository {
    pub description: Option<String>,
    pub allow_squash_merge: bool,
    pub allow_merge_commit: bool,
    pub allow_rebase_merge: bool,
    pub delete_branch_on_merge: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Branch {
    pub name: String,
    pub protection: Protection,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Protection {
    pub enabled: bool,
    pub required_status_checks: RequiredStatusChecks,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RequiredStatusChecks {
    pub contexts: Vec<String>,
}

pub struct Github<'a> {
    base_url: &'a str,
    client: Client,
}

impl<'a> Github<'a> {
    pub fn new(token: String, base_url: &'a str) -> Self {
        let client = Client::builder()
            .user_agent(GRAM_USER_AGENT)
            .default_headers(Github::default_headers(&token))
            .build()
            .unwrap();
        Self { base_url, client }
    }

    fn default_headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/json"),
        );
        headers.insert(
            header::AUTHORIZATION,
            HeaderValue::from_bytes(format!("token {}", token).as_bytes())
                .expect("failed building authorization header from token"),
        );
        headers
    }

    async fn get<T>(&self, url: &str) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let response = self
            .client
            .get(&format!("{}{}", self.base_url, url))
            .send()
            .await?;
        match response.status() {
            reqwest::StatusCode::UNAUTHORIZED => {
                let msg = format!(
                    "Encountered a http status of 401 when calling GET on url {}. Is your token correct?",
                    url
                );
                Err(anyhow!("{}", msg))
            }
            _ => {
                let r = response.error_for_status().map_err(|e| anyhow!("{}", e))?;
                Ok(r.json::<T>().await?)
            }
        }
    }
}

#[async_trait]
pub trait GithubClient {
    async fn repository(&self, owner: &str, name: &str) -> Result<Repository>;
    async fn protected_branches(&self, owner: &str, name: &str) -> Result<Vec<Branch>>;
}

#[async_trait]
impl GithubClient for Github<'_> {
    async fn repository(&self, owner: &str, name: &str) -> Result<Repository> {
        self.get::<Repository>(&format!("/repos/{}/{}", owner, name))
            .await
    }
    async fn protected_branches(&self, owner: &str, name: &str) -> Result<Vec<Branch>> {
        self.get::<Vec<Branch>>(&format!(
            "/repos/{}/{}/branches?protected=true",
            owner, name
        ))
        .await
    }
}

#[cfg(test)]
mod test {
    use super::{Branch, Github, Repository};
    use mockito::mock;
    use serde_json;
    use serde_json::json;

    #[tokio::test]
    async fn get_should_error_with_message_if_call_returns_401() {
        // arrange
        let _m = mock("GET", "/repos/owner/repo")
            .with_status(401)
            .with_header("content-type", "application")
            .create();
        let url = mockito::server_url();
        let github = Github::new("token".to_owned(), &url);

        // act
        let response = github.get::<Repository>("/repos/owner/repo").await;

        // assert
        assert!(
            response.is_err(),
            "expected response to be an error, got {:#?}",
            response
        );
        assert_eq!(
            "Encountered a http status of 401 when calling GET on url /repos/owner/repo. Is your token correct?",
            &format!("{}", response.err().unwrap())
        )
    }

    #[test]
    fn repository_should_deserialize_correctly() {
        // arrange
        let description: Option<String> = None;
        let allow_merge_commit = true;
        let allow_squash_merge = false;
        let allow_rebase_merge = true;
        let delete_branch_on_merge = true;
        let json = json!({
           "description": &description,
           "allow_merge_commit": &allow_merge_commit,
           "allow_squash_merge": &allow_squash_merge,
           "allow_rebase_merge": &allow_rebase_merge,
           "delete_branch_on_merge": &delete_branch_on_merge
        });

        // act
        let repo = serde_json::from_str::<Repository>(&json.to_string()).unwrap();

        // assert
        assert_eq!(description, repo.description);
        assert_eq!(allow_merge_commit, repo.allow_merge_commit);
        assert_eq!(allow_squash_merge, repo.allow_squash_merge);
        assert_eq!(allow_rebase_merge, repo.allow_rebase_merge);
        assert_eq!(delete_branch_on_merge, repo.delete_branch_on_merge);
    }

    #[test]
    fn branch_should_deserialize_correctly() {
        // arrange
        let branch_name = "master".to_owned();
        let json = json!([
            {
                "name": &branch_name,
                "protection": {
                    "enabled": true,
                    "required_status_checks": {
                        "contexts": ["master"]
                    }
                }
            }
        ]);

        // act
        let branch = dbg!(serde_json::from_str::<Vec<Branch>>(&json.to_string())).unwrap();

        // assert
        assert_eq!(branch_name, branch[0].name);
        assert_eq!(true, branch[0].protection.enabled);
        assert!(branch[0]
            .protection
            .required_status_checks
            .contexts
            .contains(&"master".to_string()));
    }
}
