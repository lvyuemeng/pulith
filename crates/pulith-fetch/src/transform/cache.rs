//! HTTP caching support for conditional requests.
//!
//! This module provides types and functions for implementing HTTP caching
//! based on ETags, Last-Modified timestamps, and cache control directives.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CacheError {
    #[error("Cache entry not found")]
    NotFound,
    #[error("Cache entry expired")]
    Expired,
    #[error("Invalid cache entry: {0}")]
    InvalidEntry(String),
    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheControl {
    pub max_age: Option<u64>,
    pub no_cache: bool,
    pub no_store: bool,
    pub must_revalidate: bool,
    pub private: bool,
    pub public: bool,
    pub proxy_revalidate: bool,
    pub s_maxage: Option<u64>,
}

impl Default for CacheControl {
    fn default() -> Self {
        Self {
            max_age: None,
            no_cache: false,
            no_store: false,
            must_revalidate: false,
            private: false,
            public: false,
            proxy_revalidate: false,
            s_maxage: None,
        }
    }
}

impl CacheControl {
    pub fn parse(header: &str) -> Self {
        let mut control = Self::default();

        for directive in header.split(',') {
            let directive = directive.trim();

            match directive {
                "no-cache" => control.no_cache = true,
                "no-store" => control.no_store = true,
                "must-revalidate" => control.must_revalidate = true,
                "private" => control.private = true,
                "public" => control.public = true,
                "proxy-revalidate" => control.proxy_revalidate = true,
                _ => {
                    if let Some(max_age) = directive.strip_prefix("max-age=") {
                        if let Ok(seconds) = max_age.parse::<u64>() {
                            control.max_age = Some(seconds);
                        }
                    } else if let Some(s_maxage) = directive.strip_prefix("s-maxage=") {
                        if let Ok(seconds) = s_maxage.parse::<u64>() {
                            control.s_maxage = Some(seconds);
                        }
                    }
                }
            }
        }

        control
    }

    pub fn is_cacheable(&self) -> bool {
        !self.no_store && !self.no_cache
    }

