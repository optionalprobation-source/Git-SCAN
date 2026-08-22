use ethers::providers::{Http, Provider, Middleware};
use ethers::types::Address;
use std::sync::Arc;
use futures::future::join_all;

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

    // NEW: Highly Optimized Concurrent Batch Checking
    pub async fn check_balances_batch(&self, addresses: &[String]) -> Vec<(String, Result<String, String>)> {
        let futures = addresses.iter().map(|addr_str| async move {
            let result = self.get_balance(addr_str).await;
            (addr_str.clone(), result)
        });
        
        // Execute all RPC calls simultaneously
        join_all(futures).await
    }
}
