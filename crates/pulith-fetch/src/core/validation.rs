
/// Returns `true` if the HTTP status code indicates a redirect.
///
/// # Recognized Redirect Codes
///
/// - 301: Moved Permanently
/// - 302: Found
/// - 303: See Other
/// - 307: Temporary Redirect
/// - 308: Permanent Redirect
///
/// # Examples
///
/// ```
/// use pulith_fetch::core::is_redirect;
///
/// assert!(is_redirect(301));
/// assert!(is_redirect(302));
/// assert!(!is_redirect(200));
/// assert!(!is_redirect(404));
/// ```
pub fn is_redirect(status: u16) -> bool {
    matches!(status, 301 | 302 | 303 | 307 | 308)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_redirect_all_codes() {
        // Test all recognized redirect codes
        assert!(is_redirect(301)); // Moved Permanently
        assert!(is_redirect(302)); // Found
        assert!(is_redirect(303)); // See Other
        assert!(is_redirect(307)); // Temporary Redirect
        assert!(is_redirect(308)); // Permanent Redirect
    }

    #[test]
    fn test_is_redirect_success_codes() {
        // Common success codes should not be redirects
        assert!(!is_redirect(200)); // OK
        assert!(!is_redirect(201)); // Created
        assert!(!is_redirect(204)); // No Content
    }

    #[test]
    fn test_is_redirect_client_error_codes() {
        // Client error codes should not be redirects
        assert!(!is_redirect(400)); // Bad Request
        assert!(!is_redirect(401)); // Unauthorized
        assert!(!is_redirect(403)); // Forbidden
        assert!(!is_redirect(404)); // Not Found
        assert!(!is_redirect(429)); // Too Many Requests
    }

    #[test]
    fn test_is_redirect_server_error_codes() {
        // Server error codes should not be redirects
        assert!(!is_redirect(500)); // Internal Server Error
        assert!(!is_redirect(502)); // Bad Gateway
        assert!(!is_redirect(503)); // Service Unavailable
        assert!(!is_redirect(504)); // Gateway Timeout
    }

    #[test]
    fn test_is_redirect_informational_codes() {
        // Informational codes should not be redirects
        assert!(!is_redirect(100)); // Continue
        assert!(!is_redirect(101)); // Switching Protocols
        assert!(!is_redirect(102)); // Processing
    }

    #[test]
    fn test_is_redirect_edge_cases() {
        // Edge cases around redirect codes
        assert!(!is_redirect(300)); // Multiple Choices (not in our list)
        assert!(!is_redirect(304)); // Not Modified
        assert!(!is_redirect(305)); // Use Proxy (deprecated)
        assert!(!is_redirect(306)); // (Unused)
    }

    #[test]
    fn test_is_redirect_invalid_codes() {
        // Invalid HTTP status codes
        assert!(!is_redirect(0));
        assert!(!is_redirect(99));
        assert!(!is_redirect(600));
        assert!(!is_redirect(1000));
    }

    #[test]
    fn test_is_redirect_comprehensive_coverage() {
        // Test a comprehensive range of status codes
        let redirect_codes = [301, 302, 303, 307, 308];
        let non_redirect_codes = [
            100, 101, 102, 200, 201, 202, 203, 204, 205, 206, 207, 208, 226, 300, 304, 305, 306,
            400, 401, 402, 403, 404, 405, 406, 407, 408, 409, 410, 411, 412, 413, 414, 415, 416,
            417, 418, 421, 422, 423, 424, 425, 426, 428, 429, 431, 451, 500, 501, 502, 503, 504,
            505, 506, 507, 508, 510, 511,
        ];

        for code in &redirect_codes {
            assert!(is_redirect(*code), "Code {} should be a redirect", code);
        }

        for code in &non_redirect_codes {
            assert!(
                !is_redirect(*code),
                "Code {} should NOT be a redirect",
                code
            );
        }
    }
}
