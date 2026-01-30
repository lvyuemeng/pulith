//! HTTP caching functionality.
//!
//! This module provides HTTP caching with ETag and Last-Modified
//! support following RFC 7234.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::fs;
use tokio::sync::RwLock;

/// Cache configuration.
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Cache directory
    pub dir: PathBuf,
    /// Maximum cache size in bytes
    pub max_size: Option<u64>,
    /// Maximum age for cache entries
    pub max_age: Option<Duration>,
    /// Whether to persist metadata to disk
    pub persist_metadata: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            dir: PathBuf::from(".cache/pulith-fetch"),
            max_size: Some(1024 * 1024 * 1024), // 1GB
            max_age: Some(Duration::from_secs(7 * 24 * 60 * 60)), // 7 days
            persist_metadata: true,
        }
    }
}

/// Cache entry metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry {
    /// The URL that was cached
    pub url: String,
    /// ETag from the server
    pub etag: Option<String>,
    /// Last-Modified from the server
    pub last_modified: Option<u64>, // Unix timestamp
    /// When the entry was cached
    pub cached_at: u64, // Unix timestamp
    /// Size of the cached content
    pub size: u64,
    /// SHA256 checksum of the content
    pub checksum: [u8; 32],
    /// Number of times this entry was accessed
    pub access_count: u64,
    /// Last access time
    pub last_accessed: u64, // Unix timestamp
    /// Cache control max-age from server
    pub max_age: Option<u64>, // seconds
    /// Cache control no-cache directive
    pub no_cache: bool,
}

impl CacheEntry {
    /// Check if the entry is expired based on its age and max_age.
    pub fn is_expired(&self, config_max_age: Option<Duration>) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        // Check server-provided max-age first
        if let Some(server_max_age) = self.max_age {
            if self.cached_at + server_max_age < now {
                return true;
            }
        }
        
        // Check configuration max-age
        if let Some(config_max_age) = config_max_age {
            if self.cached_at + config_max_age.as_secs() < now {
                return true;
            }
        }
        
        false
    }
    
    /// Check if the entry should be revalidated.
    pub fn should_revalidate(&self) -> bool {
        self.no_cache || self.etag.is_some() || self.last_modified.is_some()
    }
}

/// HTTP cache implementation.
pub struct Cache {
    config: CacheConfig,
    entries: RwLock<HashMap<String, CacheEntry>>,
    current_size: RwLock<u64>,
}

impl Cache {
    /// Create a new cache with the given configuration.
    pub async fn new(config: CacheConfig) -> Result<Self> {
        // Create cache directory if it doesn't exist
        fs::create_dir_all(&config.dir).await.map_err(|e| {
            Error::Network(format!("Failed to create cache directory: {}", e))
        })?;
        
        let mut cache = Self {
            entries: RwLock::new(HashMap::new()),
            current_size: RwLock::new(0),
            config,
        };
        
        // Load metadata from disk if enabled
        if cache.config.persist_metadata {
            cache.load_metadata().await?;
        }
        
        Ok(cache)
    }
    
    /// Get a cached entry for the given URL.
    pub async fn get(&self, url: &str) -> Result<Option<CacheEntry>> {
        let entries = self.entries.read().await;
        
        if let Some(entry) = entries.get(url) {
            // Check if expired
            if entry.is_expired(self.config.max_age) {
                return Ok(None);
            }
            
            // Update access count and last accessed time
            drop(entries);
            self.update_access(url).await;
            
            let entries = self.entries.read().await;
            Ok(entries.get(url).cloned())
        } else {
            Ok(None)
        }
    }
    
    /// Store content in the cache.
    pub async fn put(
        &self,
        url: String,
        content: &[u8],
        etag: Option<String>,
        last_modified: Option<u64>,
        max_age: Option<u64>,
        no_cache: bool,
    ) -> Result<()> {
        // Calculate checksum
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(content);
        let checksum = hasher.finalize().into();
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        
        let entry = CacheEntry {
            url: url.clone(),
            etag,
            last_modified,
            cached_at: now,
            size: content.len() as u64,
            checksum,
            access_count: 1,
            last_accessed: now,
            max_age,
            no_cache,
        };
        
        // Check if we need to evict entries
        if let Some(max_size) = self.config.max_size {
            let current_size = *self.current_size.read().await;
            if current_size + entry.size > max_size {
                self.evict_lru(entry.size).await?;
            }
        }
        
        // Write content to file
        let cache_file = self.cache_file_path(&url);
        fs::write(&cache_file, content).await.map_err(|e| {
            Error::Network(format!("Failed to write cache file: {}", e))
        })?;
        
        // Update metadata
        {
            let mut entries = self.entries.write().await;
            let mut current_size = self.current_size.write().await;
            
            // Remove old entry if exists
            if let Some(old_entry) = entries.remove(&url) {
                *current_size = current_size.saturating_sub(old_entry.size);
            }
            
            entries.insert(url.clone(), entry.clone());
            *current_size += entry.size;
        }
        
        // Persist metadata if enabled
        if self.config.persist_metadata {
            self.save_metadata().await?;
        }
        
        Ok(())
    }
    
