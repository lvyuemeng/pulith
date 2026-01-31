use std::time::Duration;

/// Calculate the delay before a retry attempt using exponential backoff.
///
/// The delay formula is: `base * 2^retry_count`
///
/// # Arguments
///
/// * `retry_count` - The current retry number (0-indexed: 0 = first retry)
/// * `base` - The base delay duration
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use pulith_fetch::core::retry_delay;
///
/// // No retries yet (first attempt) - no delay needed before calling this
/// assert_eq!(retry_delay(0, Duration::from_millis(100)), Duration::from_millis(100));
///
/// // First retry: base * 2^0 = base
/// assert_eq!(retry_delay(0, Duration::from_millis(100)), Duration::from_millis(100));
///
/// // Second retry: base * 2^1 = base * 2
/// assert_eq!(retry_delay(1, Duration::from_millis(100)), Duration::from_millis(200));
///
/// // Third retry: base * 2^2 = base * 4
/// assert_eq!(retry_delay(2, Duration::from_millis(100)), Duration::from_millis(400));
/// ```
pub fn retry_delay(retry_count: u32, base: Duration) -> Duration {
    // Use saturating_pow to prevent overflow
    let multiplier = 2_u32.saturating_pow(retry_count);

    // Use saturating_mul to prevent Duration overflow
    base.saturating_mul(multiplier)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delay_basic() {
        let base = Duration::from_millis(100);

        // First retry (retry_count = 0): base * 2^0 = base
        assert_eq!(retry_delay(0, base), Duration::from_millis(100));

        // Second retry (retry_count = 1): base * 2^1 = base * 2
        assert_eq!(retry_delay(1, base), Duration::from_millis(200));

        // Third retry (retry_count = 2): base * 2^2 = base * 4
        assert_eq!(retry_delay(2, base), Duration::from_millis(400));

        // Fourth retry (retry_count = 3): base * 2^3 = base * 8
        assert_eq!(retry_delay(3, base), Duration::from_millis(800));
    }

    #[test]
    fn test_retry_delay_different_base() {
        let base = Duration::from_secs(1);

        assert_eq!(retry_delay(0, base), Duration::from_secs(1));
        assert_eq!(retry_delay(1, base), Duration::from_secs(2));
        assert_eq!(retry_delay(2, base), Duration::from_secs(4));
    }

    #[test]
    fn test_retry_delay_zero_base() {
        let base = Duration::from_millis(0);

        // Even with exponential backoff, zero base stays zero
        assert_eq!(retry_delay(0, base), Duration::from_millis(0));
        assert_eq!(retry_delay(1, base), Duration::from_millis(0));
        assert_eq!(retry_delay(10, base), Duration::from_millis(0));
    }

    #[test]
    fn test_retry_delay_large_values() {
        let base = Duration::from_millis(1);

        // Test that it handles large retry counts without panicking
        let delay = retry_delay(20, base);
        // 2^20 = 1,048,576 milliseconds = ~1048 seconds
        assert_eq!(delay, Duration::from_millis(1_048_576));
    }

    #[test]
    fn test_retry_delay_overflow_protection() {
        // Use a very large base to test saturating behavior
        let base = Duration::from_secs(u64::MAX / 2);

        // Even with multiplier, should not overflow due to saturating_mul
        let delay = retry_delay(2, base);
        assert!(delay > Duration::from_secs(0));
    }

    #[test]
    fn test_retry_delay_exponential_growth() {
        let base = Duration::from_millis(10);

        // Verify exponential growth pattern
        let delays: Vec<Duration> = (0..5).map(|i| retry_delay(i, base)).collect();

        // Each delay should be double the previous (except the first)
        for i in 1..delays.len() {
            assert_eq!(delays[i], delays[i - 1] * 2);
        }
    }

    #[test]
    fn test_retry_delay_microseconds() {
        let base = Duration::from_micros(500);

        assert_eq!(retry_delay(0, base), Duration::from_micros(500));
        assert_eq!(retry_delay(1, base), Duration::from_micros(1000));
        assert_eq!(retry_delay(2, base), Duration::from_micros(2000));
    }

    #[test]
    fn test_retry_delay_nanoseconds() {
        let base = Duration::from_nanos(1000);

        assert_eq!(retry_delay(0, base), Duration::from_nanos(1000));
        assert_eq!(retry_delay(1, base), Duration::from_nanos(2000));
        assert_eq!(retry_delay(2, base), Duration::from_nanos(4000));
    }
}
