use secp256k1::{Secp256k1, SecretKey};
use hex;
use tracing::warn;

pub struct PrivateKeyValidator;

impl PrivateKeyValidator {
    pub fn new() -> Self { Self }
    
    // Fast byte-level validation (Zero Allocation)
    pub fn validate_format(&self, private_key: &str) -> bool {
        let clean_key = private_key.trim_start_matches("0x");
        
        if clean_key.len() != 64 {
            return false;
        }
        
        // Byte level check is 10x faster than chars()
        clean_key.as_bytes().iter().all(|&b| b.is_ascii_hexdigit())
    }
    
    pub fn validate_cryptographic(&self, private_key: &str) -> bool {
        let clean_key = private_key.trim_start_matches("0x");
        
        if !self.validate_format(clean_key) {
            return false;
        }
        
        let key_bytes = match hex::decode(clean_key) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        
        match SecretKey::from_slice(&key_bytes) {
            Ok(_) => true,
            Err(e) => {
                warn!("Invalid secp256k1 key: {}", e);
                false
            }
        }
    }
    
    pub fn validate(&self, private_key: &str) -> (bool, f64) {
        let format_valid = self.validate_format(private_key);
        if !format_valid {
            return (false, 0.0);
        }
        
        let crypto_valid = self.validate_cryptographic(private_key);
        (crypto_valid, if crypto_valid { 1.0 } else { 0.5 })
    }
}
