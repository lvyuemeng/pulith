use std::sync::atomic::{AtomicU64, AtomicU8, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::time::sleep;

/// Token bucket algorithm implementation for bandwidth limiting.
/// 
/// This implementation uses atomic operations for thread safety and
/// provides async token acquisition with proper rate limiting.
/// 
/// The adaptive version can dynamically adjust the refill rate based on
/// network conditions and congestion control algorithms.
pub struct TokenBucket {
    tokens: AtomicU64,
    capacity: u64,
    refill_rate: AtomicU64, // Changed to AtomicU64 for dynamic adjustment
    last_refill: Arc<AtomicInstant>,
    // Adaptive rate limiting fields
    adaptive_config: Arc<AdaptiveConfig>,
    metrics: Arc<RateMetrics>,
    congestion_state: AtomicU8, // 0: normal, 1: congestion, 2: recovery
}

/// Configuration for adaptive rate limiting
#[derive(Debug, Clone)]
pub struct AdaptiveConfig {
    /// Minimum refill rate (bytes per second)
    pub min_rate: u64,
    /// Maximum refill rate (bytes per second)
    pub max_rate: u64,
    /// Target utilization threshold (0.0 to 1.0)
    pub target_utilization: f64,
    /// Congestion detection threshold (0.0 to 1.0)
    pub congestion_threshold: f64,
    /// Recovery factor when congestion is detected (0.0 to 1.0)
    pub recovery_factor: f64,
    /// Growth factor when increasing rate (1.0 to 2.0)
    pub growth_factor: f64,
    /// Measurement window for rate adjustments (in milliseconds)
    pub measurement_window_ms: u64,
}

impl Default for AdaptiveConfig {
    fn default() -> Self {
        Self {
            min_rate: 1024, // 1KB/s minimum
            max_rate: 100 * 1024 * 1024, // 100MB/s maximum
            target_utilization: 0.8,
            congestion_threshold: 0.95,
            recovery_factor: 0.5,
            growth_factor: 1.1,
            measurement_window_ms: 1000, // 1 second window
        }
    }
}

/// Metrics for tracking rate limiting performance
#[derive(Debug, Default)]
pub struct RateMetrics {
    /// Total bytes acquired
    pub total_bytes: AtomicU64,
    /// Total wait time in microseconds
    pub total_wait_time_us: AtomicU64,
    /// Number of acquisitions
    pub acquisition_count: AtomicU64,
    /// Number of times tokens were not immediately available
    pub wait_count: AtomicU64,
    /// Last measurement timestamp
    pub last_measurement: AtomicU64, // Unix timestamp in milliseconds
    /// Bytes acquired in current measurement window
    pub window_bytes: AtomicU64,
}

impl RateMetrics {
    pub fn record_acquisition(&self, bytes: u64, wait_time_us: u64) {
        self.total_bytes.fetch_add(bytes, Ordering::Relaxed);
        self.total_wait_time_us.fetch_add(wait_time_us, Ordering::Relaxed);
        self.acquisition_count.fetch_add(1, Ordering::Relaxed);
        if wait_time_us > 0 {
            self.wait_count.fetch_add(1, Ordering::Relaxed);
        }
        self.window_bytes.fetch_add(bytes, Ordering::Relaxed);
    }
    
    pub fn get_throughput(&self) -> f64 {
        let total_bytes = self.total_bytes.load(Ordering::Relaxed);
        let total_wait_us = self.total_wait_time_us.load(Ordering::Relaxed);
        let count = self.acquisition_count.load(Ordering::Relaxed);
        
        if count == 0 {
            return 0.0;
        }
        
        // Calculate effective throughput considering wait times
        let total_time_s = total_wait_us as f64 / 1_000_000.0;
        if total_time_s > 0.0 {
            total_bytes as f64 / total_time_s
        } else {
            0.0
        }
    }
    
    pub fn get_utilization(&self, current_rate: u64) -> f64 {
        let window_bytes = self.window_bytes.load(Ordering::Relaxed);
        let window_duration_s = 1.0; // 1 second window
        let expected_bytes = current_rate as f64 * window_duration_s;
        
        if expected_bytes > 0.0 {
            window_bytes as f64 / expected_bytes
        } else {
            0.0
        }
    }
    
    pub fn reset_window(&self) {
        self.window_bytes.store(0, Ordering::Relaxed);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        self.last_measurement.store(now, Ordering::Relaxed);
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum CongestionState {
    Normal = 0,
    Congestion = 1,
    Recovery = 2,
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
            refill_rate: AtomicU64::new(refill_rate),
            last_refill: Arc::new(AtomicInstant::new(Instant::now())),
            adaptive_config: Arc::new(AdaptiveConfig::default()),
            metrics: Arc::new(RateMetrics::default()),
            congestion_state: AtomicU8::new(CongestionState::Normal as u8),
        }
    }
    
    /// Create a new adaptive TokenBucket with custom configuration.
    /// 
    /// # Arguments
    /// 
    /// * `capacity` - Maximum number of tokens the bucket can hold (in bytes)
    /// * `refill_rate` - Initial rate at which tokens are refilled (in bytes per second)
    /// * `config` - Adaptive configuration for rate adjustments
    pub fn new_adaptive(capacity: u64, refill_rate: u64, config: AdaptiveConfig) -> Self {
        Self {
            tokens: AtomicU64::new(capacity),
            capacity,
            refill_rate: AtomicU64::new(refill_rate),
            last_refill: Arc::new(AtomicInstant::new(Instant::now())),
            adaptive_config: Arc::new(config),
            metrics: Arc::new(RateMetrics::default()),
            congestion_state: AtomicU8::new(CongestionState::Normal as u8),
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
        let tokens_needed = bytes as u64;
        let start_time = Instant::now();
        
        loop {
            // Refill tokens based on elapsed time
            self.refill();
            
            // Try to acquire tokens
            let current_tokens = self.tokens.load(Ordering::Relaxed);
            if current_tokens >= tokens_needed {
                // Successfully acquire tokens
                let new_tokens = current_tokens - tokens_needed;
                if self.tokens.compare_exchange_weak(
                    current_tokens,
                    new_tokens,
                    Ordering::Relaxed,
                    Ordering::Relaxed
                ).is_ok() {
                    // Record metrics and potentially adjust rate
                    let wait_time_us = start_time.elapsed().as_micros() as u64;
                    self.metrics.record_acquisition(tokens_needed, wait_time_us);
                    self.check_and_adjust_rate();
                    return;
                }
                // If compare_exchange failed, retry the loop
                continue;
            }
            
            // Not enough tokens, calculate wait time
            let deficit = tokens_needed - current_tokens;
            let current_rate = self.refill_rate.load(Ordering::Relaxed);
            let wait_time = Duration::from_secs_f64(deficit as f64 / current_rate as f64);
            
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
            let current_rate = self.refill_rate.load(Ordering::Relaxed);
            let tokens_to_add = (current_rate as f64 * elapsed.as_secs_f64()) as u64;
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
    
    /// Get the current refill rate.
    pub fn current_rate(&self) -> u64 {
        self.refill_rate.load(Ordering::Relaxed)
    }
    
    /// Check network conditions and adjust the refill rate accordingly.
    fn check_and_adjust_rate(&self) {
        let config = &self.adaptive_config;
        let current_rate = self.refill_rate.load(Ordering::Relaxed);
        let utilization = self.metrics.get_utilization(current_rate);
        
        // Check if we should adjust the rate based on measurement window
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let last_measurement = self.metrics.last_measurement.load(Ordering::Relaxed);
        
        if now - last_measurement >= config.measurement_window_ms {
            self.adjust_rate_based_on_conditions(utilization);
            self.metrics.reset_window();
        }
    }
    
    /// Adjust the rate based on current utilization and congestion state.
    fn adjust_rate_based_on_conditions(&self, utilization: f64) {
        let config = &self.adaptive_config;
        let current_rate = self.refill_rate.load(Ordering::Relaxed);
        let current_state = self.congestion_state.load(Ordering::Relaxed);
        
        match current_state {
            0 => { // Normal state
                if utilization > config.congestion_threshold {
                    // Enter congestion state
                    self.congestion_state.store(CongestionState::Congestion as u8, Ordering::Relaxed);
                    let new_rate = (current_rate as f64 * config.recovery_factor).max(config.min_rate as f64) as u64;
                    self.refill_rate.store(new_rate, Ordering::Relaxed);
                } else if utilization < config.target_utilization {
                    // Increase rate gradually
                    let new_rate = (current_rate as f64 * config.growth_factor).min(config.max_rate as f64) as u64;
                    self.refill_rate.store(new_rate, Ordering::Relaxed);
                }
            }
            1 => { // Congestion state
                if utilization < config.target_utilization {
                    // Enter recovery state
                    self.congestion_state.store(CongestionState::Recovery as u8, Ordering::Relaxed);
                } else {
                    // Stay in congestion, further reduce rate
                    let new_rate = (current_rate as f64 * config.recovery_factor).max(config.min_rate as f64) as u64;
                    self.refill_rate.store(new_rate, Ordering::Relaxed);
                }
            }
            2 => { // Recovery state
                if utilization < config.congestion_threshold {
                    // Back to normal state
                    self.congestion_state.store(CongestionState::Normal as u8, Ordering::Relaxed);
                    let new_rate = (current_rate as f64 * config.growth_factor).min(config.max_rate as f64) as u64;
                    self.refill_rate.store(new_rate, Ordering::Relaxed);
                } else {
                    // Still congested, go back to congestion state
                    self.congestion_state.store(CongestionState::Congestion as u8, Ordering::Relaxed);
                }
            }
            _ => {}
        }
    }
    
    /// Get current metrics for monitoring.
    pub fn get_metrics(&self) -> &RateMetrics {
        &self.metrics
    }
    
    /// Force a rate adjustment (useful for testing or manual control).
    pub fn set_rate(&self, new_rate: u64) {
        let config = &self.adaptive_config;
        let clamped_rate = new_rate.clamp(config.min_rate, config.max_rate);
        self.refill_rate.store(clamped_rate, Ordering::Relaxed);
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
    
    #[tokio::test]
    async fn test_adaptive_rate_limiting() {
        let config = AdaptiveConfig {
            min_rate: 10,
            max_rate: 1000,
            target_utilization: 0.8,
            congestion_threshold: 0.9,
            recovery_factor: 0.5,
            growth_factor: 1.2,
            measurement_window_ms: 100,
        };
        
        let bucket = TokenBucket::new_adaptive(100, 100, config);
        
        // Initially should have the configured rate
        assert_eq!(bucket.current_rate(), 100);
        
        // Force a rate adjustment
        bucket.set_rate(500);
        assert_eq!(bucket.current_rate(), 500);
        
        // Test that rate is clamped to max
        bucket.set_rate(2000);
        assert_eq!(bucket.current_rate(), 1000);
        
        // Test that rate is clamped to min
        bucket.set_rate(5);
        assert_eq!(bucket.current_rate(), 10);
    }
    
    #[tokio::test]
    async fn test_congestion_detection() {
        let config = AdaptiveConfig {
            min_rate: 10,
            max_rate: 1000,
            target_utilization: 0.5,
            congestion_threshold: 0.8,
            recovery_factor: 0.5,
            growth_factor: 1.2,
            measurement_window_ms: 50,
        };
        
        let bucket = TokenBucket::new_adaptive(100, 100, config);
        
        // Simulate high utilization by acquiring many tokens
        for _ in 0..20 {
            bucket.acquire(10).await;
        }
        
        // Wait for measurement window
        sleep(Duration::from_millis(60)).await;
        
        // Acquire more tokens to trigger rate adjustment
        bucket.acquire(10).await;
        
        // Rate should have been adjusted due to congestion
        assert!(bucket.current_rate() < 100);
    }
    
    #[tokio::test]
    async fn test_metrics_collection() {
        let bucket = TokenBucket::new(100, 100);
        let metrics = bucket.get_metrics();
        
        // Initially no metrics
        assert_eq!(metrics.total_bytes.load(Ordering::Relaxed), 0);
        assert_eq!(metrics.acquisition_count.load(Ordering::Relaxed), 0);
        
        // Acquire tokens to update metrics
        bucket.acquire(50).await;
        
        // Metrics should now show the acquisition
        assert!(metrics.total_bytes.load(Ordering::Relaxed) > 0);
        assert!(metrics.acquisition_count.load(Ordering::Relaxed) > 0);
    }
}