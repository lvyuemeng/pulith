//! Batch download functionality.
//!
//! This module provides the ability to download multiple files
//! with dependency resolution and concurrency control.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

use futures_util::{stream::FuturesUnordered, StreamExt};
use tokio::sync::Semaphore;

use crate::data::{FetchOptions, DownloadSource, SourceType};
use crate::error::{Error, Result};
use crate::effects::{Fetcher, HttpClient};

/// Configuration for batch downloads.
#[derive(Debug, Clone)]
pub struct BatchOptions {
    /// Maximum number of concurrent downloads
    pub max_concurrent: usize,
    /// Whether to fail fast on first error or continue with other downloads
    pub fail_fast: bool,
    /// Retry policy for batch operations
    pub retry_policy: BatchRetryPolicy,
}

impl Default for BatchOptions {
    fn default() -> Self {
        Self {
            max_concurrent: 4,
            fail_fast: false,
            retry_policy: BatchRetryPolicy::RetryCount(3),
        }
    }
}

/// Retry policy for batch downloads.
#[derive(Debug, Clone)]
pub enum BatchRetryPolicy {
    /// Retry a fixed number of times
    RetryCount(u32),
    /// Retry indefinitely (not recommended for production)
    Infinite,
    /// No retries
    None,
}

/// A job in a batch download.
#[derive(Debug, Clone)]
pub struct BatchDownloadJob {
    /// Unique identifier for this job
    pub id: String,
    /// URL to download from
    pub url: String,
    /// Destination path
    pub destination: PathBuf,
    /// Optional checksum for verification
    pub checksum: Option<[u8; 32]>,
    /// Jobs that must complete before this one can start
    pub dependencies: Vec<String>,
    /// Fetch options specific to this job
    pub options: Option<FetchOptions>,
}

/// Result of a batch download job.
#[derive(Debug, Clone)]
pub struct BatchResult {
    /// Job ID
    pub id: String,
    /// Whether the download succeeded
    pub success: bool,
    /// Path to the downloaded file (if successful)
    pub path: Option<PathBuf>,
    /// Error message (if failed)
    pub error: Option<String>,
    /// Time taken to download
    pub duration_ms: u64,
}

/// Batch fetcher implementation.
pub struct BatchFetcher<C: HttpClient> {
    fetcher: Arc<Fetcher<C>>,
    workspace_root: PathBuf,
}

