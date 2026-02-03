use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use pulith_fetch::core::{calculate_segments, Segment};
use std::time::Duration;

fn bench_calculate_segments(c: &mut Criterion) {
    let mut group = c.benchmark_group("calculate_segments");

    // Test different file sizes with various segment counts
    for file_size in [
        1024 * 1024,       // 1MB
        10 * 1024 * 1024,  // 10MB
        100 * 1024 * 1024, // 100MB
    ]
    .iter()
    {
        for num_segments in [1, 4, 8, 16].iter() {
            group.throughput(Throughput::Bytes(*file_size));
            group.bench_with_input(
                BenchmarkId::new(
                    "file_size",
                    format!("{}_segments_{}", file_size, num_segments),
                ),
                &(*file_size, *num_segments),
                |b, &(file_size, num_segments)| {
                    b.iter(|| {
                        let segments =
                            calculate_segments(black_box(file_size), black_box(num_segments))
                                .unwrap();
                        black_box(segments)
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_segment_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("segment_iteration");

    // Create segments once and benchmark iteration
    let file_size = 100 * 1024 * 1024; // 100MB
    let num_segments = 16;
    let segments = calculate_segments(file_size, num_segments).unwrap();

    group.bench_with_input(
        BenchmarkId::new("iterate_segments", num_segments),
        &segments,
        |b, segments| {
            b.iter(|| {
                let mut total_bytes = 0u64;
                for segment in segments {
                    total_bytes += black_box(segment.end - segment.start);
                }
                black_box(total_bytes)
            });
        },
    );

    group.finish();
}

fn bench_segment_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("segment_creation");

    group.bench_function("create_segment", |b| {
        b.iter(|| {
            let segment = Segment {
                index: black_box(0),
                start: black_box(0),
                end: black_box(1024 * 1024),
            };
            black_box(segment)
        });
    });

    group.finish();
}

criterion_group!(
    name = segment_benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(20);
    targets = bench_calculate_segments, bench_segment_iteration, bench_segment_creation
);

criterion_main!(segment_benches);
