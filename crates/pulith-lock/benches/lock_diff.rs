use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use pulith_lock::{LockFile, LockedResource};

fn build_lock(entries: usize, version_offset: usize) -> LockFile {
    let mut lock = LockFile::default();
    for i in 0..entries {
        lock.upsert(
            format!("example/resource-{i}"),
            LockedResource::new(
                format!("1.{}.{}", version_offset, i % 10),
                format!("https://example.com/resource-{i}.tgz"),
            ),
        );
    }
    lock
}

fn bench_lock_diff(c: &mut Criterion) {
    let mut group = c.benchmark_group("lock_diff");

    for entries in [100_usize, 1_000, 5_000] {
        let base = build_lock(entries, 0);
        let mut next = build_lock(entries, 1);
        next.upsert(
            "example/new-entry",
            LockedResource::new("2.0.0", "https://example.com/new-entry.tgz"),
        );

        group.bench_with_input(BenchmarkId::new("entries", entries), &entries, |b, _| {
            b.iter(|| {
                let diff = base.diff(&next);
                std::hint::black_box(diff);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_lock_diff);
criterion_main!(benches);
