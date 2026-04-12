use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use pulith_resource::{ResourceId, VersionSelector};
use pulith_state::{ResourceLifecycle, ResourceRecordPatch, StateReady};

fn prepare_state(entries: usize) -> (tempfile::TempDir, StateReady, ResourceId) {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = StateReady::initialize(temp.path().join("state.json")).expect("state init");

    for i in 0..entries {
        let id = ResourceId::parse(format!("example/runtime-{i}")).expect("id");
        state
            .ensure_resource_record(id.clone(), VersionSelector::alias("lts").expect("alias"))
            .expect("ensure");
        state
            .patch_resource_record(
                &id,
                ResourceRecordPatch::install_path(Some(
                    temp.path().join(format!("missing/runtime-{i}")),
                ))
                .with_lifecycle(ResourceLifecycle::Installed),
            )
            .expect("patch");
    }

    let target = ResourceId::parse(format!("example/runtime-{}", entries / 2)).expect("target");
    (temp, state, target)
}

fn bench_repair_plan(c: &mut Criterion) {
    let mut group = c.benchmark_group("state_repair_plan");

    for entries in [100_usize, 1_000, 5_000] {
        let (_temp, state, target) = prepare_state(entries);

        group.bench_with_input(BenchmarkId::new("entries", entries), &entries, |b, _| {
            b.iter(|| {
                let plan = state
                    .plan_resource_state_repair(&target, None)
                    .expect("repair plan");
                std::hint::black_box(plan);
            });
        });
    }

    group.finish();
}

criterion_group!(benches, bench_repair_plan);
criterion_main!(benches);
