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
        
        // SIMD-optimized hex validation (when available)
        #[cfg(target_arch = "x86_64")]
        {
            return validate_hex_simd(clean_key.as_bytes());
        }
        
        #[cfg(not(target_arch = "x86_64"))]
        {
            clean_key.as_bytes().iter().all(|&b| b.is_ascii_hexdigit())
        }
    }
    
    // Optimized cryptographic validation
    #[inline(always)]
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
        
        let curve_order: [u8; 32] = [
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
            0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFE,
            0xBA, 0xAE, 0xDC, 0xE6, 0xAF, 0x48, 0xA0, 0x3B,
            0xBF, 0xD2, 0x5E, 0x8C, 0xD0, 0x36, 0x41, 0x41,
        ];
        
        key_bytes.as_slice() < &curve_order
    }
}

// SIMD-accelerated hex validation for x86_64
#[cfg(target_arch = "x86_64")]
#[inline(always)]
fn validate_hex_simd(bytes: &[u8]) -> bool {
    use std::arch::x86_64::*;
    
    if bytes.len() != 64 {
        return false;
    }
    
    unsafe {
        // Load 32 bytes at a time
        let chunk1 = _mm256_loadu_si256(bytes.as_ptr() as *const __m256i);
        let chunk2 = _mm256_loadu_si256(bytes.as_ptr().add(32) as *const __m256i);
        
        // Create comparison masks
        let zero = _mm256_set1_epi8(0);
        let nine = _mm256_set1_epi8(9);
        let a_upper = _mm256_set1_epi8(b'A' as i8);
        let f_upper = _mm256_set1_epi8(b'F' as i8);
        let a_lower = _mm256_set1_epi8(b'a' as i8);
        let f_lower = _mm256_set1_epi8(b'f' as i8);
        
        // Check chunk 1
        let is_digit1 = _mm256_and_si256(
            _mm256_cmpgt_epi8(chunk1, zero),
            _mm256_cmplt_epi8(chunk1, nine)
        );
        let is_upper1 = _mm256_and_si256(
            _mm256_cmpgt_epi8(chunk1, a_upper),
            _mm256_cmplt_epi8(chunk1, f_upper)
        );
        let is_lower1 = _mm256_and_si256(
            _mm256_cmpgt_epi8(chunk1, a_lower),
            _mm256_cmplt_epi8(chunk1, f_lower)
        );
        let is_hex1 = _mm256_or_si256(
            _mm256_or_si256(is_digit1, is_upper1),
            is_lower1
        );
        let mask1 = _mm256_movemask_epi8(is_hex1);
        
        // Check chunk 2
        let is_digit2 = _mm256_and_si256(
            _mm256_cmpgt_epi8(chunk2, zero),
            _mm256_cmplt_epi8(chunk2, nine)
        );
        let is_upper2 = _mm256_and_si256(
            _mm256_cmpgt_epi8(chunk2, a_upper),
            _mm256_cmplt_epi8(chunk2, f_upper)
        );
        let is_lower2 = _mm256_and_si256(
            _mm256_cmpgt_epi8(chunk2, a_lower),
            _mm256_cmplt_epi8(chunk2, f_lower)
        );
        let is_hex2 = _mm256_or_si256(
            _mm256_or_si256(is_digit2, is_upper2),
            is_lower2
        );
        let mask2 = _mm256_movemask_epi8(is_hex2);
        
        // All bits must be set (all valid hex)
        mask1 == !0 && mask2 == !0
    }
}

// Fallback for non-x86_64 architectures
#[cfg(not(target_arch = "x86_64"))]
#[inline(always)]
fn validate_hex_simd(bytes: &[u8]) -> bool {
    bytes.iter().all(|&b| b.is_ascii_hexdigit())
}