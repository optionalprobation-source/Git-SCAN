use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use once_cell::sync::Lazy;
use crate::models::scan::FilePriority;

// Skip patterns - files to ignore completely
static SKIP_AC: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .match_kind(MatchKind::LeftmostFirst)
        .build(&[
            // Build artifacts and dependencies
            "node_modules/",
            ".git/",
            "dist/",
            "build/",
            "artifacts/",
            "cache/",
            "__pycache__/",
            
            // Minified and compiled files
            ".min.js",
            ".min.css",
            ".map",
            
            // Lock files
            ".lock",
            "package-lock.json",
            "yarn.lock",
            "Cargo.lock",
            
            // Binary and compiled files
            ".pyc",
            ".class",
            ".o",
            ".so",
            ".dll",
            ".exe",
            ".bin",
            
            // Test files
            "test/",
            "tests/",
            "spec/",
            "specs/",
            "__tests__/",
            
            // Documentation
            "docs/",
            
            // Temporary files
            ".tmp",
            ".temp",
            ".swp",
        ])
        .expect("Failed to compile SKIP patterns")
});

// Target patterns - files to scan
static TARGET_AC: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .match_kind(MatchKind::LeftmostFirst)
        .build(&[
            // Environment files
            ".env",
            ".env.local",
            ".env.production",
            ".env.development",
            
            // Credentials
            ".git-credentials",
            "credentials",
            
            // SSH keys
            "id_rsa",
            "id_ed25519",
            "id_dsa",
            
            // Certificates and keys
            ".pem",
            ".key",
            "keystore.json",
            
            // Secrets
            ".secret",
            "secrets.",
            "secret_key",
            
            // Blockchain configs
            "hardhat.config",
            "truffle-config",
            "foundry.toml",
            
            // Wallet files
            "wallet.",
            "wallets.",
            
            // Deploy scripts
            "deploy.",
            "deployment.",
            
            // Config files
            "config.",
            "settings.",
            
            // Private keys
            "private_key",
            "private-key",
            
            // Seed phrases
            "mnemonic",
            "seed_phrase",
            "seed-phrase",
        ])
        .expect("Failed to compile TARGET patterns")
});

// Critical file patterns - highest priority
static CRITICAL_AC: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasickBuilder::new()
        .ascii_case_insensitive(true)
        .match_kind(MatchKind::LeftmostFirst)
        .build(&[
            ".env",
            ".git-credentials",
            "id_rsa",
            "id_ed25519",
            ".pem",
            ".key",
            "keystore.json",
            ".secret",
            "private_key",
            "mnemonic",
            "seed_phrase",
        ])
        .expect("Failed to compile CRITICAL patterns")
});

// Zero-allocation scan check
#[inline(always)]
pub fn should_scan_file(filename: &str) -> bool {
    if SKIP_AC.is_match(filename) {
        return false;
    }
    
    TARGET_AC.is_match(filename)
}

// Zero-allocation priority check
#[inline(always)]
pub fn get_file_priority(filename: &str) -> FilePriority {
    if CRITICAL_AC.is_match(filename) {
        return FilePriority::Critical;
    }
    
    if TARGET_AC.is_match(filename) {
        return FilePriority::High;
    }
    
    FilePriority::Medium
}

// Batch file filtering for maximum throughput
pub fn filter_scan_files(file_paths: &[String]) -> Vec<String> {
    use rayon::prelude::*;
    
    file_paths
        .par_iter()
        .filter(|path| should_scan_file(path))
        .cloned()
        .collect()
}