use crate::config::Config;
use crate::models::github::GitHubEvent;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};
use futures::stream::StreamExt;

pub struct GitHubEventsPoller {
    client: Client,
    config: Arc<Config>,
}

impl GitHubEventsPoller {
    pub fn new(config: Arc<Config>) -> Self {
        let client = Client::builder()
            .pool_max_idle_per_host(20)
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(30))
            .user_agent("git-scanner/1.0")
            .build()
            .unwrap();
        
        Self { client, config }
    }
    
    // Poll GitHub Events API continuously
    pub async fn poll<F>(&self, mut callback: F)
    where
        F: FnMut(GitHubEvent) + Send + 'static,
    {
        loop {
            match self.fetch_events().await {
                Ok(events) => {
                    for event in events {
                        if event.event_type == "PushEvent" {
                            callback(event);
                        }
                    }
                }
                Err(e) => {
                    warn!("Failed to fetch events: {}", e);
                }
            }
            
            sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
        }
    }
    
    // Fetch events from GitHub API
    async fn fetch_events(&self) -> Result<Vec<GitHubEvent>, reqwest::Error> {
        let url = format!("{}/events", self.config.github_api_url);
        
        let request = self.client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");
        
        let request = if let Some(token) = &self.config.github_token {
            request.header("Authorization", format!("Bearer {}", token))
        } else {
            request
        };
        
        let response = request.send().await?;
        
        if response.status().is_success() {
            let events = response.json::<Vec<GitHubEvent>>().await?;
            info!("📡 Fetched {} events", events.len());
            Ok(events)
        } else {
            error!("GitHub API error: {}", response.status());
            Ok(vec![])
        }
    }
}