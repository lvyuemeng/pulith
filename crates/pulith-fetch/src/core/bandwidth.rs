use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Token bucket algorithm implementation for bandwidth limiting.
/// 
/// This implementation uses atomic operations for thread safety and
/// provides async token acquisition with proper rate limiting.
pub struct TokenBucket {
    tokens: AtomicU64,
    capacity: u64,
    refill_rate: u64,
    last_refill: Arc<AtomicInstant>,
}

/// Atomic wrapper for Instant to use in atomic operations.
#[derive(Debug)]
struct AtomicInstant {
    instant: std::sync::Mutex<Instant>,
}

impl AtomicInstant {
    fn new(instant: Instant) -> Self {
        Self {
            instant: std::sync::Mutex::new(instant),
        }
    }
    
    fn get(&self) -> Instant {
        *self.instant.lock().unwrap()
    }
    
    fn set(&self, instant: Instant) {
        *self.instant.lock().unwrap() = instant;
    }
}

impl TokenBucket {
    /// Create a new TokenBucket with the specified capacity and refill rate.
    /// 
    /// # Arguments
    /// 
    /// * `capacity` - Maximum number of tokens the bucket can hold (in bytes)
    /// * `refill_rate` - Rate at which tokens are refilled (in bytes per second)
    pub fn new(capacity: u64, refill_rate: u64) -> Self {
        Self {
            tokens: AtomicU64::new(capacity),
            capacity,
            refill_rate,
            last_refill: Arc::new(AtomicInstant::new(Instant::now())),
        }
    }
    
    /// Acquire the specified number of tokens, waiting if necessary.
    /// 
    /// This method will block until enough tokens are available.
    /// 
    /// # Arguments
    /// 
    /// * `bytes` - Number of tokens (bytes) to acquire
    pub async fn acquire(&self, bytes: usize) {
        let tokens_needed = bytes as f64;
        
        loop {
            // Refill tokens based on elapsed time
            self.refill();
            
            // Try to acquire tokens
            let current_tokens = self.tokens.load(Ordering::Relaxed);
            if current_tokens >= tokens_needed as u64 {
                // Successfully acquire tokens
                let new_tokens = current_tokens - tokens_needed as u64;
                if self.tokens.compare_exchange_weak(
                    current_tokens,
                    new_tokens,
                    Ordering::Relaxed,
                    Ordering::Relaxed
                ).is_ok() {
                    return;
                }
                // If compare_exchange failed, retry the loop
                continue;
            }
            
            // Not enough tokens, calculate wait time
            let deficit = tokens_needed as u64 - current_tokens;
            let wait_time = Duration::from_secs_f64(deficit as f64 / self.refill_rate as f64);
            
            // Wait for tokens to be available
            sleep(wait_time).await;
        }
    }
    
    /// Try to acquire tokens without waiting.
    /// 
    /// Returns true if tokens were acquired, false otherwise.
    /// 
    /// # Arguments
    /// 
    /// * `bytes` - Number of tokens (bytes) to acquire
    pub fn try_acquire(&self, bytes: usize) -> bool {
        let tokens_needed = bytes as u64;
        
        // Refill tokens based on elapsed time
        self.refill();
        
        // Try to acquire tokens
        let current_tokens = self.tokens.load(Ordering::Relaxed);
        if current_tokens >= tokens_needed {
            // Successfully acquire tokens
            if self.tokens.compare_exchange_weak(
                current_tokens,
                current_tokens - tokens_needed,
                Ordering::Relaxed,
                Ordering::Relaxed
            ).is_ok() {
                return true;
            }
        }
        
        false
    }
    
    /// Refill tokens based on elapsed time since last refill.
    fn refill(&self) {
        let now = Instant::now();
        let last_refill = self.last_refill.get();
        let elapsed = now.duration_since(last_refill);
        
        if elapsed.as_secs_f64() > 0.0 {
            let tokens_to_add = (self.refill_rate as f64 * elapsed.as_secs_f64()) as u64;
            let current_tokens = self.tokens.load(Ordering::Relaxed);
            let new_tokens = (current_tokens + tokens_to_add).min(self.capacity);
            
            self.tokens.store(new_tokens, Ordering::Relaxed);
            self.last_refill.set(now);
        }
    }
    
    /// Get the current number of tokens in the bucket.
    pub fn available_tokens(&self) -> u64 {
        self.refill();
        self.tokens.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};
    
    #[tokio::test]
    async fn test_token_bucket_basic() {
        // Create a bucket with 100 bytes capacity and 50 bytes/second refill rate
        let bucket = TokenBucket::new(100, 50);
        
        // Should be able to acquire 50 bytes immediately
        bucket.acquire(50).await;
        assert!(bucket.available_tokens() <= 50);
        
        // Should be able to acquire another 50 bytes immediately
        bucket.acquire(50).await;
        assert!(bucket.available_tokens() <= 0);
        
        // Acquiring more should require waiting
        let start = Instant::now();
        bucket.acquire(25).await;
        let elapsed = start.elapsed();
        
        // Should have waited at least 0.5 seconds (25 bytes at 50 bytes/second)
        assert!(elapsed >= Duration::from_millis(450));
        assert!(elapsed <= Duration::from_millis(550));
    }
    
    #[tokio::test]
    async fn test_token_bucket_refill() {
        let bucket = TokenBucket::new(100, 100);
        
        // Acquire all tokens
        bucket.acquire(100).await;
        assert!(bucket.available_tokens() <= 0);
        
        // Wait for refill
        sleep(Duration::from_millis(100)).await;
        
        // Should have some tokens available
        let available = bucket.available_tokens();
        assert!(available > 5); // Should have at least 10 bytes (100 bytes/s * 0.1s)
        assert!(available <= 15); // Allow some tolerance
    }
    
    #[tokio::test]
    async fn test_token_bucket_concurrent() {
        let bucket = Arc::new(TokenBucket::new(1000, 100));
        let mut handles = vec![];
        
        // Spawn 10 concurrent tasks each trying to acquire 100 bytes
        for _ in 0..10 {
            let bucket_clone = Arc::clone(&bucket);
            let handle = tokio::spawn(async move {
                bucket_clone.acquire(100).await;
            });
            handles.push(handle);
        }
        
        // Wait for all tasks to complete
        for handle in handles {
            handle.await.unwrap();
        }
        
        // All tokens should be consumed
        assert!(bucket.available_tokens() <= 0);
    }
}