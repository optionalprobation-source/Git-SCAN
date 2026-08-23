use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

pub struct TokenRotator {
    tokens: Vec<String>,
    current_index: AtomicUsize,
    last_rotation: Mutex<Instant>,
}

impl TokenRotator {
    pub fn new(tokens: Vec<String>) -> Arc<Self> {
        Arc::new(Self {
            tokens,
            current_index: AtomicUsize::new(0),
            last_rotation: Mutex::new(Instant::now()),
        })
    }

    /// Har 60 second me token rotate karta hai.
    /// Returns current token (ya None agar tokens empty hain).
    pub fn get_token(&self) -> Option<String> {
        if self.tokens.is_empty() {
            return None;
        }

        let now = Instant::now();
        let mut last = self.last_rotation.lock().unwrap();

        if now.duration_since(*last) >= Duration::from_secs(60) {
            let idx = self.current_index.fetch_add(1, Ordering::Relaxed);
            *last = now;
            // Ensure index wraps around
            self.current_index.store(idx % self.tokens.len(), Ordering::Relaxed);
        }

        let idx = self.current_index.load(Ordering::Relaxed) % self.tokens.len();
        self.tokens.get(idx).cloned()
    }
}