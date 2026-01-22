use std::time::Duration;

pub fn retry_delay(attempt: u32, base: Duration) -> Duration {
    let multiplier = 2_u64.saturating_pow(attempt.saturating_sub(1));
    let total_millis = base.as_millis() as u64 * multiplier;
    Duration::from_millis(total_millis)
}

pub fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_retry_delay_first_attempt() {
        let delay = retry_delay(1, Duration::from_millis(100));
        assert_eq!(delay, Duration::from_millis(100));
    }

    #[test]
    fn test_retry_delay_exponential_backoff() {
        assert_eq!(
            retry_delay(1, Duration::from_millis(100)),
            Duration::from_millis(100)
        );
        assert_eq!(
            retry_delay(2, Duration::from_millis(100)),
            Duration::from_millis(200)
        );
        assert_eq!(
            retry_delay(3, Duration::from_millis(100)),
            Duration::from_millis(400)
        );
        assert_eq!(
            retry_delay(4, Duration::from_millis(100)),
            Duration::from_millis(800)
        );
    }

    #[test]
    fn test_is_redirect() {
        assert!(is_redirect(301));
        assert!(is_redirect(302));
        assert!(is_redirect(303));
        assert!(is_redirect(307));
        assert!(is_redirect(308));
        assert!(!is_redirect(200));
        assert!(!is_redirect(404));
        assert!(!is_redirect(500));
    }
}
