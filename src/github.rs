use anyhow::Result;
use async_trait::async_trait;
use reqwest::{
    header,
    header::{HeaderMap, HeaderValue},
    Client,
};
use serde::Deserialize;

static GRAM_USER_AGENT: &str = "gram";
static GITHUB_BASE_URL: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub description: Option<String>,
    pub allow_squash_merge: Option<bool>,
}

#[async_trait]
pub trait GithubClient {
    async fn repository(&self, owner: &str, repo: &str) -> Result<Repository>;
}

pub struct Github {
    client: Client,
}

impl Github {
    pub fn new(token: &str) -> Self {
        let client = Client::builder()
            .user_agent(GRAM_USER_AGENT)
            .default_headers(Github::default_headers(token))
            .build()
            .unwrap();
        Self { client }
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
}

#[async_trait]
impl GithubClient for Github {
    async fn repository(&self, owner: &str, repo: &str) -> Result<Repository> {
        let repository = self
            .client
            .get(&format!("{}/repos/{}/{}", GITHUB_BASE_URL, owner, repo))
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .send()
            .await?
            .json::<Repository>()
            .await?;
        Ok(repository)
    }
}
