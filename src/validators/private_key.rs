use secp256k1::SecretKey;
use tracing::warn;

pub struct PrivateKeyValidator;

impl PrivateKeyValidator {
    pub fn new() -> Self {
        Self
    }
    
    // Fast byte-level validation (Zero Allocation)
    #[inline(always)]
    pub fn validate_format(&self, private_key: &str) -> bool {
        let clean_key = private_key.trim_start_matches("0x");
        
        if clean_key.len() != 64 {
            return false;
        }
        
        // Byte level check is 10x faster than chars()
        clean_key.as_bytes().iter().all(|&b| b.is_ascii_hexdigit())
    }
    
    // Optimized cryptographic validation
    #[inline(always)]
    pub fn validate_cryptographic(&self, private_key: &str) -> bool {
        let clean_key = private_key.trim_start_matches("0x");
        
        if !self.validate_format(clean_key) {
            return false;
        }
        
        // Fast hex decode
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
    
    // Combined validation with confidence score
    #[inline(always)]
    pub fn validate(&self, private_key: &str) -> (bool, f64) {
        let format_valid = self.validate_format(private_key);
        
        if !format_valid {
            return (false, 0.0);
        }
        
        let crypto_valid = self.validate_cryptographic(private_key);
        (crypto_valid, if crypto_valid { 1.0 } else { 0.5 })
    }
    
    // Check if key is within valid secp256k1 range
    pub fn is_in_valid_range(&self, private_key: &str) -> bool {
        let clean_key = private_key.trim_start_matches("0x");
        
        let key_bytes = match hex::decode(clean_key) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };
        
        // secp256k1 curve order (n)
        let curve_order: [u8; 32] = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE,
            0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B,
            0xBF, 0xD2, 0x5E, 0x8C, 0xD0, 0x36, 0x41, 0x41,
        ];
        
        // Rust native optimized slice comparison
        key_bytes.as_slice() < &curve_order
    }
}