use async_trait::async_trait;
use serde::Deserialize;
use std::error::Error;

static GRAM_USER_AGENT: &str = "gram";
static GITHUB_BASE_URL: &str = "https://api.github.com";

#[derive(Debug, Deserialize)]
pub struct Repository {
    pub description: Option<String>,
}

#[async_trait]
pub trait GithubClient {
    async fn repository(&self, owner: &str, repo: &str) -> Result<Repository, Box<dyn Error>>;
}

pub struct Github {
    client: reqwest::Client,
}

impl Github {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .user_agent(GRAM_USER_AGENT)
            .build()
            .unwrap();
        Self { client }
    }
}

#[async_trait]
impl GithubClient for Github {
    async fn repository(&self, owner: &str, repo: &str) -> Result<Repository, Box<dyn Error>> {
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