    pub fn is_fresh(&self, stored_time: SystemTime) -> bool {
        if self.must_revalidate {
            return false;
        }

        let max_age = self.s_maxage.or(self.max_age);
        if let Some(max_age) = max_age {
            let elapsed = stored_time.elapsed().unwrap_or_default().as_secs();
            elapsed < max_age
        } else {
            true
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    pub url: String,
    pub etag: Option<String>,
    pub last_modified: Option<SystemTime>,
    pub cache_control: CacheControl,
    pub stored_at: SystemTime,
    pub content_length: Option<u64>,
    pub content_type: Option<String>,
    pub headers: HashMap<String, String>,
    pub vary: Option<String>,
}

impl CacheEntry {
    pub fn new(url: String) -> Self {
        Self {
            url,
            etag: None,
            last_modified: None,
            cache_control: CacheControl::default(),
            stored_at: SystemTime::now(),
            content_length: None,
            content_type: None,
            headers: HashMap::new(),
            vary: None,
        }
    }

    pub fn is_valid(&self) -> bool {
        if !self.cache_control.is_fresh(self.stored_at) {
            return false;
        }

        let max_age = self.cache_control.max_age.unwrap_or(86400);
        let elapsed = self.stored_at.elapsed().unwrap_or_default().as_secs();
        elapsed < max_age
    }

    pub fn cache_key(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.url.hash(&mut hasher);

        if let Some(vary) = &self.vary {
            vary.hash(&mut hasher);
        }

        format!("cache_{}", hasher.finish())
    }
}

#[derive(Debug, Clone)]
pub struct ConditionalHeaders {
    pub if_none_match: Option<String>,
    pub if_modified_since: Option<SystemTime>,
}

impl ConditionalHeaders {
    pub fn from_cache_entry(entry: &CacheEntry) -> Self {
        Self {
            if_none_match: entry.etag.clone(),
            if_modified_since: entry.last_modified,
        }
    }

    pub fn to_header_map(&self) -> reqwest::header::HeaderMap {
        let mut headers = reqwest::header::HeaderMap::new();

        if let Some(if_none_match) = &self.if_none_match {
            headers.insert(
                reqwest::header::IF_NONE_MATCH,
                if_none_match.parse().unwrap(),
            );
        }

        if let Some(if_modified_since) = self.if_modified_since {
            if let Ok(since_str) = httpdate::fmt_http_date(if_modified_since) {
                headers.insert(
                    reqwest::header::IF_MODIFIED_SINCE,
                    since_str.parse().unwrap(),
                );
            }
        }

        headers
    }
}

#[derive(Debug, Clone)]
pub enum CacheValidation {
    Fresh,
    StaleNeedsValidation,
    Invalid,
}

#[derive(Debug)]
pub struct HttpCache {
    entries: HashMap<String, CacheEntry>,
    max_entries: usize,
}

impl HttpCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
        }
    }

    pub fn get(&self, url: &str) -> Result<&CacheEntry, CacheError> {
        self.entries.get(url).ok_or(CacheError::NotFound)
    }

    pub fn put(&mut self, entry: CacheEntry) -> Result<(), CacheError> {
        if self.entries.len() >= self.max_entries {
            self.evict_oldest();
        }

        self.entries.insert(entry.url.clone(), entry);
        Ok(())
    }

    pub fn remove(&mut self, url: &str) -> Result<(), CacheError> {
        self.entries.remove(url).ok_or(CacheError::NotFound)?;
        Ok(())
    }

    pub fn validate(&self, url: &str) -> Result<CacheValidation, CacheError> {
        let entry = self.get(url)?;

        if !entry.is_valid() {
            return Ok(CacheValidation::Invalid);
        }

        if entry.cache_control.is_fresh(entry.stored_at) {
            Ok(CacheValidation::Fresh)
        } else {
            Ok(CacheValidation::StaleNeedsValidation)
        }
    }

    pub fn get_conditional_headers(&self, url: &str) -> Result<ConditionalHeaders, CacheError> {
        let entry = self.get(url)?;
        Ok(ConditionalHeaders::from_cache_entry(entry))
    }

    pub fn update_from_response(
        &mut self,
        url: &str,
        response: &reqwest::Response,
    ) -> Result<(), CacheError> {
        let mut entry = CacheEntry::new(url.to_string());

        if let Some(etag) = response.headers().get(reqwest::header::ETAG) {
            entry.etag = Some(etag.to_str().unwrap_or_default().to_string());
        }

        if let Some(last_modified) = response.headers().get(reqwest::header::LAST_MODIFIED) {
            if let Ok(parsed) =
                httpdate::parse_http_date(last_modified.to_str().unwrap_or_default())
            {
                entry.last_modified = Some(parsed);
            }
        }

        if let Some(cache_control) = response.headers().get(reqwest::header::CACHE_CONTROL) {
            entry.cache_control = CacheControl::parse(cache_control.to_str().unwrap_or_default());
        }

        if let Some(content_length) = response.headers().get(reqwest::header::CONTENT_LENGTH) {
            if let Ok(length) = content_length.to_str().unwrap_or_default().parse::<u64>() {
                entry.content_length = Some(length);
            }
        }

        if let Some(content_type) = response.headers().get(reqwest::header::CONTENT_TYPE) {
            entry.content_type = Some(content_type.to_str().unwrap_or_default().to_string());
        }

        if let Some(vary) = response.headers().get(reqwest::header::VARY) {
            entry.vary = Some(vary.to_str().unwrap_or_default().to_string());
        }

        for (name, value) in response.headers() {
            let name_str = name.as_str();
            if !matches!(
                name,
                &reqwest::header::ETAG
                    | &reqwest::header::LAST_MODIFIED
                    | &reqwest::header::CACHE_CONTROL
                    | &reqwest::header::CONTENT_LENGTH
                    | &reqwest::header::CONTENT_TYPE
                    | &reqwest::header::VARY
            ) {
                entry.headers.insert(
                    name_str.to_string(),
                    value.to_str().unwrap_or_default().to_string(),
                );
            }
        }

        self.put(entry)
    }

    pub fn clear(&mut self) {
        self.entries.clear();
    }

    pub fn stats(&self) -> CacheStats {
        CacheStats {
            total_entries: self.entries.len(),
            max_entries: self.max_entries,
            fresh_entries: self
                .entries
                .values()
                .filter(|e| e.is_valid() && e.cache_control.is_fresh(e.stored_at))
                .count(),
            stale_entries: self
                .entries
                .values()
                .filter(|e| e.is_valid() && !e.cache_control.is_fresh(e.stored_at))
                .count(),
            expired_entries: self.entries.values().filter(|e| !e.is_valid()).count(),
        }
    }

    fn evict_oldest(&mut self) {
        if self.entries.is_empty() {
            return;
        }

        let oldest_url = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.stored_at)
            .map(|(url, _)| url.clone());

        if let Some(url) = oldest_url {
            self.entries.remove(&url);
        }
    }
}

