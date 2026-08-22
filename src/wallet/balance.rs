use ethers::providers::{Http, Provider, Middleware};
use ethers::types::Address;
use std::sync::Arc;
use futures::future::join_all;
use tracing::{info, warn};

pub struct BalanceChecker {
    provider: Arc<Provider<Http>>,
}

impl BalanceChecker {
    pub fn new(rpc_url: &str) -> Result<Self, String> {
        match Provider::<Http>::try_from(rpc_url) {
            Ok(provider) => Ok(Self { provider: Arc::new(provider) }),
            Err(e) => Err(format!("RPC error: {}", e)),
        }
    }
    
    // Check single balance
    pub async fn get_balance(&self, address: &str) -> Result<String, String> {
        let addr = match address.parse::<Address>() {
            Ok(a) => a,
            Err(e) => return Err(format!("Address error: {}", e)),
        };
        
        match self.provider.get_balance(addr, None).await {
            Ok(balance) => Ok(ethers::utils::format_ether(balance)),
            Err(e) => Err(format!("Balance error: {}", e)),
        }
    }

    // Concurrent Batch Checking (Unlimited)
    pub async fn check_balances_batch(
        &self,
        addresses: &[String],
    ) -> Vec<(String, Result<String, String>)> {
        if addresses.is_empty() {
            return vec![];
        }
        
        info!("🔍 Checking {} addresses in batch", addresses.len());
        
        let futures = addresses.iter().map(|addr_str| {
            let provider = self.provider.clone();
            let addr_str = addr_str.clone();
            
            async move {
                let addr = match addr_str.parse::<Address>() {
                    Ok(a) => a,
                    Err(e) => return (addr_str, Err(format!("Address error: {}", e))),
                };
                
                match provider.get_balance(addr, None).await {
                    Ok(balance) => (addr_str, Ok(ethers::utils::format_ether(balance))),
                    Err(e) => (addr_str, Err(format!("Balance error: {}", e))),
                }
            }
        });
        
        join_all(futures).await
    }
    
    // Batch with concurrency limit (Rate limit protection)
    pub async fn check_balances_batch_limited(
        &self,
        addresses: &[String],
        concurrency: usize,
    ) -> Vec<(String, Result<String, String>)> {
        use futures::stream::{self, StreamExt};
        
        if addresses.is_empty() {
            return vec![];
        }
        
        let results = stream::iter(addresses.iter().cloned())
            .map(|addr| {
                let provider = self.provider.clone();
                async move {
                    let addr_parse = addr.parse::<Address>();
                    match addr_parse {
                        Ok(a) => {
                            match provider.get_balance(a, None).await {
                                Ok(balance) => (addr, Ok(ethers::utils::format_ether(balance))),
                                Err(e) => (addr, Err(format!("Balance error: {}", e))),
                            }
                        }
                        Err(e) => (addr, Err(format!("Address error: {}", e))),
                    }
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;
        
        results
    }
}