    /// Validate cached entry against server metadata.
    pub async fn validate(&self, url: &str, server_etag: Option<&str>, server_last_modified: Option<u64>) -> Result<bool> {
        let entries = self.entries.read().await;
        
        if let Some(entry) = entries.get(url) {
            // Check ETag
            if let (Some(cached_etag), Some(server_etag)) = (&entry.etag, server_etag) {
                if cached_etag == server_etag {
                    return Ok(true);
                }
            }
            
            // Check Last-Modified
            if let (Some(cached_modified), Some(server_modified)) = (entry.last_modified, server_last_modified) {
                if cached_modified >= server_modified {
                    return Ok(true);
                }
            }
        }
        
        Ok(false)
    }
    
    /// Get the cache file path for a URL.
    fn cache_file_path(&self, url: &str) -> PathBuf {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(url.as_bytes());
        let hash = hex::encode(hasher.finalize());
        self.config.dir.join(format!("{}.cache", hash))
    }
    
    /// Update access statistics for an entry.
    async fn update_access(&self, url: &str) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.get_mut(url) {
            entry.access_count += 1;
            entry.last_accessed = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs();
        }
    }
    
    /// Evict least recently used entries to make space.
    async fn evict_lru(&self, needed_space: u64) -> Result<()> {
        let mut entries = self.entries.write().await;
        let mut current_size = self.current_size.write().await;
        
        // Collect entries sorted by last accessed time
        let mut sorted_entries: Vec<_> = entries.iter().collect();
        sorted_entries.sort_by_key(|(_, entry)| entry.last_accessed);
        
        let mut freed_space = 0u64;
        let mut to_remove = Vec::new();
        
        for (url, entry) in sorted_entries {
            if freed_space >= needed_space {
                break;
            }
            
            to_remove.push(url.clone());
            freed_space += entry.size;
        }
        
        // Remove entries and delete files
        for url in to_remove {
            if let Some(entry) = entries.remove(&url) {
                *current_size = current_size.saturating_sub(entry.size);
                
                // Delete cache file
                let cache_file = self.cache_file_path(&url);
                let _ = fs::remove_file(cache_file).await;
            }
        }
        
        Ok(())
    }
    
    /// Load metadata from disk.
    async fn load_metadata(&self) -> Result<()> {
        let metadata_file = self.config.dir.join("metadata.json");
        
        if !metadata_file.exists() {
            return Ok(());
        }
        
        let content = fs::read_to_string(&metadata_file).await.map_err(|e| {
            Error::Network(format!("Failed to read metadata file: {}", e))
        })?;
        
        let loaded_entries: HashMap<String, CacheEntry> = serde_json::from_str(&content)
            .map_err(|e| Error::InvalidState(format!("Invalid metadata format: {}", e)))?;
        
        // Calculate current size
        let mut total_size = 0u64;
        for entry in loaded_entries.values() {
            total_size += entry.size;
        }
        
        *self.entries.write().await = loaded_entries;
        *self.current_size.write().await = total_size;
        
        Ok(())
    }
    
    /// Save metadata to disk.
    async fn save_metadata(&self) -> Result<()> {
        let metadata_file = self.config.dir.join("metadata.json");
        
        let entries = self.entries.read().await;
        let content = serde_json::to_string_pretty(&*entries)
            .map_err(|e| Error::InvalidState(format!("Failed to serialize metadata: {}", e)))?;
        
        fs::write(&metadata_file, content).await.map_err(|e| {
            Error::Network(format!("Failed to write metadata file: {}", e))
        })?;
        
        Ok(())
    }
    
    /// Clear all cached entries.
    pub async fn clear(&self) -> Result<()> {
        let entries = self.entries.read().await;
        
        // Delete all cache files
        for url in entries.keys() {
            let cache_file = self.cache_file_path(url);
            let _ = fs::remove_file(cache_file).await;
        }
        
        // Clear in-memory state
        drop(entries);
        self.entries.write().await.clear();
        *self.current_size.write().await = 0;
        
        // Delete metadata file
        let metadata_file = self.config.dir.join("metadata.json");
        let _ = fs::remove_file(metadata_file).await;
        
        Ok(())
    }
    
    /// Get cache statistics.
    pub async fn stats(&self) -> CacheStats {
        let entries = self.entries.read().await;
        let current_size = *self.current_size.read().await;
        
        CacheStats {
            entry_count: entries.len(),
            total_size: current_size,
            max_size: self.config.max_size,
            hit_count: 0, // TODO: Implement hit tracking
            miss_count: 0, // TODO: Implement miss tracking
        }
    }
}

