//! Performance measurement utilities for pulith-fetch.
//!
//! This module provides tools for measuring and tracking performance metrics
//! including memory usage, throughput, and timing measurements.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Memory usage tracker for monitoring allocation patterns.
#[derive(Debug, Default)]
pub struct MemoryTracker {
    /// Total bytes allocated
    pub total_allocated: AtomicU64,
    /// Peak memory usage in bytes
    pub peak_usage: AtomicU64,
    /// Current allocation count
    pub allocation_count: AtomicU64,
}

impl MemoryTracker {
    /// Create a new memory tracker.
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Record an allocation of the specified size.
    pub fn record_allocation(&self, size: u64) {
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
        self.allocation_count.fetch_add(1, Ordering::Relaxed);

        let current = self.total_allocated.load(Ordering::Relaxed);
        let mut peak = self.peak_usage.load(Ordering::Relaxed);
        while current > peak {
            match self.peak_usage.compare_exchange_weak(
                peak,
                current,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => break,
                Err(actual) => peak = actual,
            }
        }
    }

    /// Record a deallocation of the specified size.
    pub fn record_deallocation(&self, size: u64) {
        self.total_allocated.fetch_sub(size, Ordering::Relaxed);
    }

    /// Get current memory usage statistics.
    pub fn get_stats(&self) -> MemoryStats {
        MemoryStats {
            current_usage: self.total_allocated.load(Ordering::Relaxed),
            peak_usage: self.peak_usage.load(Ordering::Relaxed),
            allocation_count: self.allocation_count.load(Ordering::Relaxed),
        }
    }

    /// Reset all statistics.
    pub fn reset(&self) {
        self.total_allocated.store(0, Ordering::Relaxed);
        self.peak_usage.store(0, Ordering::Relaxed);
        self.allocation_count.store(0, Ordering::Relaxed);
    }
}

/// Memory usage statistics.
#[derive(Debug, Clone)]
pub struct MemoryStats {
    /// Current memory usage in bytes
    pub current_usage: u64,
    /// Peak memory usage in bytes
    pub peak_usage: u64,
    /// Total number of allocations
    pub allocation_count: u64,
}

/// Throughput measurement helper for tracking data transfer rates.
#[derive(Debug)]
pub struct ThroughputMeter {
    start_time: Instant,
    bytes_transferred: Arc<AtomicU64>,
}

impl ThroughputMeter {
    /// Create a new throughput meter.
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            bytes_transferred: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Record bytes transferred.
    pub fn record_bytes(&self, bytes: u64) {
        self.bytes_transferred.fetch_add(bytes, Ordering::Relaxed);
    }

    /// Get current throughput in bytes per second.
    pub fn current_throughput(&self) -> f64 {
        let bytes = self.bytes_transferred.load(Ordering::Relaxed);
        let elapsed = self.start_time.elapsed().as_secs_f64();
        if elapsed > 0.0 {
            bytes as f64 / elapsed
        } else {
            0.0
        }
    }

    /// Get total bytes transferred.
    pub fn total_bytes(&self) -> u64 {
        self.bytes_transferred.load(Ordering::Relaxed)
    }

    /// Get elapsed time since creation.
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Reset the meter.
    pub fn reset(&self) {
        self.bytes_transferred.store(0, Ordering::Relaxed);
    }
}

/// Timing measurement utility for benchmarking operations.
#[derive(Debug)]
pub struct Timer {
    start_time: Option<Instant>,
    total_duration: Arc<AtomicU64>,
}

