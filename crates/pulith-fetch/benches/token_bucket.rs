use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use pulith_fetch::core::TokenBucket;
use std::sync::Arc;
use std::time::Duration;

fn bench_token_bucket_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_bucket_throughput");
    
    // Test different bandwidth limits (bytes per second)
    for bandwidth in [1024 * 1024, 10 * 1024 * 1024, 100 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*bandwidth));
        group.bench_with_input(
            BenchmarkId::new("acquire_tokens", bandwidth),
            bandwidth,
            |b, &bandwidth| {
                // Use a runtime for async benchmarks
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let bucket = TokenBucket::new(bandwidth, bandwidth);
                        let chunk_size = 64 * 1024; // 64KB chunks
                        
                        // Simulate acquiring tokens for multiple chunks
                        for _ in 0..100 {
                            bucket.acquire(black_box(chunk_size)).await;
                        }
                    });
                });
            },
        );
    }
    
    group.finish();
}

fn bench_token_bucket_concurrent(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_bucket_concurrent");
    
    for concurrent_tasks in [1, 2, 4, 8].iter() {
        group.bench_with_input(
            BenchmarkId::new("concurrent_acquire", concurrent_tasks),
            concurrent_tasks,
            |b, &concurrent_tasks| {
                // Use a runtime for async benchmarks
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let bucket = Arc::new(TokenBucket::new(10 * 1024 * 1024, 10 * 1024 * 1024));
                        let mut handles = vec![];
                        
                        for _ in 0..concurrent_tasks {
                            let bucket_clone: Arc<TokenBucket> = Arc::clone(&bucket);
                            let handle = tokio::spawn(async move {
                                for _ in 0..10 {
                                    bucket_clone.acquire(black_box(1024)).await;
                                }
                            });
                            handles.push(handle);
                        }
                        
                        for handle in handles {
                            handle.await.unwrap();
                        }
                    });
                });
            },
        );
    }
    
    group.finish();
}

fn bench_token_bucket_try_acquire(c: &mut Criterion) {
    let mut group = c.benchmark_group("token_bucket_try_acquire");
    
    for bucket_size in [1024, 10240, 102400].iter() {
        group.bench_with_input(
            BenchmarkId::new("try_acquire", bucket_size),
            bucket_size,
            |b, &bucket_size| {
                let bucket = TokenBucket::new(bucket_size, bucket_size);
                
                b.iter(|| {
                    // Try to acquire tokens multiple times
                    for _ in 0..1000 {
                        black_box(bucket.try_acquire(black_box(1024)));
                    }
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    name = token_bucket_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(20);
    targets = bench_token_bucket_throughput, bench_token_bucket_concurrent, bench_token_bucket_try_acquire
);

criterion_main!(token_bucket_benches);