#[derive(Debug, Clone)]
pub struct CacheStats {
    pub total_entries: usize,
    pub max_entries: usize,
    pub fresh_entries: usize,
    pub stale_entries: usize,
    pub expired_entries: usize,
}

mod httpdate {
    use std::time::{SystemTime, UNIX_EPOCH};

    pub fn parse_http_date(date_str: &str) -> Result<SystemTime, Box<dyn std::error::Error>> {
        let timestamp = chrono::DateTime::parse_from_rfc2822(date_str)?;
        let system_time = UNIX_EPOCH + std::time::Duration::from_secs(timestamp.timestamp() as u64);
        Ok(system_time)
    }

    pub fn fmt_http_date(time: SystemTime) -> Result<String, Box<dyn std::error::Error>> {
        let datetime = chrono::DateTime::<chrono::Utc>::from(time);
        Ok(datetime.to_rfc2822())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_cache_control_parsing() {
        let control = CacheControl::parse("max-age=3600, public");
        assert_eq!(control.max_age, Some(3600));
        assert!(control.public);
        assert!(!control.no_cache);

        let control = CacheControl::parse("no-cache, must-revalidate");
        assert!(control.no_cache);
        assert!(control.must_revalidate);
        assert!(control.max_age.is_none());
    }

    #[test]
    fn test_cache_control_freshness() {
        let mut control = CacheControl::default();
        control.max_age = Some(3600);

        let past_time = SystemTime::now() - Duration::from_secs(1800);
        assert!(control.is_fresh(past_time));

        let too_old_time = SystemTime::now() - Duration::from_secs(7200);
        assert!(!control.is_fresh(too_old_time));
    }

    #[test]
    fn test_cache_entry_validation() {
        let mut entry = CacheEntry::new("https://example.com".to_string());
        entry.cache_control.max_age = Some(3600);
        entry.stored_at = SystemTime::now() - Duration::from_secs(1800);

        assert!(entry.is_valid());

        entry.stored_at = SystemTime::now() - Duration::from_secs(7200);
        assert!(!entry.is_valid());
    }

    #[test]
    fn test_http_cache_operations() {
        let mut cache = HttpCache::new(10);

        let mut entry = CacheEntry::new("https://example.com".to_string());
        entry.etag = Some("\"12345\"".to_string());
        entry.cache_control.max_age = Some(3600);

        cache.put(entry.clone()).unwrap();

        let retrieved = cache.get("https://example.com").unwrap();
        assert_eq!(retrieved.etag, entry.etag);

        let validation = cache.validate("https://example.com").unwrap();
        assert!(matches!(validation, CacheValidation::Fresh));

        let stats = cache.stats();
        assert_eq!(stats.total_entries, 1);
        assert_eq!(stats.fresh_entries, 1);
    }

    #[test]
    fn test_conditional_headers() {
        let mut entry = CacheEntry::new("https://example.com".to_string());
        entry.etag = Some("\"12345\"".to_string());
        entry.last_modified = Some(SystemTime::now() - Duration::from_secs(3600));

        let headers = ConditionalHeaders::from_cache_entry(&entry);
        assert_eq!(headers.if_none_match, Some("\"12345\"".to_string()));
        assert!(headers.if_modified_since.is_some());
    }

    #[test]
    fn test_cache_eviction() {
        let mut cache = HttpCache::new(2);

        let entry1 = CacheEntry::new("https://example1.com".to_string());
        let entry2 = CacheEntry::new("https://example2.com".to_string());
        let entry3 = CacheEntry::new("https://example3.com".to_string());

        cache.put(entry1).unwrap();
        cache.put(entry2).unwrap();
        cache.put(entry3).unwrap();

        assert!(cache.get("https://example1.com").is_err());
        assert!(cache.get("https://example2.com").is_ok());
        assert!(cache.get("https://example3.com").is_ok());
    }
}
