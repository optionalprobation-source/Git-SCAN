use crate::config::Config;
use crate::core::TokenRotator;
use crate::models::github::{CommitDetail, GitHubEvent};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use futures::stream::{self, StreamExt};
use tracing::{info, warn};

#[derive(Clone)]
pub struct CommitFetcher {
    client: Client,
    config: Arc<Config>,
    token_rotator: Arc<TokenRotator>,
}

#[derive(Debug, Clone)]
pub struct CommitWithRepo {
    pub commit: CommitDetail,
    pub repo_name: String,
}

impl CommitFetcher {
    pub fn new(config: Arc<Config>, token_rotator: Arc<TokenRotator>) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(10)
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(15))
            .user_agent("git-scanner/1.0")
            .build()
            .unwrap();

        Self { client, config, token_rotator }
    }

    pub async fn fetch_commits(&self, event: &GitHubEvent) -> Vec<CommitWithRepo> {
        let repo_name = event.repo.name.clone();

        let commits = match &event.payload.commits {
            Some(commits) if !commits.is_empty() => commits.clone(),
            _ => {
                if let Some(head_sha) = &event.payload.head {
                    vec![crate::models::github::Commit {
                        sha: head_sha.clone(),
                        author: None,
                        message: None,
                        distinct: None,
                        url: None,
                    }]
                } else {
                    return vec![];
                }
            }
        };

        let results = stream::iter(commits)
            .map(|commit| {
                let client = self.client.clone();
                let config = self.config.clone();
                let token_rotator = self.token_rotator.clone();
                let repo_name = repo_name.clone();
                let sha = commit.sha.clone();

                async move {
                    fetch_single_commit(client, config, token_rotator, &repo_name, &sha).await
                }
            })
            .buffer_unordered(5)
            .collect::<Vec<_>>()
            .await;

        let mut commit_details: Vec<CommitWithRepo> = Vec::new();

        for result in results {
            match result {
                Ok(Some(commit)) => {
                    commit_details.push(CommitWithRepo {
                        commit,
                        repo_name: repo_name.clone(),
                    });
                }
                Ok(None) => {}
                Err(e) => {
                    warn!("Commit fetch error: {}", e);
                }
            }
        }

        commit_details
    }
}

async fn fetch_single_commit(
    client: Client,
    config: Arc<Config>,
    token_rotator: Arc<TokenRotator>,
    repo_name: &str,
    sha: &str,
) -> Result<Option<CommitDetail>, reqwest::Error> {
    let url = format!(
        "{}/repos/{}/commits/{}",
        config.github_api_url, repo_name, sha
    );

    let mut request = client
        .get(&url)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28");

    if let Some(token) = token_rotator.get_token() {
        request = request.header("Authorization", format!("Bearer {}", token));
    }

    let response = request.send().await?;
    let status = response.status();

    if status.is_success() {
        let commit = response.json::<CommitDetail>().await?;
        Ok(Some(commit))
    } else {
        let body = response.text().await?;
        warn!("❌ API error: {} - {}", status, body);
        Ok(None)
    }
}