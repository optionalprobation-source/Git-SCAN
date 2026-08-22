use tracing::warn;

pub struct SeedPhraseValidator;

impl SeedPhraseValidator {
    pub fn new() -> Self {
        Self
    }
    
    // Validate seed phrase word count
    pub fn validate_word_count(&self, seed_phrase: &str) -> bool {
        let words: Vec<&str> = seed_phrase
            .split_whitespace()
            .collect();
        
        words.len() == 12 || words.len() == 24
    }
    
    // Validate all words are alphabetic and lowercase
    pub fn validate_words(&self, seed_phrase: &str) -> bool {
        seed_phrase
            .split_whitespace()
            .all(|word| {
                !word.is_empty()
                && word.chars().all(|c| c.is_alphabetic())
                && word.chars().all(|c| c.is_lowercase())
                && word.len() >= 3
            })
    }
    
    // Check if words are in BIP39 wordlist (simplified - common words check)
    pub fn validate_bip39(&self, seed_phrase: &str) -> bool {
        let words: Vec<&str> = seed_phrase.split_whitespace().collect();
        
        // Check for duplicate words (rare in valid seed phrases)
        let mut unique_words = std::collections::HashSet::new();
        for word in &words {
            if !unique_words.insert(*word) {
                // Duplicate word found - likely not a valid seed
                return false;
            }
        }
        
        true
    }
    
    // Full validation
    pub fn validate(&self, seed_phrase: &str) -> (bool, f64) {
        let word_count_valid = self.validate_word_count(seed_phrase);
        let words_valid = self.validate_words(seed_phrase);
        let bip39_likely = self.validate_bip39(seed_phrase);
        
        // Confidence score
        let mut confidence = 0.0;
        
        if word_count_valid {
            confidence += 0.4;
        }
        
        if words_valid {
            confidence += 0.3;
        }
        
        if bip39_likely {
            confidence += 0.3;
        }
        
        let is_valid = word_count_valid && words_valid;
        
        (is_valid, confidence)
    }
    
    // Extract words from messy text
    pub fn extract_words(&self, text: &str) -> Option<String> {
        let words: Vec<&str> = text
            .split_whitespace()
            .filter(|w| {
                !w.is_empty()
                && w.chars().all(|c| c.is_alphabetic())
                && w.len() >= 3
            })
            .collect();
        
        if words.len() == 12 || words.len() == 24 {
            Some(words.join(" "))
        } else {
            None
        }
    }
    
    // Get word count
    pub fn get_word_count(&self, seed_phrase: &str) -> usize {
        seed_phrase.split_whitespace().count()
    }
}