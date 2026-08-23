use super::private_key::{PRIVATE_KEY_PATTERNS, extract_private_key};
use super::seed_phrase::{SEED_PHRASE_PATTERNS, extract_seed_phrase};
use crate::models::scan::{CryptoSecret, SecretType};
use std::collections::HashSet;

pub struct PatternMatcher;

impl PatternMatcher {
    pub fn new() -> Self {
        Self
    }
    
    // SNIPER MODE: No validation, just pattern match
    pub fn scan_content(&self, content: &str) -> Vec<CryptoSecret> {
        let mut secrets = Vec::with_capacity(4);
        
        // Scan private keys - NO FORMAT VALIDATION
        self.scan_private_keys(content, &mut secrets);
        
        // Scan seed phrases - NO WORD COUNT VALIDATION
        self.scan_seed_phrases(content, &mut secrets);
        
        // Remove duplicates
        self.deduplicate(secrets)
    }
    
    fn scan_private_keys(&self, content: &str, secrets: &mut Vec<CryptoSecret>) {
        for pattern in PRIVATE_KEY_PATTERNS.iter() {
            for captures in pattern.captures_iter(content) {
                if let Some(matched) = captures.get(0) {
                    let matched_text = matched.as_str();
                    
                    // SIRF extract karo, validation MAT KARO
                    if let Some(private_key) = extract_private_key(matched_text) {
                        secrets.push(CryptoSecret {
                            secret_type: SecretType::PrivateKey,
                            value: private_key,
                            raw_match: matched_text.to_string(),
                            line_number: None,
                        });
                    }
                }
            }
        }
    }
    
    fn scan_seed_phrases(&self, content: &str, secrets: &mut Vec<CryptoSecret>) {
        for pattern in SEED_PHRASE_PATTERNS.iter() {
            for captures in pattern.captures_iter(content) {
                if let Some(matched) = captures.get(0) {
                    let matched_text = matched.as_str();
                    
                    // SIRF extract karo, validation MAT KARO
                    if let Some(seed_phrase) = extract_seed_phrase(matched_text) {
                        secrets.push(CryptoSecret {
                            secret_type: SecretType::SeedPhrase,
                            value: seed_phrase,
                            raw_match: matched_text.to_string(),
                            line_number: None,
                        });
                    }
                }
            }
        }
    }
    
    fn deduplicate(&self, secrets: Vec<CryptoSecret>) -> Vec<CryptoSecret> {
        let mut seen = HashSet::with_capacity(secrets.len());
        secrets
            .into_iter()
            .filter(|s| {
                let key = format!("{:?}:{}", s.secret_type, s.value);
                seen.insert(key)
            })
            .collect()
    }
}