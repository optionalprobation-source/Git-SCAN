use crate::config::Config;
use crate::core::TokenRotator;
use crate::models::github::GitHubEvent;
use futures::StreamExt;
use tokio_tungstenite::tungstenite::Message;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn, error};

pub struct GitHubEventsPoller {
    config: Arc<Config>,
    token_rotator: Arc<TokenRotator>,
}

impl GitHubEventsPoller {
    pub fn new(config: Arc<Config>, token_rotator: Arc<TokenRotator>) -> Self {
        Self { config, token_rotator }
    }

    pub async fn run<F>(&self, mut callback: F)
    where
        F: FnMut(GitHubEvent) + Send + 'static,
    {
        loop {
            // 1. WebSocket try karo (agar chale to best)
            match self.connect_websocket().await {
                Ok(mut stream) => {
                    info!("🌐 WebSocket connected! Live events streaming...");
                    while let Some(msg) = stream.next().await {
                        match msg {
                            Ok(Message::Text(text)) => {
                                if let Ok(event) = serde_json::from_str::<GitHubEvent>(&text) {
                                    if event.event_type == "PushEvent" {
                                        callback(event);
                                    }
                                }
                            }
                            Ok(Message::Ping(_)) => {}
                            Ok(Message::Close(_)) => {
                                warn!("WebSocket closed");
                                break;
                            }
                            Err(e) => {
                                warn!("WebSocket error: {}", e);
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                Err(e) => {
                    warn!("WebSocket connection failed: {}", e);
                }
            }

            // 2. Ultra-fast polling (1 sec per loop)
            info!("🔄 Polling mode active...");
            let mut consecutive_errors = 0;
            loop {
                match self.fetch_events_polling().await {
                    Ok(events) => {
                        consecutive_errors = 0;
                        for event in events {
                            if event.event_type == "PushEvent" {
                                callback(event);
                            }
                        }
                    }
                    Err(e) => {
                        consecutive_errors += 1;
                        warn!("Polling error: {}", e);
                        if consecutive_errors >= 5 {
                            error!("Too many errors, waiting 30 seconds...");
                            sleep(Duration::from_secs(30)).await;
                            consecutive_errors = 0;
                        }
                    }
                }

                // Poll interval = max(1, config.poll_interval_secs)
                let interval = self.config.poll_interval_secs.max(1);
                sleep(Duration::from_secs(interval)).await;
            }
        }
    }

    async fn connect_websocket(&self) -> Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>
        >,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let url = "wss://ws.github.com/events";
        let (stream, _) = tokio_tungstenite::connect_async(url).await?;
        Ok(stream)
    }

    async fn fetch_events_polling(&self) -> Result<Vec<GitHubEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/events", self.config.github_api_url);
        let client = reqwest::Client::builder()
            .pool_max_idle_per_host(20)
            .tcp_keepalive(Duration::from_secs(60))
            .timeout(Duration::from_secs(30))
            .user_agent("git-scanner/1.0")
            .build()?;

        let mut request = client
            .get(&url)
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28");

        if let Some(token) = self.token_rotator.get_token() {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request.send().await?;
        let status = response.status();

        if status.is_success() {
            let events = response.json::<Vec<GitHubEvent>>().await?;
            info!("📡 Fetched {} events", events.len());
            Ok(events)
        } else if status == 403 {
            error!("GitHub API 403 Forbidden");
            Ok(vec![])
        } else {
            error!("GitHub API error: {}", status);
            Ok(vec![])
        }
    }
}