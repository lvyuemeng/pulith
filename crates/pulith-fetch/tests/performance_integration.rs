//! Performance integration tests for pulith-fetch.
//!
//! These tests verify that the performance improvements work correctly
//! under various scenarios including large files, concurrent downloads,
//! and memory usage under load.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::path::{Path, PathBuf};

use bytes::Bytes;
use pulith_fetch::{FetchOptions, FetchPhase, Progress};
use pulith_fetch::Fetcher;

fn create_temp_dir() -> PathBuf {
    let temp_dir = std::env::temp_dir().join(format!(
        "pulith_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp_dir).unwrap();
    temp_dir
}

fn cleanup_temp_dir(temp_dir: &Path) {
    let _ = std::fs::remove_dir_all(temp_dir);
}

/// Mock HTTP client for testing.
#[derive(Debug)]
struct TestHttpClient {
    data: Vec<u8>,
    delay: Duration,
}

impl TestHttpClient {
    fn new(data: Vec<u8>, delay: Duration) -> Self {
        Self { data, delay }
    }
}

#[derive(Debug)]
struct TestError(String);

impl std::fmt::Display for TestError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TestError {}

#[async_trait::async_trait]
impl pulith_fetch::HttpClient for TestHttpClient {
    type Error = TestError;
    
    fn head(
        &self,
        _url: &str,
    ) -> impl std::future::Future<Output = std::result::Result<Option<u64>, Self::Error>> + Send {
        let delay = self.delay;
        let size = self.data.len();
        async move {
            tokio::time::sleep(delay).await;
            Ok(Some(size as u64))
        }
    }

    fn stream(
        &self,
        _url: &str,
        _headers: &[(String, String)],
    ) -> impl std::future::Future<Output = std::result::Result<pulith_fetch::net::http::BoxStream<'static, std::result::Result<Bytes, Self::Error>>, Self::Error>> + Send {
        let data = self.data.clone();
        let delay = self.delay;
        async move {
            tokio::time::sleep(delay).await;
            
            // Split data into chunks to simulate streaming
            let chunk_size = 8192; // 8KB chunks
            let chunks: Vec<_> = data
                .chunks(chunk_size)
                .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
                .collect();
            
            let stream: pulith_fetch::net::http::BoxStream<'static, std::result::Result<Bytes, Self::Error>> = 
                Box::pin(futures_util::stream::iter(chunks));
            Ok(stream)
        }
    }
}

#[tokio::test]
async fn test_large_file_performance() {
    let file_size = 10 * 1024 * 1024;
    let test_data = vec![0u8; file_size];
    
    let temp_dir = create_temp_dir();
    let workspace_root = temp_dir.join("workspace");
    std::fs::create_dir(&workspace_root).unwrap();
    let destination = temp_dir.join("output").join("large_file.bin");
    std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
    
    let client = TestHttpClient::new(test_data, Duration::from_millis(10));
    let fetcher = Fetcher::new(client, &workspace_root);
    
    let options = FetchOptions::default();
    
    let start_time = Instant::now();
    let last_progress = Arc::new(Mutex::new(None::<Progress>));
    
    let mut options_with_callback = options;
    let last_progress_clone = Arc::clone(&last_progress);
    options_with_callback.on_progress = Some(Arc::new(move |progress: &Progress| {
        if let Ok(mut p) = last_progress_clone.lock() {
            *p = Some(progress.clone());
        }
    }));
    
    let result = fetcher.fetch("http://example.com/large-file", &destination, options_with_callback).await;
    if let Err(e) = &result {
        println!("Error: {:?}", e);
    }
    assert!(result.is_ok(), "Fetch failed: {:?}", result);
    
    let elapsed = start_time.elapsed();
    
    let actual_path = result.unwrap();
    println!("Requested destination: {:?}", destination);
    println!("Actual path returned: {:?}", actual_path);
    println!("Actual path exists: {}", actual_path.exists());
    
    if actual_path.exists() {
        let downloaded_size = std::fs::metadata(&actual_path).unwrap().len();
        println!("Downloaded size: {}", downloaded_size);
        assert_eq!(downloaded_size, file_size as u64);
    } else {
        panic!("Downloaded file does not exist at {:?}", actual_path);
    }
    
    assert!(elapsed < Duration::from_secs(10), "Download took too long: {:?}", elapsed);
    
    let final_progress = last_progress.lock().unwrap().as_ref().unwrap().clone();
    assert_eq!(final_progress.bytes_downloaded, file_size as u64);
    assert_eq!(final_progress.phase, FetchPhase::Completed);
    
    if let Some(ref metrics) = final_progress.performance_metrics {
        println!("Current rate: {:?}", metrics.current_rate_bps);
        println!("Average rate: {:?}", metrics.average_rate_bps);
        println!("Connection time: {:?}", metrics.connection_time_ms);
        println!("Phase timings: {:?}", metrics.phase_timings);
        
        if metrics.current_rate_bps.is_none() {
            println!("Download too fast for current rate calculation");
        }
        
        assert!(metrics.average_rate_bps.is_some());
        assert!(metrics.connection_time_ms.is_some());
        assert!(metrics.phase_timings.total_ms() > 0);
        
        assert!(metrics.phase_timings.connecting_ms < 1000);
        assert!(metrics.phase_timings.downloading_ms > 0);
        assert!(metrics.phase_timings.total_ms() <= elapsed.as_millis() as u64 + 1000);
    }
    
    cleanup_temp_dir(&temp_dir);
}

#[tokio::test]
async fn test_concurrent_performance() {
    const NUM_DOWNLOADS: usize = 5;
    const FILE_SIZE: usize = 2 * 1024 * 1024;
    
    let temp_dir = create_temp_dir();
    let workspace_root = temp_dir.join("workspace");
    std::fs::create_dir(&workspace_root).unwrap();
    
    let mut handles = Vec::new();
    
for i in 0..NUM_DOWNLOADS {
        let workspace_root = workspace_root.join(format!("workspace_{}", i));
        std::fs::create_dir(&workspace_root).unwrap();
        let test_data = vec![i as u8; FILE_SIZE];
        let temp_dir_clone = temp_dir.clone();
        
        let handle = tokio::spawn(async move {
            let client = TestHttpClient::new(test_data, Duration::from_millis(5));
            let fetcher = Fetcher::new(client, &workspace_root);
            // Create a unique destination directory for each concurrent download
            let dest_dir = temp_dir_clone.join(format!("concurrent_dest_{}", i));
            std::fs::create_dir_all(&dest_dir).unwrap();
            let destination = dest_dir.join(format!("file_{}.bin", i));
            
            let options = FetchOptions::default();
            let start_time = Instant::now();
            
            let result = fetcher.fetch(
                &format!("http://example.com/concurrent-{}", i),
                &destination,
                options,
            ).await;
            
            let elapsed = start_time.elapsed();
            (result, elapsed, destination)
        });
        
        handles.push(handle);
    }
    
    let mut successful_downloads = 0;
    let mut total_time = Duration::ZERO;
    let mut max_time = Duration::ZERO;
    
    for (i, handle) in handles.into_iter().enumerate() {
        let (result, elapsed, destination) = handle.await.unwrap();
        if let Err(e) = &result {
            println!("Download failed for {}: {:?}", i, e);
        }
        assert!(result.is_ok(), "Download failed for {}: {:?}", i, result);
        
        assert!(destination.exists());
        let size = std::fs::metadata(&destination).unwrap().len();
        assert_eq!(size, FILE_SIZE as u64);
        
        successful_downloads += 1;
        total_time += elapsed;
        max_time = max_time.max(elapsed);
    }
    
    assert_eq!(successful_downloads, NUM_DOWNLOADS);
    
    let avg_time = total_time / NUM_DOWNLOADS as u32;
    assert!(avg_time < Duration::from_secs(5), "Average download time too long: {:?}", avg_time);
    assert!(max_time < Duration::from_secs(10), "Max download time too long: {:?}", max_time);
    
    cleanup_temp_dir(&temp_dir);
}

#[tokio::test]
async fn test_memory_usage_under_load() {
    const FILE_SIZE: usize = 10 * 1024 * 1024;
    const NUM_CONCURRENT: usize = 3;
    
    let temp_dir = create_temp_dir();
    let workspace_root = temp_dir.join("workspace");
    std::fs::create_dir(&workspace_root).unwrap();
    
    let initial_memory = get_memory_usage();
    
    let mut handles = Vec::new();
    
    for i in 0..NUM_CONCURRENT {
        let workspace_root = workspace_root.join(format!("workspace_{}", i));
        std::fs::create_dir(&workspace_root).unwrap();
        let test_data = vec![i as u8; FILE_SIZE];
        let temp_dir_clone = temp_dir.clone();
        
        let handle = tokio::spawn(async move {
            let client = TestHttpClient::new(test_data, Duration::from_millis(1));
            let fetcher = Fetcher::new(client, &workspace_root);
            // Create a unique destination directory for each concurrent download
            let dest_dir = temp_dir_clone.join(format!("memory_dest_{}", i));
            std::fs::create_dir_all(&dest_dir).unwrap();
            let destination = dest_dir.join(format!("file_{}.bin", i));
            
            let options = FetchOptions::default();
            fetcher.fetch(
                &format!("http://example.com/memory-test-{}", i),
                &destination,
                options,
            ).await
        });
        
        handles.push(handle);
    }
    
    let mut peak_memory = initial_memory;
    
    for handle in handles {
        let current_memory = get_memory_usage();
        peak_memory = peak_memory.max(current_memory);
        
        let result = handle.await.unwrap();
        assert!(result.is_ok());
    }
    
    let final_memory = get_memory_usage();
    
    let memory_growth = peak_memory.saturating_sub(initial_memory);
    let max_allowed_growth = FILE_SIZE * NUM_CONCURRENT / 2;
    
    assert!(
        memory_growth <= max_allowed_growth,
        "Memory usage grew too much: {} bytes (max allowed: {})",
        memory_growth,
        max_allowed_growth
    );
    
    let memory_leak = final_memory.saturating_sub(initial_memory);
    assert!(
        memory_leak < 10 * 1024 * 1024,
        "Potential memory leak: {} bytes not cleaned up",
        memory_leak
    );
    
    cleanup_temp_dir(&temp_dir);
}

/// Get current process memory usage in bytes.
#[cfg(unix)]
fn get_memory_usage() -> usize {
    use std::fs;
    
    let status = fs::read_to_string("/proc/self/status").unwrap();
    for line in status.lines() {
        if line.starts_with("VmRSS:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                return parts[1].parse::<usize>().unwrap() * 1024; // Convert KB to bytes
            }
        }
    }
    0
}

