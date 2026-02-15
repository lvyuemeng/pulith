//! Rate control: limiting, backoff, and throttling.  
  
pub mod backoff;  
pub mod bandwidth;  
pub mod throttled;  
  
pub use backoff::retry_delay;  
pub use bandwidth::{TokenBucket, AdaptiveConfig, RateMetrics};  
pub use throttled::{ThrottledStream, AsyncThrottledStream}; 
