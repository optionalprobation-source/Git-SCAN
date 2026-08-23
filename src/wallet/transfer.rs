use ethers::prelude::*;
use ethers::providers::{Http, Provider, Middleware};
use ethers::middleware::SignerMiddleware;
use std::sync::Arc;
use crate::models::wallet::TransferResult;
use tracing::{info, warn};

pub struct TransferExecutor {
    provider: Arc<Provider<Http>>,
    chain_id: u64,
}

impl TransferExecutor {
    pub fn new(rpc_url: &str, chain_id: u64) -> Result<Self, String> {
        match Provider::<Http>::try_from(rpc_url) {
            Ok(provider) => Ok(Self { provider: Arc::new(provider), chain_id }),
            Err(e) => Err(format!("RPC error: {}", e)),
        }
    }
    
    pub async fn transfer_native(
        &self,
        private_key: &str,
        to_address: &str,
        amount: &str,
        gas_limit: u64,
        gas_price_gwei: u64,
    ) -> Result<TransferResult, String> {
        let clean_key = private_key.trim_start_matches("0x");
        
        let wallet: LocalWallet = match clean_key.parse() {
            Ok(w) => w,
            Err(e) => return Err(format!("Key error: {}", e)),
        };
        let wallet = wallet.with_chain_id(self.chain_id);
        
        let from_address = format!("{:?}", wallet.address());
        let client = SignerMiddleware::new(self.provider.clone(), wallet.clone());
        
        let to: ethers::types::Address = match to_address.parse() {
            Ok(a) => a,
            Err(e) => return Err(format!("To address error: {}", e)),
        };
        
        let amount_wei = match ethers::utils::parse_ether(amount) {
            Ok(v) => v,
            Err(e) => return Err(format!("Amount error: {}", e)),
        };
        
        // LEGACY transaction - sab chains par kaam karta hai
        let tx = TransactionRequest::new()
            .to(to)
            .value(amount_wei)
            .gas(gas_limit)
            .gas_price(ethers::utils::parse_units(gas_price_gwei, "gwei").unwrap());
        
        info!("📤 Sending from {} to {}", from_address, to_address);
        
        match client.send_transaction(tx, None).await {
            Ok(pending) => {
                let tx_hash = format!("{:?}", pending.tx_hash());
                Ok(TransferResult {
                    from_address,
                    to_address: to_address.to_string(),
                    amount: amount.to_string(),
                    tx_hash,
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                warn!("Transfer failed: {}", e);
                Ok(TransferResult {
                    from_address,
                    to_address: to_address.to_string(),
                    amount: amount.to_string(),
                    tx_hash: String::new(),
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }
    
    pub fn calculate_max_amount(balance: &str, gas_price_gwei: u64, gas_limit: u64) -> String {
        let balance_eth: f64 = balance.parse().unwrap_or(0.0);
        let gas_cost = (gas_price_gwei as f64 * gas_limit as f64) / 1_000_000_000.0;
        let max = balance_eth - gas_cost;
        if max > 0.0 { format!("{:.6}", max) } else { "0".to_string() }
    }
}