/// Cache statistics.
#[derive(Debug, Clone)]
pub struct CacheStats {
    /// Number of entries in the cache
    pub entry_count: usize,
    /// Total size of cached content
    pub total_size: u64,
    /// Maximum cache size
    pub max_size: Option<u64>,
    /// Number of cache hits
    pub hit_count: u64,
    /// Number of cache misses
    pub miss_count: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    async fn create_test_cache() -> (Cache, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            dir: temp_dir.path().to_path_buf(),
            max_size: Some(1024),
            max_age: Some(Duration::from_secs(3600)),
            persist_metadata: true,
        };
        (Cache::new(config).await.unwrap(), temp_dir)
    }
    
    #[tokio::test]
    async fn test_cache_put_and_get() {
        let (cache, _temp_dir) = create_test_cache().await;
        
        let url = "https://example.com/test.txt";
        let content = b"Hello, World!";
        
        // Put content in cache
        cache.put(
            url.to_string(),
            content,
            Some("\"etag123\"".to_string()),
            Some(1234567890),
            Some(3600),
            false,
        )
        .await
        .unwrap();
        
        // Get content from cache
        let entry = cache.get(url).await.unwrap().unwrap();
        assert_eq!(entry.url, url);
        assert_eq!(entry.etag, Some("\"etag123\"".to_string()));
        assert_eq!(entry.last_modified, Some(1234567890));
        assert_eq!(entry.size, content.len() as u64);
    }
    
    #[tokio::test]
    async fn test_cache_expiration() {
        let (cache, _temp_dir) = create_test_cache().await;
        
        let url = "https://example.com/test.txt";
        let content = b"Hello, World!";
        
        // Put content with short max age
        cache.put(
            url.to_string(),
            content,
            None,
            None,
            Some(1), // 1 second
            false,
        )
        .await
        .unwrap();
        
        // Should be valid immediately
        assert!(cache.get(url).await.unwrap().is_some());
        
        // TODO: Mock time to test expiration
    }
    
    #[tokio::test]
    async fn test_cache_validation() {
        let (cache, _temp_dir) = create_test_cache().await;
        
        let url = "https://example.com/test.txt";
        let content = b"Hello, World!";
        
        // Put content in cache
        cache.put(
            url.to_string(),
            content,
            Some("\"etag123\"".to_string()),
            Some(1234567890),
            None,
            false,
        )
        .await
        .unwrap();
        
        // Validate with matching ETag
        assert!(cache.validate(url, Some("\"etag123\""), None).await.unwrap());
        
        // Validate with non-matching ETag
        assert!(!cache.validate(url, Some("\"etag456\""), None).await.unwrap());
        
        // Validate with matching Last-Modified
        assert!(cache.validate(url, None, Some(1234567890)).await.unwrap());
        
        // Validate with newer Last-Modified
        assert!(!cache.validate(url, None, Some(1234567891)).await.unwrap());
    }
    
    #[tokio::test]
    async fn test_cache_eviction() {
        let (cache, _temp_dir) = create_test_cache().await;
        
        // Fill cache beyond max size
        for i in 0..5 {
            let url = format!("https://example.com/test{}.txt", i);
            let content = vec![b'x'; 300]; // 300 bytes each
            cache.put(url, &content, None, None, None, false).await.unwrap();
        }
        
        // Check that only 3 entries fit (1024 / 300 ≈ 3)
        let stats = cache.stats().await;
        assert!(stats.entry_count <= 3);
    }
    
    #[tokio::test]
    async fn test_cache_clear() {
        let (cache, _temp_dir) = create_test_cache().await;
        
        // Add some entries
        cache.put(
            "https://example.com/test1.txt".to_string(),
            b"content1",
            None,
            None,
            None,
            false,
        )
        .await
        .unwrap();
        
        cache.put(
            "https://example.com/test2.txt".to_string(),
            b"content2",
            None,
            None,
            None,
            false,
        )
        .await
        .unwrap();
        
        // Clear cache
        cache.clear().await.unwrap();
        
        // Check that cache is empty
        let stats = cache.stats().await;
        assert_eq!(stats.entry_count, 0);
        assert_eq!(stats.total_size, 0);
    }
    
    #[tokio::test]
    async fn test_metadata_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let config = CacheConfig {
            dir: temp_dir.path().to_path_buf(),
            max_size: Some(1024),
            max_age: None,
            persist_metadata: true,
        };
        
        // Create cache and add entry
        let cache1 = Cache::new(config.clone()).await.unwrap();
        cache1.put(
            "https://example.com/test.txt".to_string(),
            b"Hello, World!",
            Some("\"etag123\"".to_string()),
            None,
            None,
            false,
        )
        .await
        .unwrap();
        drop(cache1);
        
        // Create new cache instance and check that entry is loaded
        let cache2 = Cache::new(config).await.unwrap();
        let entry = cache2.get("https://example.com/test.txt").await.unwrap().unwrap();
        assert_eq!(entry.etag, Some("\"etag123\"".to_string()));
    }
}