use ethers::providers::{Http, Provider, Middleware};
use ethers::types::Address;
use std::sync::Arc;
use futures::future::join_all;
use tracing::{info, warn};

pub struct BalanceChecker {
    providers: Vec<Arc<Provider<Http>>>,
}

impl BalanceChecker {
    pub fn new(rpc_url: &str) -> Result<Self, String> {
        // Multiple RPC endpoints - race karo
        let rpc_urls = vec![
            rpc_url.to_string(),
            "https://eth.llamarpc.com".to_string(),
            "https://rpc.ankr.com/eth".to_string(),
            "https://cloudflare-eth.com".to_string(),
        ];
        
        let mut providers = Vec::new();
        
        for url in rpc_urls {
            if let Ok(provider) = Provider::<Http>::try_from(url.as_str()) {
                providers.push(Arc::new(provider));
            }
        }
        
        if providers.is_empty() {
            return Err("No RPC provider available".to_string());
        }
        
        Ok(Self { providers })
    }
    
    // Fastest provider se balance check karo
    pub async fn get_balance(&self, address: &str) -> Result<String, String> {
        let addr = match address.parse::<Address>() {
            Ok(a) => a,
            Err(e) => return Err(format!("Address error: {}", e)),
        };
        
        // Race karo - jo sabse pehle response de
        let futures: Vec<_> = self.providers.iter().map(|provider| {
            let provider = provider.clone();
            async move {
                match provider.get_balance(addr, None).await {
                    Ok(balance) => Ok(ethers::utils::format_ether(balance)),
                    Err(e) => Err(format!("Balance error: {}", e)),
                }
            }
        }).collect();
        
        // Sabse pehle complete hone wala future
        for future in futures {
            if let Ok(balance) = future.await {
                return Ok(balance);
            }
        }
        
        Err("All RPC providers failed".to_string())
    }

    // Concurrent Batch Checking
    pub async fn check_balances_batch(
        &self,
        addresses: &[String],
    ) -> Vec<(String, Result<String, String>)> {
        if addresses.is_empty() {
            return vec![];
        }
        
        info!("🔍 Checking {} addresses in batch", addresses.len());
        
        let futures = addresses.iter().map(|addr_str| {
            let self_clone = self;
            let addr_str = addr_str.clone();
            
            async move {
                let result = self_clone.get_balance(&addr_str).await;
                (addr_str, result)
            }
        });
        
        join_all(futures).await
    }
    
    // Batch with concurrency limit
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
                let self_clone = self;
                async move {
                    let result = self_clone.get_balance(&addr).await;
                    (addr, result)
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<_>>()
            .await;
        
        results
    }
}