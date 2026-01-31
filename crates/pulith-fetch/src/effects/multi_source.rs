//! Multi-source download functionality.
//!
//! This module provides the ability to download from multiple sources
//! with different strategies for source selection and fallback.

use std::sync::Arc;
use futures_util::stream::{StreamExt, FuturesUnordered};

use crate::data::{DownloadSource, MultiSourceOptions, SourceSelectionStrategy};
use crate::error::{Error, Result};
use crate::effects::fetcher::Fetcher;
use crate::effects::http::HttpClient;

/// Multi-source fetcher implementation.
pub struct MultiSourceFetcher<C: HttpClient> {
    fetcher: Arc<Fetcher<C>>,
}

impl<C: HttpClient + 'static> MultiSourceFetcher<C> {
    /// Create a new multi-source fetcher.
    pub fn new(fetcher: Arc<Fetcher<C>>) -> Self {
        Self { fetcher }
    }

    /// Fetch from multiple sources using the specified strategy.
    pub async fn fetch_multi_source(
        &self,
        sources: Vec<DownloadSource>,
        destination: &std::path::Path,
        options: MultiSourceOptions,
    ) -> Result<std::path::PathBuf> {
        if sources.is_empty() {
            return Err(Error::InvalidState("No sources provided".into()));
        }

        match options.strategy {
            SourceSelectionStrategy::Priority => self.fetch_priority(sources, destination, options).await,
            SourceSelectionStrategy::RaceAll => self.fetch_race(sources, destination, options).await,
            SourceSelectionStrategy::FastestFirst => self.fetch_fastest(sources, destination, options).await,
            SourceSelectionStrategy::Geographic => self.fetch_geographic(sources, destination, options).await,
        }
    }

    /// Try sources in priority order until one succeeds.
    async fn fetch_priority(
        &self,
        mut sources: Vec<DownloadSource>,
        destination: &std::path::Path,
        _options: MultiSourceOptions,
    ) -> Result<std::path::PathBuf> {
        // Sort by priority (lower number = higher priority)
        sources.sort_by_key(|s| s.priority);
        let source_count = sources.len();

        for source in &sources {
            let fetch_options = crate::data::FetchOptions::default()
                .checksum(source.checksum);
            
            match self.try_source(source, destination, &fetch_options).await {
                Ok(path) => return Ok(path),
                Err(_) => continue,
            }
        }

        Err(Error::MaxRetriesExceeded { count: source_count as u32 })
    }

    /// Try all sources in parallel and use the first successful result.
    async fn fetch_race(
        &self,
        sources: Vec<DownloadSource>,
        destination: &std::path::Path,
        _options: MultiSourceOptions,
    ) -> Result<std::path::PathBuf> {
        let mut futures = FuturesUnordered::new();
        let sources_count = sources.len() as u32;

        for source in sources {
            let fetcher = Arc::clone(&self.fetcher);
            let dest = destination.to_path_buf();
            let fetch_options = crate::data::FetchOptions::default()
                .checksum(source.checksum);
            
            futures.push(tokio::spawn(async move {
                fetcher.try_source(&source, &dest, &fetch_options).await
            }));
        }

        // Wait for the first successful result
        while let Some(result) = futures.next().await {
            match result {
                Ok(inner_result) => match inner_result {
                    Ok(path) => return Ok(path),
                    Err(_) => continue,
                },
                Err(_) => continue,
            }
        }

        Err(Error::MaxRetriesExceeded { count: sources_count })
    }

    /// Try sources in order of fastest response time.
    async fn fetch_fastest(
        &self,
        sources: Vec<DownloadSource>,
        destination: &std::path::Path,
        options: MultiSourceOptions,
    ) -> Result<std::path::PathBuf> {
        // For now, fall back to priority-based selection
        // In a real implementation, we would measure response times
        self.fetch_priority(sources, destination, options).await
    }

    /// Try sources based on geographic proximity.
    async fn fetch_geographic(
        &self,
        sources: Vec<DownloadSource>,
        destination: &std::path::Path,
        options: MultiSourceOptions,
    ) -> Result<std::path::PathBuf> {
        // For now, fall back to priority-based selection
        // In a real implementation, we would use IP geolocation
        self.fetch_priority(sources, destination, options).await
    }

    /// Try a single source with verification.
    async fn try_source(
        &self,
        source: &DownloadSource,
        destination: &std::path::Path,
        options: &crate::data::FetchOptions,
    ) -> Result<std::path::PathBuf> {
            // Create fetch options for this source
            let mut fetch_options = options.clone();
            fetch_options.checksum = source.checksum;

            // Fetch using the base fetcher
            self.fetcher.fetch(&source.url, destination, fetch_options).await
        }
}