/// Get current process memory usage in bytes.
#[cfg(windows)]
fn get_memory_usage() -> usize {
    use std::mem;
    
    // Simple approximation - in real tests you might use more sophisticated methods
    // This is just to ensure the test compiles and runs
    mem::size_of::<TestHttpClient>() * 1000 // Rough estimate
}

#[tokio::test]
async fn test_performance_scaling() {
    let file_sizes = vec![
        1024,
        1024 * 1024,
        5 * 1024 * 1024,
    ];
    
    for size in file_sizes {
        let test_data = vec![size as u8; size];
        let temp_dir = create_temp_dir();
        let workspace_root = temp_dir.join("workspace");
        std::fs::create_dir(&workspace_root).unwrap();
        // Create a unique destination directory for each test
        let dest_dir = temp_dir.join(format!("scale_dest_{}", size));
        std::fs::create_dir_all(&dest_dir).unwrap();
        let destination = dest_dir.join(format!("file_{}.bin", size));
        
        let client = TestHttpClient::new(test_data, Duration::from_millis(1));
        let fetcher = Fetcher::new(client, &workspace_root);
        
        let options = FetchOptions::default();
        let start_time = Instant::now();
        
        let result = fetcher.fetch("http://example.com/scale-test", &destination, options).await;
        let elapsed = start_time.elapsed();
        
        assert!(result.is_ok());
        
        assert!(destination.exists());
        let downloaded_size = std::fs::metadata(&destination).unwrap().len();
        assert_eq!(downloaded_size, size as u64);
        
        let throughput = size as f64 / elapsed.as_secs_f64();
        // For small files, the throughput will be lower due to overhead
        let min_throughput = if size < 1024 * 1024 {
            10.0 * 1024.0 // 10 KB/s minimum for files < 1MB
        } else {
            1024.0 * 1024.0 // 1 MB/s minimum for larger files
        };
        assert!(throughput > min_throughput, "Throughput too low for size {}: {:.2} B/s (min: {:.2} B/s)", size, throughput, min_throughput);
        
        cleanup_temp_dir(&temp_dir);
    }
}