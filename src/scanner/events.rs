use crate::config::Config;
use crate::core::TokenRotator;
use crate::models::github::GitHubEvent;
use futures::StreamExt; // ✅ Sahi: futures crate se
use tokio_tungstenite::tungstenite::Message; // ✅ Sahi: re-exported path
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
                                warn!("WebSocket closed by server");
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

            info!("🔄 Falling back to polling for 60 seconds...");
            let start = std::time::Instant::now();
            while start.elapsed() < Duration::from_secs(60) {
                match self.fetch_events_polling().await {
                    Ok(events) => {
                        for event in events {
                            if event.event_type == "PushEvent" {
                                callback(event);
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Polling error: {}", e);
                    }
                }
                sleep(Duration::from_secs(self.config.poll_interval_secs)).await;
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
            info!("📡 Fetched {} events (polling)", events.len());
            Ok(events)
        } else if status == 403 {
            error!("GitHub API 403 Forbidden (rate limit)");
            Ok(vec![])
        } else {
            error!("GitHub API error: {}", status);
            Ok(vec![])
        }
    }
}