use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use pulith_fetch::effects::ThrottledStream;
use pulith_fetch::core::TokenBucket;
use bytes::Bytes;
use futures_util::{Stream, StreamExt};
use std::pin::Pin;
use std::task::{Context, Poll};
use std::time::Duration;

// Mock stream for testing
struct MockStream {
    chunks: Vec<Bytes>,
    index: usize,
}

impl MockStream {
    fn new(chunk_size: usize, num_chunks: usize) -> Self {
        let data = vec![0u8; chunk_size];
        let chunks: Vec<_> = (0..num_chunks)
            .map(|_| Bytes::copy_from_slice(&data))
            .collect();
        Self { chunks, index: 0 }
    }
}

impl Stream for MockStream {
    type Item = Result<Bytes, Box<dyn std::error::Error + Send>>;

    fn poll_next(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.index < self.chunks.len() {
            let chunk = self.chunks[self.index].clone();
            self.index += 1;
            Poll::Ready(Some(Ok(chunk)))
        } else {
            Poll::Ready(None)
        }
    }
}

fn bench_throttled_stream_throughput(c: &mut Criterion) {
    let mut group = c.benchmark_group("throttled_stream_throughput");
    
    // Test different bandwidth limits
    for bandwidth in [1024 * 1024, 10 * 1024 * 1024, 100 * 1024 * 1024].iter() {
        let total_bytes = 10 * 1024 * 1024; // 10MB total
        group.throughput(Throughput::Bytes(total_bytes));
        group.bench_with_input(
            BenchmarkId::new("throttled_stream", bandwidth),
            bandwidth,
            |b, &bandwidth| {
                // Use a runtime for async benchmarks
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let mock_stream = MockStream::new(64 * 1024, (total_bytes / (64 * 1024)) as usize);
                        let throttled_stream = ThrottledStream::new(mock_stream, bandwidth);
                        
                        let mut total_processed = 0;
                        let mut stream = Box::pin(throttled_stream);
                        
                        while let Some(chunk) = stream.next().await {
                            total_processed += black_box(chunk.unwrap().len());
                        }
                        
                        total_processed
                    });
                });
            },
        );
    }
    
    group.finish();
}

fn bench_stream_processing_chunk_sizes(c: &mut Criterion) {
    let mut group = c.benchmark_group("stream_processing_chunk_sizes");
    
    // Test different chunk sizes
    for chunk_size in [1024, 4096, 16384, 65536].iter() {
        let total_bytes = 10 * 1024 * 1024; // 10MB total
        group.throughput(Throughput::Bytes(total_bytes));
        group.bench_with_input(
            BenchmarkId::new("chunk_processing", chunk_size),
            chunk_size,
            |b, &chunk_size| {
                // Use a runtime for async benchmarks
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let bandwidth = 100 * 1024 * 1024; // High bandwidth
                        let num_chunks = (total_bytes / chunk_size) as usize;
                        let mock_stream = MockStream::new(chunk_size as usize, num_chunks);
                        let throttled_stream = ThrottledStream::new(mock_stream, bandwidth);
                        
                        let mut total_processed = 0;
                        let mut stream = Box::pin(throttled_stream);
                        
                        while let Some(chunk) = stream.next().await {
                            total_processed += black_box(chunk.unwrap().len());
                        }
                        
                        total_processed
                    });
                });
            },
        );
    }
    
    group.finish();
}

fn bench_unthrottled_stream(c: &mut Criterion) {
    let mut group = c.benchmark_group("unthrottled_stream");
    
    for total_bytes in [1024 * 1024, 10 * 1024 * 1024, 100 * 1024 * 1024].iter() {
        group.throughput(Throughput::Bytes(*total_bytes));
        group.bench_with_input(
            BenchmarkId::new("raw_stream", total_bytes),
            total_bytes,
            |b, &total_bytes| {
                // Use a runtime for async benchmarks
                let rt = tokio::runtime::Runtime::new().unwrap();
                
                b.iter(|| {
                    rt.block_on(async {
                        let chunk_size: usize = 64 * 1024;
                        let num_chunks = (total_bytes / chunk_size as u64) as usize;
                        let mock_stream = MockStream::new(chunk_size, num_chunks);
                        
                        let mut total_processed = 0;
                        let mut stream = Box::pin(mock_stream);
                        
                        while let Some(chunk) = stream.next().await {
                            total_processed += black_box(chunk.unwrap().len());
                        }
                        
                        total_processed
                    });
                });
            },
        );
    }
    
    group.finish();
}

criterion_group!(
    name = stream_processing_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(20);
    targets = bench_throttled_stream_throughput, bench_stream_processing_chunk_sizes, bench_unthrottled_stream
);

criterion_main!(stream_processing_benches);