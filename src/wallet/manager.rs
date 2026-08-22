use crate::config::Config;
use crate::models::wallet::{TransferResult, WalletInfo};
use crate::wallet::address::AddressDeriver;
use crate::wallet::balance::BalanceChecker;
use crate::wallet::transfer::TransferExecutor;
use std::sync::Arc;
use std::collections::HashMap;
use tracing::{info, warn};
use rayon::prelude::*;

pub struct WalletManager {
    config: Arc<Config>,
    address_deriver: AddressDeriver,
    balance_checker: BalanceChecker,
    transfer_executor: TransferExecutor,
}

impl WalletManager {
    pub fn new(config: Arc<Config>) -> Result<Self, String> {
        let address_deriver = AddressDeriver::new();
        let balance_checker = BalanceChecker::new(&config.rpc_url)?;
        let transfer_executor = TransferExecutor::new(&config.rpc_url, config.chain_id)?;
        
        Ok(Self {
            config,
            address_deriver,
            balance_checker,
            transfer_executor,
        })
    }
    
    // Single key processing
    pub async fn process_private_key(
        &self,
        private_key: &str,
    ) -> Result<(WalletInfo, Option<TransferResult>), String> {
        let address = match self.address_deriver.derive_address(private_key) {
            Some(a) => a,
            None => return Err("Failed to derive address".to_string()),
        };
        
        info!("👛 Derived address: {}", address);
        
        let balance = self.balance_checker.get_balance(&address).await?;
        
        let wallet_info = WalletInfo {
            address: address.clone(),
            private_key: private_key.to_string(),
            balance: balance.clone(),
            network: "bsc".to_string(),
        };
        
        let balance_float: f64 = balance.parse().unwrap_or(0.0);
        
        if balance_float > self.config.min_balance_threshold {
            let max_amount = TransferExecutor::calculate_max_amount(
                &balance,
                self.config.gas_price_gwei,
                self.config.gas_limit,
            );
            
            if max_amount == "0" || max_amount.starts_with('-') {
                return Ok((wallet_info, None));
            }
            
            info!("📤 Transferring {} to {}", max_amount, self.config.recipient_address);
            
            let transfer_result = self.transfer_executor.transfer_native(
                private_key,
                &self.config.recipient_address,
                &max_amount,
                self.config.gas_limit,
                self.config.gas_price_gwei,
            ).await?;
            
            Ok((wallet_info, Some(transfer_result)))
        } else {
            Ok((wallet_info, None))
        }
    }
    
    // NEW: Batch processing with parallel address derivation + concurrent balance check
    pub async fn process_keys_batch(
        &self,
        private_keys: Vec<String>,
    ) -> Vec<(WalletInfo, Option<TransferResult>)> {
        if private_keys.is_empty() {
            return vec![];
        }
        
        info!("🔄 Processing {} private keys in batch", private_keys.len());
        
        // 1. Parallel address derivation (CPU-bound -> Rayon)
        let derived: Vec<(String, String)> = private_keys
            .par_iter()
            .filter_map(|key| {
                self.address_deriver
                    .derive_address(key)
                    .map(|addr| (key.clone(), addr))
            })
            .collect();
        
        if derived.is_empty() {
            warn!("⚠️ No valid addresses derived");
            return vec![];
        }
        
        // 2. Build lookup map
        let mut addr_to_key = HashMap::with_capacity(derived.len());
        let addresses: Vec<String> = derived
            .iter()
            .map(|(key, addr)| {
                addr_to_key.insert(addr.clone(), key.clone());
                addr.clone()
            })
            .collect();
        
        // 3. Concurrent balance check with rate limiting
        let concurrency_limit = std::cmp::min(addresses.len(), 10);
        let balances = self.balance_checker
            .check_balances_batch_limited(&addresses, concurrency_limit)
            .await;
        
        // 4. Process sweeps
        let mut results = Vec::with_capacity(balances.len());
        
        for (address, balance_result) in balances {
            if let Ok(balance) = balance_result {
                if let Some(key) = addr_to_key.get(&address) {
                    let balance_float: f64 = balance.parse().unwrap_or(0.0);
                    
                    let wallet_info = WalletInfo {
                        address: address.clone(),
                        private_key: key.clone(),
                        balance: balance.clone(),
                        network: "bsc".to_string(),
                    };
                    
                    let mut transfer_res = None;
                    
                    if balance_float > self.config.min_balance_threshold {
                        if !self.config.recipient_address.is_empty() {
                            let max_amount = TransferExecutor::calculate_max_amount(
                                &balance,
                                self.config.gas_price_gwei,
                                self.config.gas_limit,
                            );
                            
                            if max_amount != "0" && !max_amount.starts_with('-') {
                                info!("📤 Sweeping {} from {}", max_amount, address);
                                
                                match self.transfer_executor.transfer_native(
                                    key,
                                    &self.config.recipient_address,
                                    &max_amount,
                                    self.config.gas_limit,
                                    self.config.gas_price_gwei,
                                ).await {
                                    Ok(res) => transfer_res = Some(res),
                                    Err(e) => warn!("Transfer error: {}", e),
                                }
                            }
                        }
                    }
                    
                    results.push((wallet_info, transfer_res));
                }
            }
        }
        
        info!("✅ Batch processing complete - {} results", results.len());
        results
    }
}