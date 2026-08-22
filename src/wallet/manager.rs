use crate::config::Config;
use crate::models::wallet::{TransferResult, WalletInfo};
use crate::wallet::address::AddressDeriver;
use crate::wallet::balance::BalanceChecker;
use crate::wallet::transfer::TransferExecutor;
use std::sync::Arc;
use tracing::info;

pub struct WalletManager {
    config: Arc<Config>,
    address_deriver: AddressDeriver,
    balance_checker: BalanceChecker,
    transfer_executor: TransferExecutor,
}

impl WalletManager {
    pub fn new(config: Arc<Config>) -> Result<Self, String> {
        Ok(Self {
            address_deriver: AddressDeriver::new(),
            balance_checker: BalanceChecker::new(&config.rpc_url)?,
            transfer_executor: TransferExecutor::new(&config.rpc_url, config.chain_id)?,
            config,
        })
    }
    
    // NEW: Process multiple keys simultaneously
    pub async fn process_keys_batch(
        &self,
        private_keys: Vec<String>,
    ) -> Vec<(WalletInfo, Option<TransferResult>)> {
        let mut results = Vec::new();
        let mut valid_addresses = Vec::new();
        let mut addr_to_key = std::collections::HashMap::new();

        // 1. Fast derive all addresses (CPU Bound)
        for key in private_keys {
            if let Some(address) = self.address_deriver.derive_address(&key) {
                valid_addresses.push(address.clone());
                addr_to_key.insert(address, key);
            }
        }

        // 2. Fast concurrent balance check (Network Bound)
        let balances = self.balance_checker.check_balances_batch(&valid_addresses).await;

        // 3. Process sweeps for positive balances
        for (address, balance_result) in balances {
            if let Ok(balance) = balance_result {
                let key = addr_to_key.get(&address).unwrap();
                let balance_float: f64 = balance.parse().unwrap_or(0.0);
                
                let mut wallet_info = WalletInfo {
                    address: address.clone(),
                    private_key: key.clone(),
                    balance: balance.clone(),
                    network: "configured_chain".to_string(),
                };

                let mut transfer_res = None;

                if balance_float > self.config.min_balance_threshold {
                    let max_amount = TransferExecutor::calculate_max_amount(
                        &balance,
                        self.config.gas_price_gwei,
                        self.config.gas_limit,
                    );
                    
                    if max_amount != "0" {
                        info!("📤 Sweeping {} from {}", max_amount, address);
                        if let Ok(res) = self.transfer_executor.transfer_native(
                            key, &self.config.recipient_address, &max_amount,
                            self.config.gas_limit, self.config.gas_price_gwei
                        ).await {
                            transfer_res = Some(res);
                        }
                    }
                }
                results.push((wallet_info, transfer_res));
            }
        }
        results
    }
}
