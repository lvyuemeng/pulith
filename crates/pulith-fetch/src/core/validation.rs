use std::time::Duration;

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
