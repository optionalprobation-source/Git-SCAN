use aho_corasick::{AhoCorasick, MatchKind};
use once_cell::sync::Lazy;
use crate::models::scan::FilePriority;

// Ek hi baar compile hone wala super-fast Aho-Corasick engine
static SKIP_AC: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .match_kind(MatchKind::LeftmostFirst)
        .build(&[
            "node_modules/", ".git/", "dist/", "build/", "artifacts/", "cache/",
            ".min.js", ".map", ".lock", "package-lock.json", "yarn.lock"
        ]).unwrap()
});

static TARGET_AC: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(&[
            ".env", ".git-credentials", ".secret", "id_rsa", "id_ed25519", 
            ".pem", ".key", "keystore.json", "hardhat.config", "truffle-config", 
            "wallet.", "deploy.", "config.", "secrets.", "settings."
        ]).unwrap()
});

static CRITICAL_AC: Lazy<AhoCorasick> = Lazy::new(|| {
    AhoCorasick::builder()
        .ascii_case_insensitive(true)
        .build(&[
            ".env", ".git-credentials", "id_rsa", "id_ed25519", ".pem", ".key", "keystore.json", ".secret"
        ]).unwrap()
});

// Zero-allocation scan check
pub fn should_scan_file(filename: &str) -> bool {
    if SKIP_AC.is_match(filename) {
        return false;
    }
    TARGET_AC.is_match(filename)
}

// Zero-allocation priority check
pub fn get_file_priority(filename: &str) -> FilePriority {
    if CRITICAL_AC.is_match(filename) {
        return FilePriority::Critical;
    }
    if TARGET_AC.is_match(filename) {
        return FilePriority::High;
    }
    FilePriority::Medium
}