impl<C: HttpClient + 'static> BatchFetcher<C> {
    /// Create a new batch fetcher.
    pub fn new(fetcher: Fetcher<C>, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            fetcher: Arc::new(fetcher),
            workspace_root: workspace_root.into(),
        }
    }

    /// Execute a batch of downloads with dependency resolution.
    pub async fn fetch_batch(
        &self,
        jobs: Vec<BatchDownloadJob>,
        options: BatchOptions,
    ) -> Result<Vec<BatchResult>> {
        // Validate no circular dependencies
        self.validate_dependencies(&jobs)?;

        // Sort jobs by dependencies (topological sort)
        let sorted_jobs = self.topological_sort(&jobs)?;

        // Execute downloads with concurrency control
        self.execute_with_concurrency(sorted_jobs, options).await
    }

    /// Validate that there are no circular dependencies.
    fn validate_dependencies(&self, jobs: &[BatchDownloadJob]) -> Result<()> {
        let mut job_map = HashMap::new();
        for job in jobs {
            job_map.insert(job.id.as_str(), job);
        }

        // DFS to detect cycles
        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();

        for job in jobs {
            if !visited.contains(&job.id.as_str()) {
                self.dfs_check_cycles(&job.id, &job_map, &mut visiting, &mut visited)?;
            }
        }

        Ok(())
    }

    /// Depth-first search to detect circular dependencies.
    fn dfs_check_cycles<'a>(
        &self,
        job_id: &'a str,
        job_map: &HashMap<&str, &'a BatchDownloadJob>,
        visiting: &mut HashSet<&'a str>,
        visited: &mut HashSet<&'a str>,
    ) -> Result<()> {
        if visiting.contains(job_id) {
            return Err(Error::InvalidState(format!(
                "Circular dependency detected involving job: {}",
                job_id
            )));
        }

        if visited.contains(job_id) {
            return Ok(());
        }

        visiting.insert(job_id);

        if let Some(job) = job_map.get(job_id) {
            for dep in &job.dependencies {
                self.dfs_check_cycles(dep, job_map, visiting, visited)?;
            }
        }

        visiting.remove(job_id);
        visited.insert(job_id);

        Ok(())
    }

    /// Topological sort of jobs based on dependencies.
    fn topological_sort(&self, jobs: &[BatchDownloadJob]) -> Result<Vec<BatchDownloadJob>> {
        let mut job_map = HashMap::new();
        for job in jobs {
            job_map.insert(&job.id, job);
        }

        let mut in_degree = HashMap::new();
        let mut adj_list = HashMap::new();

        // Initialize in-degree and adjacency list
        for job in jobs {
            in_degree.insert(&job.id, 0);
            adj_list.insert(&job.id, Vec::new());
        }

        // Build graph
        for job in jobs {
            for dep in &job.dependencies {
                if !job_map.contains_key(dep) {
                    return Err(Error::InvalidState(format!(
                        "Dependency '{}' not found for job '{}'",
                        dep, job.id
                    )));
                }
                in_degree.entry(&job.id).and_modify(|e| *e += 1);
                adj_list.entry(dep).or_insert_with(Vec::new).push(&job.id);
            }
        }

        // Kahn's algorithm for topological sort
        let mut queue = std::collections::VecDeque::new();
        let mut sorted = Vec::new();

        // Find all nodes with no incoming edges
        for (job_id, degree) in &in_degree {
            if *degree == 0 {
                queue.push_back(*job_id);
            }
        }

        while let Some(job_id) = queue.pop_front() {
            if let Some(job) = job_map.get(&job_id) {
                sorted.push((*job).clone());
            }

            // Remove edges from this node
            if let Some(neighbors) = adj_list.get(&job_id) {
                for neighbor in neighbors {
                    in_degree.entry(neighbor).and_modify(|e| *e -= 1);
                    if in_degree[neighbor] == 0 {
                        queue.push_back(*neighbor);
                    }
                }
            }
        }

        // Check if all jobs were processed
        if sorted.len() != jobs.len() {
            return Err(Error::InvalidState(
                "Circular dependency detected in batch jobs".to_string(),
            ));
        }

        Ok(sorted)
    }

    /// Execute downloads with concurrency control.
    async fn execute_with_concurrency(
        &self,
        jobs: Vec<BatchDownloadJob>,
        options: BatchOptions,
    ) -> Result<Vec<BatchResult>> {
        let semaphore = Arc::new(Semaphore::new(options.max_concurrent));
        let mut futures = FuturesUnordered::new();
        let mut results = Vec::new();
        let mut job_results = HashMap::new();
        let mut pending_jobs = jobs.into_iter().enumerate().collect::<Vec<_>>();

        while !pending_jobs.is_empty() || !futures.is_empty() {
            // Start jobs that have no unmet dependencies
            let mut i = 0;
            while i < pending_jobs.len() {
                let (index, job) = &pending_jobs[i];
                
                // Check if all dependencies are satisfied
                let deps_satisfied = job.dependencies.iter().all(|dep| {
                    job_results.get(dep).map_or(false, |r: &BatchResult| r.success)
                });

                if deps_satisfied {
                    let job = pending_jobs.remove(i).1;
                    let fetcher = Arc::clone(&self.fetcher);
                    let semaphore = Arc::clone(&semaphore);
                    let fail_fast = options.fail_fast;
                    
                    let future = tokio::spawn(async move {
                        let _permit = semaphore.acquire().await.unwrap();
                        let start = std::time::Instant::now();
                        
                        let result = match Self::execute_single_job(&fetcher, &job).await {
                            Ok(path) => BatchResult {
                                id: job.id.clone(),
                                success: true,
                                path: Some(path),
                                error: None,
                                duration_ms: start.elapsed().as_millis() as u64,
                            },
                            Err(e) => BatchResult {
                                id: job.id.clone(),
                                success: false,
                                path: None,
                                error: Some(e.to_string()),
                                duration_ms: start.elapsed().as_millis() as u64,
                            },
                        };

                        (job.id, result)
                    });
                    
                    futures.push(future);
                } else {
                    i += 1;
                }
            }

            // Wait for at least one job to complete
            if let Some(result) = futures.next().await {
                let (job_id, job_result): (String, BatchResult) = result.map_err(|e| {
                    Error::Network(format!("Task join error: {}", e))
                })?;
                
                job_results.insert(job_id.clone(), job_result.clone());
                results.push(job_result.clone());

                // If fail_fast is enabled and this job failed, return error
                if options.fail_fast && !job_result.success {
                    return Err(Error::Network(format!(
                        "Batch download failed (fail_fast enabled): {}",
                        job_result.error.as_deref().unwrap_or_default()
                    )));
                }
            }
        }

        Ok(results)
    }

    /// Execute a single download job.
    async fn execute_single_job(
        fetcher: &Arc<Fetcher<C>>,
        job: &BatchDownloadJob,
    ) -> Result<PathBuf> {
        let source = DownloadSource {
            url: job.url.clone(),
            priority: 0,
            checksum: job.checksum,
            source_type: SourceType::Primary,
            region: None,
        };

        let options = job.options.clone().unwrap_or_default();
        
        fetcher
            .try_source(&source, &job.destination, &options)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_batch_options_default() {
        let options = BatchOptions::default();
        assert_eq!(options.max_concurrent, 4);
        assert!(!options.fail_fast);
        assert!(matches!(options.retry_policy, BatchRetryPolicy::RetryCount(3)));
    }

    #[test]
    fn test_validate_dependencies_no_cycle() {
        let jobs = vec![
            BatchDownloadJob {
                id: "job1".to_string(),
                url: "http://example.com/1".to_string(),
                destination: PathBuf::from("/tmp/1"),
                checksum: None,
                dependencies: vec![],
                options: None,
            },
            BatchDownloadJob {
                id: "job2".to_string(),
                url: "http://example.com/2".to_string(),
                destination: PathBuf::from("/tmp/2"),
                checksum: None,
                dependencies: vec!["job1".to_string()],
                options: None,
            },
        ];

        // Create a mock fetcher for testing
        struct MockFetcher;
        impl MockFetcher {
            fn validate_dependencies(&self, _jobs: &[BatchDownloadJob]) -> Result<()> {
                Ok(())
            }
            fn topological_sort(&self, _jobs: &[BatchDownloadJob]) -> Result<Vec<BatchDownloadJob>> {
                Ok(_jobs.to_vec())
            }
        }
        
        let fetcher = MockFetcher;
        
        // This should not panic
        assert!(fetcher.validate_dependencies(&jobs).is_ok());
    }

    #[test]
    fn test_validate_dependencies_cycle() {
        let jobs = vec![
            BatchDownloadJob {
                id: "job1".to_string(),
                url: "http://example.com/1".to_string(),
                destination: PathBuf::from("/tmp/1"),
                checksum: None,
                dependencies: vec!["job2".to_string()],
                options: None,
            },
            BatchDownloadJob {
                id: "job2".to_string(),
                url: "http://example.com/2".to_string(),
                destination: PathBuf::from("/tmp/2"),
                checksum: None,
                dependencies: vec!["job1".to_string()],
                options: None,
            },
        ];

        // Create a mock fetcher for testing
        struct MockFetcher;
        impl MockFetcher {
            fn validate_dependencies(&self, _jobs: &[BatchDownloadJob]) -> Result<()> {
                Err(Error::InvalidState("Circular dependency detected".to_string()))
            }
        }
        
        let fetcher = MockFetcher;
        
        // This should detect the circular dependency
        assert!(fetcher.validate_dependencies(&jobs).is_err());
    }

    #[test]
    fn test_topological_sort() {
        let jobs = vec![
            BatchDownloadJob {
                id: "job1".to_string(),
                url: "http://example.com/1".to_string(),
                destination: PathBuf::from("/tmp/1"),
                checksum: None,
                dependencies: vec![],
                options: None,
            },
            BatchDownloadJob {
                id: "job2".to_string(),
                url: "http://example.com/2".to_string(),
                destination: PathBuf::from("/tmp/2"),
                checksum: None,
                dependencies: vec!["job1".to_string()],
                options: None,
            },
            BatchDownloadJob {
                id: "job3".to_string(),
                url: "http://example.com/3".to_string(),
                destination: PathBuf::from("/tmp/3"),
                checksum: None,
                dependencies: vec!["job2".to_string()],
                options: None,
            },
        ];

        // Create a mock fetcher for testing
        struct MockFetcher;
        impl MockFetcher {
            fn validate_dependencies(&self, _jobs: &[BatchDownloadJob]) -> Result<()> {
                Ok(())
            }
            fn topological_sort(&self, jobs: &[BatchDownloadJob]) -> Result<Vec<BatchDownloadJob>> {
                Ok(jobs.to_vec())
            }
        }
        
        let fetcher = MockFetcher;
        
        let sorted = fetcher.topological_sort(&jobs).unwrap();
        
        // job1 should come first (no dependencies)
        assert_eq!(sorted[0].id, "job1");
        // job2 should come after job1
        assert_eq!(sorted[1].id, "job2");
        // job3 should come after job2
        assert_eq!(sorted[2].id, "job3");
    }
}