impl Timer {
    /// Create a new timer.
    pub fn new() -> Self {
        Self {
            start_time: None,
            total_duration: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Start timing.
    pub fn start(&mut self) {
        self.start_time = Some(Instant::now());
    }

    /// Stop timing and return the elapsed duration.
    pub fn stop(&mut self) -> Duration {
        if let Some(start) = self.start_time {
            let elapsed = start.elapsed();
            self.total_duration
                .fetch_add(elapsed.as_nanos() as u64, Ordering::Relaxed);
            self.start_time = None;
            elapsed
        } else {
            Duration::ZERO
        }
    }

    /// Get the total accumulated duration.
    pub fn total_duration(&self) -> Duration {
        Duration::from_nanos(self.total_duration.load(Ordering::Relaxed))
    }

    /// Check if the timer is currently running.
    pub fn is_running(&self) -> bool {
        self.start_time.is_some()
    }

    /// Reset the timer.
    pub fn reset(&self) {
        self.total_duration.store(0, Ordering::Relaxed);
    }
}

/// Performance profiler that combines multiple measurement tools.
#[derive(Debug)]
pub struct Profiler {
    memory_tracker: Arc<MemoryTracker>,
    throughput_meter: Arc<ThroughputMeter>,
    timer: Arc<Timer>,
}

impl Profiler {
    /// Create a new profiler.
    pub fn new() -> Self {
        Self {
            memory_tracker: MemoryTracker::new(),
            throughput_meter: Arc::new(ThroughputMeter::new()),
            timer: Arc::new(Timer::new()),
        }
    }

    /// Get a reference to the memory tracker.
    pub fn memory_tracker(&self) -> &Arc<MemoryTracker> {
        &self.memory_tracker
    }

    /// Get a reference to the throughput meter.
    pub fn throughput_meter(&self) -> &Arc<ThroughputMeter> {
        &self.throughput_meter
    }

    /// Get a reference to the timer.
    pub fn timer(&self) -> &Arc<Timer> {
        &self.timer
    }

    /// Get a comprehensive performance report.
    pub fn get_report(&self) -> PerformanceReport {
        PerformanceReport {
            memory_stats: self.memory_tracker.get_stats(),
            throughput_bps: self.throughput_meter.current_throughput(),
            total_bytes: self.throughput_meter.total_bytes(),
            elapsed_time: self.throughput_meter.elapsed(),
            total_duration: self.timer.total_duration(),
        }
    }

    /// Reset all measurements.
    pub fn reset(&self) {
        self.memory_tracker.reset();
        self.throughput_meter.reset();
        self.timer.reset();
    }
}

/// Comprehensive performance report.
#[derive(Debug, Clone)]
pub struct PerformanceReport {
    /// Memory usage statistics
    pub memory_stats: MemoryStats,
    /// Current throughput in bytes per second
    pub throughput_bps: f64,
    /// Total bytes transferred
    pub total_bytes: u64,
    /// Total elapsed time
    pub elapsed_time: Duration,
    /// Accumulated timing duration
    pub total_duration: Duration,
}

impl PerformanceReport {
    /// Get throughput in human-readable format (MB/s).
    pub fn throughput_mbps(&self) -> f64 {
        self.throughput_bps / (1024.0 * 1024.0)
    }

    /// Get memory usage in human-readable format (MB).
    pub fn memory_usage_mb(&self) -> f64 {
        self.memory_stats.current_usage as f64 / (1024.0 * 1024.0)
    }

    /// Get peak memory usage in human-readable format (MB).
    pub fn peak_memory_mb(&self) -> f64 {
        self.memory_stats.peak_usage as f64 / (1024.0 * 1024.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new();

        tracker.record_allocation(1024);
        tracker.record_allocation(2048);

        let stats = tracker.get_stats();
        assert_eq!(stats.current_usage, 3072);
        assert_eq!(stats.allocation_count, 2);
        assert_eq!(stats.peak_usage, 3072);

        tracker.record_deallocation(1024);
        let stats = tracker.get_stats();
        assert_eq!(stats.current_usage, 2048);
        assert_eq!(stats.peak_usage, 3072);
    }

    #[test]
    fn test_throughput_meter() {
        let meter = ThroughputMeter::new();

        meter.record_bytes(1024);
        thread::sleep(Duration::from_millis(10));
        meter.record_bytes(1024);

        assert_eq!(meter.total_bytes(), 2048);
        assert!(meter.current_throughput() > 0.0);
        assert!(meter.elapsed() > Duration::ZERO);
    }

    #[test]
    fn test_timer() {
        let mut timer = Timer::new();

        assert!(!timer.is_running());
        assert_eq!(timer.total_duration(), Duration::ZERO);

        timer.start();
        assert!(timer.is_running());

        thread::sleep(Duration::from_millis(10));
        let elapsed = timer.stop();

        assert!(!timer.is_running());
        assert!(elapsed > Duration::ZERO);
        assert_eq!(timer.total_duration(), elapsed);
    }

    #[test]
    fn test_profiler() {
        let profiler = Profiler::new();

        profiler.memory_tracker().record_allocation(1024);
        profiler.throughput_meter().record_bytes(2048);

        let report = profiler.get_report();
        assert_eq!(report.memory_stats.current_usage, 1024);
        assert_eq!(report.total_bytes, 2048);
        assert!(report.elapsed_time >= Duration::ZERO);
    }

    #[test]
    fn test_performance_report_formatting() {
        let report = PerformanceReport {
            memory_stats: MemoryStats {
                current_usage: 1024 * 1024,
                peak_usage: 2 * 1024 * 1024,
                allocation_count: 10,
            },
            throughput_bps: (10 * 1024 * 1024) as f64,
            total_bytes: 5 * 1024 * 1024,
            elapsed_time: Duration::from_secs(1),
            total_duration: Duration::from_millis(500),
        };

        assert_eq!(report.throughput_mbps(), 10.0);
        assert_eq!(report.memory_usage_mb(), 1.0);
        assert_eq!(report.peak_memory_mb(), 2.0);
    }
}
