use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulith_fetch::core::calculate_segments;
use pulith_fetch::data::sources::DownloadSource;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;

// Simple memory tracking using std::alloc
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering};

static ALLOCATED: AtomicU64 = AtomicU64::new(0);

struct MemoryTracker;

unsafe impl GlobalAlloc for MemoryTracker {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = System.alloc(layout);
        if !ptr.is_null() {
            ALLOCATED.fetch_add(layout.size() as u64, Ordering::Relaxed);
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        ALLOCATED.fetch_sub(layout.size() as u64, Ordering::Relaxed);
    }
}

#[global_allocator]
static GLOBAL: MemoryTracker = MemoryTracker;

fn get_memory_usage() -> u64 {
    ALLOCATED.load(Ordering::Relaxed)
}

fn bench_segment_calculation_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("segment_calculation_memory");

    // Reset memory counter before each benchmark
    for file_size in [
        1024 * 1024,
        10 * 1024 * 1024,
        100 * 1024 * 1024,
        1024 * 1024 * 1024,
    ]
    .iter()
    {
        group.throughput(Throughput::Bytes(*file_size));
        group.bench_with_input(
            BenchmarkId::new("calculate_segments", file_size),
            file_size,
            |b, &file_size| {
                b.iter(|| {
                    // Reset memory counter
                    ALLOCATED.store(0, Ordering::Relaxed);

                    let segment_size = 1024 * 1024; // 1MB segments
                    let num_segments = ((file_size / segment_size) as u32).max(1).min(16);

                    let segments =
                        calculate_segments(black_box(file_size), black_box(num_segments)).unwrap();

                    let memory_used = get_memory_usage();
                    (segments, memory_used)
                });
            },
        );
    }

    group.finish();
}

fn bench_large_buffer_allocation(c: &mut Criterion) {
    let mut group = c.benchmark_group("large_buffer_allocation");

    for buffer_size in [1024 * 1024, 10 * 1024 * 1024, 100 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*buffer_size as u64));
        group.bench_with_input(
            BenchmarkId::new("allocate_buffer", buffer_size),
            buffer_size,
            |b, &buffer_size| {
                b.iter(|| {
                    // Reset memory counter
                    ALLOCATED.store(0, Ordering::Relaxed);

                    let buffer = vec![0u8; buffer_size as usize];
                    black_box(buffer);

                    let memory_used = get_memory_usage();
                    memory_used
                });
            },
        );
    }

    group.finish();
}

fn bench_concurrent_downloads_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("concurrent_downloads_memory");

    for concurrent_count in [1, 2, 4, 8, 16].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_buffers", concurrent_count),
            concurrent_count,
            |b, &concurrent_count| {
                b.iter(|| {
                    // Reset memory counter
                    ALLOCATED.store(0, Ordering::Relaxed);

                    let mut buffers = Vec::new();
                    for _ in 0..concurrent_count {
                        // Each buffer is 1MB
                        let buffer = vec![0u8; 1024 * 1024];
                        buffers.push(buffer);
                    }

                    black_box(buffers);

                    let memory_used = get_memory_usage();
                    memory_used
                });
            },
        );
    }

    group.finish();
}

fn bench_stream_processing_memory(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_processing_memory");

    for chunk_size in [1024, 4096, 16384, 65536].iter() {
        group.throughput(Throughput::Bytes(*chunk_size as u64));
        group.bench_with_input(
            BenchmarkId::new("stream_chunk", chunk_size),
            chunk_size,
            |b, &chunk_size| {
                b.iter(|| {
                    // Reset memory counter
                    ALLOCATED.store(0, Ordering::Relaxed);

                    // Simulate processing multiple chunks
                    let mut chunks = Vec::new();
                    for _ in 0..100 {
                        let chunk = vec![0u8; chunk_size as usize];
                        chunks.push(chunk);
                    }

                    black_box(chunks);

                    let memory_used = get_memory_usage();
                    memory_used
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    name = memory_usage_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(20);
    targets = bench_segment_calculation_memory, bench_large_buffer_allocation,
              bench_concurrent_downloads_memory, bench_stream_processing_memory
);

criterion_main!(memory_usage_benches);
