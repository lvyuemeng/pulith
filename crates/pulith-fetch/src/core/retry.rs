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
