use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::{
    header,
    header::{HeaderMap, HeaderValue},
    Client, Response,
};
use serde::Deserialize;
use structopt::clap::{crate_name, crate_version};

static GRAM_USER_AGENT: &str = concat!(crate_name!(), " ", crate_version!());
pub static GITHUB_BASE_URL: &str = "https://api.github.com";

#[derive(Clone, Debug, Deserialize)]
pub struct Repository {
    pub description: Option<String>,
    pub allow_squash_merge: Option<bool>,
    pub allow_merge_commit: Option<bool>,
    pub allow_rebase_merge: Option<bool>,
}

#[async_trait]
pub trait GithubClient {
    async fn get(&self, url: &str) -> Result<Response>;
}

pub struct Github {
    base_url: &'static str,
    client: Client,
}

impl Github {
    pub fn new(token: &str, base_url: &'static str) -> Self {
        let client = Client::builder()
            .user_agent(GRAM_USER_AGENT)
            .default_headers(Github::default_headers(token))
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
}

#[async_trait]
impl GithubClient for Github {
    async fn get(&self, url: &str) -> Result<Response> {
        self.client
            .get(&format!("{}{}", self.base_url, url))
            .send()
            .await?
            .error_for_status()
            .map_err(|e| anyhow!("{}", e))
    }
}

#[cfg(test)]
mod test {
    use super::{Github, GithubClient};
    use mockito::mock;
    use tokio;

    #[tokio::test]
    async fn test_something() {
        let _m = mock("GET", "/repos/owner/repo")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{ "description": "test" } "#)
            .create();
        let client = Github::new("test", "base");
        let repo = client.get("").await;
        println!("{:#?}", repo);
    }
}
