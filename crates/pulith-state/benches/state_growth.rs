use std::path::PathBuf;

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use pulith_resource::{Metadata, ResourceId, VersionSelector};
use pulith_state::{
    ResourceLifecycle, ResourceRecord, STATE_SNAPSHOT_SCHEMA_VERSION, StateReady, StateSnapshot,
};

fn seed_snapshot(resource_count: usize) -> StateSnapshot {
    let resources = (0..resource_count)
        .map(|index| ResourceRecord {
            id: ResourceId::parse(format!("example/runtime-{index}")).unwrap(),
            selector: VersionSelector::exact("1.0.0").unwrap(),
            resolved_version: None,
            locator: None,
            artifact_key: None,
            install_path: Some(PathBuf::from(format!("/installs/runtime-{index}"))),
            lifecycle: ResourceLifecycle::Installed,
            metadata: Metadata::new(),
        })
        .collect();

    StateSnapshot {
        schema_version: STATE_SNAPSHOT_SCHEMA_VERSION,
        resources,
        activations: vec![],
    }
}

fn bench_state_save(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_save");
    for size in [10usize, 100, 1_000, 5_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter(|| {
                let temp = tempfile::tempdir().unwrap();
                let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
                state.save(&seed_snapshot(size)).unwrap();
            });
        });
    }
    group.finish();
}

fn bench_state_patch(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_patch");
    for size in [10usize, 100, 1_000, 5_000] {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            let temp = tempfile::tempdir().unwrap();
            let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
            state.save(&seed_snapshot(size)).unwrap();
            let id = ResourceId::parse(format!("example/runtime-{}", size / 2)).unwrap();

            b.iter(|| {
                state
                    .set_resource_lifecycle(&id, ResourceLifecycle::Active)
                    .unwrap();
            });
        });
    }
    group.finish();
}

criterion_group!(benches, bench_state_save, bench_state_patch);
criterion_main!(benches);
