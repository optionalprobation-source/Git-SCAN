use once_cell::sync::Lazy;
use regex::Regex;

// Pre-compiled regex patterns for private keys
pub static PRIVATE_KEY_PATTERNS: Lazy<Vec<Regex>> = Lazy::new(|| {
    vec![
        // Pattern 1: PRIVATE_KEY=0x... (with variable name)
        Regex::new(r#"(?i)(private[_\-]?key|secret[_\-]?key|wallet[_\-]?key|pk)\s*[=:]\s*['"]?(0x[a-fA-F0-9]{64})['"]?"#).unwrap(),
        
        // Pattern 2: privateKeys: ["0x..."] (Hardhat style)
        Regex::new(r#"(?i)privateKeys\s*:\s*\[\s*['"](0x[a-fA-F0-9]{64})['"]"#).unwrap(),
        
        // Pattern 3: Raw 0x + 64 hex chars (anywhere in text)
        Regex::new(r"0x[a-fA-F0-9]{64}").unwrap(),
        
        // Pattern 4: 64 hex chars without 0x (with key-like variable)
        Regex::new(r#"(?i)(private[_\-]?key|secret[_\-]?key|wallet[_\-]?key)\s*[=:]\s*['"]?([a-fA-F0-9]{64})['"]?"#).unwrap(),
    ]
});

// Extract clean private key from matched text
pub fn extract_private_key(text: &str) -> Option<String> {
    // Try to find 0x + 64 hex first
    let hex_pattern = Regex::new(r"0x[a-fA-F0-9]{64}").unwrap();
    if let Some(m) = hex_pattern.find(text) {
        return Some(m.as_str().to_string());
    }
    
    // Try to find 64 hex without 0x
    let bare_hex = Regex::new(r"(?<![a-fA-F0-9])([a-fA-F0-9]{64})(?![a-fA-F0-9])").unwrap();
    if let Some(m) = bare_hex.find(text) {
        return Some(format!("0x{}", m.as_str()));
    }
    
    None
}

// Quick validation - is this a valid private key format?
pub fn is_valid_format(key: &str) -> bool {
    let key = key.trim_start_matches("0x");
    
    if key.len() != 64 {
        return false;
    }
    
    // Byte-level check - faster than chars()
    key.as_bytes().iter().all(|&b| b.is_ascii_hexdigit())
}