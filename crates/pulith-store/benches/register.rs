use std::hint::black_box;

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use pulith_resource::Metadata;
use pulith_store::{StoreKey, StoreProvenance, StoreReady, StoreRoots};

struct RegisterContext {
    _temp: tempfile::TempDir,
    store: StoreReady,
    source_file: std::path::PathBuf,
    extract_root: std::path::PathBuf,
}

fn setup_context(size: usize) -> RegisterContext {
    let temp = tempfile::tempdir().unwrap();
    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();

    let source_file = temp.path().join("source/runtime.bin");
    std::fs::create_dir_all(source_file.parent().unwrap()).unwrap();
    std::fs::write(&source_file, vec![0x7a; size]).unwrap();

    let extract_root = temp.path().join("source/extract");
    std::fs::create_dir_all(&extract_root).unwrap();
    std::fs::write(extract_root.join("tool.bin"), vec![0x5b; size]).unwrap();

    RegisterContext {
        _temp: temp,
        store,
        source_file,
        extract_root,
    }
}

fn bench_register_artifact(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_register_artifact");
    for size in [64 * 1024usize, 1024 * 1024, 8 * 1024 * 1024] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || setup_context(size),
                |context| {
                    let key = StoreKey::logical("bench-artifact").unwrap();
                    let artifact = context
                        .store
                        .register_artifact(
                            &key,
                            (
                                context.source_file.as_path(),
                                StoreProvenance {
                                    origin: Some("bench://artifact".to_string()),
                                    metadata: Metadata::from([(
                                        "bench.path".to_string(),
                                        "register_artifact".to_string(),
                                    )]),
                                },
                            ),
                        )
                        .unwrap();

                    black_box(artifact.path);
                },
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

fn bench_register_extract(c: &mut Criterion) {
    let mut group = c.benchmark_group("store_register_extract");
    for size in [64 * 1024usize, 1024 * 1024, 8 * 1024 * 1024] {
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || setup_context(size),
                |context| {
                    let key = StoreKey::logical("bench-extract").unwrap();
                    let extract = context
                        .store
                        .register_extract(
                            &key,
                            (
                                context.extract_root.as_path(),
                                StoreProvenance {
                                    origin: Some("bench://extract".to_string()),
                                    metadata: Metadata::from([(
                                        "bench.path".to_string(),
                                        "register_extract".to_string(),
                                    )]),
                                },
                            ),
                        )
                        .unwrap();

                    black_box(extract.path);
                },
                BatchSize::LargeInput,
            )
        });
    }
    group.finish();
}

criterion_group!(
    register_benches,
    bench_register_artifact,
    bench_register_extract
);
criterion_main!(register_benches);
