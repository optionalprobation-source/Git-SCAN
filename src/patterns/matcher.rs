use super::private_key::{PRIVATE_KEY_PATTERNS, extract_private_key, is_valid_format};
use super::seed_phrase::{SEED_PHRASE_PATTERNS, extract_seed_phrase, is_valid_word_count};
use crate::models::scan::{CryptoSecret, SecretType};
use std::collections::HashSet;

pub struct PatternMatcher;

impl PatternMatcher {
    pub fn new() -> Self {
        Self
    }
    
    // Fast scan for crypto secrets (private keys + seed phrases only)
    pub fn scan_content(&self, content: &str) -> Vec<CryptoSecret> {
        let mut secrets = Vec::with_capacity(4); // Pre-allocate
        
        // Scan for private keys
        self.scan_private_keys(content, &mut secrets);
        
        // Scan for seed phrases
        self.scan_seed_phrases(content, &mut secrets);
        
        // Remove duplicates
        self.deduplicate(secrets)
    }
    
    fn scan_private_keys(&self, content: &str, secrets: &mut Vec<CryptoSecret>) {
        for pattern in PRIVATE_KEY_PATTERNS.iter() {
            for captures in pattern.captures_iter(content) {
                if let Some(matched) = captures.get(0) {
                    let matched_text = matched.as_str();
                    
                    // Extract clean private key
                    if let Some(private_key) = extract_private_key(matched_text) {
                        // Validate format
                        if is_valid_format(&private_key) {
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
    }
    
    fn scan_seed_phrases(&self, content: &str, secrets: &mut Vec<CryptoSecret>) {
        for pattern in SEED_PHRASE_PATTERNS.iter() {
            for captures in pattern.captures_iter(content) {
                if let Some(matched) = captures.get(0) {
                    let matched_text = matched.as_str();
                    
                    // Extract clean seed phrase
                    if let Some(seed_phrase) = extract_seed_phrase(matched_text) {
                        // Validate word count
                        if is_valid_word_count(&seed_phrase) {
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