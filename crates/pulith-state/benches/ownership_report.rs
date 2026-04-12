use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use pulith_resource::ResourceId;
use pulith_state::StateReady;

fn prepare_state(entries: usize, shared_targets: usize) -> (tempfile::TempDir, StateReady) {
    let temp = tempfile::tempdir().expect("tempdir");
    let state = StateReady::initialize(temp.path().join("state.json")).expect("state init");

    for i in 0..entries {
        let id = ResourceId::parse(format!("example/owner-{i}")).expect("resource id");
        let target = temp
            .path()
            .join("activation")
            .join(format!("slot-{}", i % shared_targets));
        state
            .record_activation(&id, target)
            .expect("record activation");
    }

    (temp, state)
}

fn bench_ownership_report(c: &mut Criterion) {
    {
        let mut direct_group = c.benchmark_group("state_ownership_report_direct");
        for entries in [1_000_usize, 5_000, 10_000] {
            let (_temp, state) = prepare_state(entries, 100);
            direct_group.bench_with_input(
                BenchmarkId::new("entries", entries),
                &entries,
                |b, _| {
                    b.iter(|| {
                        let report = state
                            .activation_ownership_report()
                            .expect("ownership report");
                        std::hint::black_box(report);
                    });
                },
            );
        }
        direct_group.finish();
    }

    {
        let mut indexed_group = c.benchmark_group("state_ownership_report_indexed");
        for entries in [1_000_usize, 5_000, 10_000] {
            let (_temp, state) = prepare_state(entries, 100);
            let index = state.build_analysis_index().expect("analysis index");
            indexed_group.bench_with_input(
                BenchmarkId::new("entries", entries),
                &entries,
                |b, _| {
                    b.iter(|| {
                        let report = state.activation_ownership_report_with_index(&index);
                        std::hint::black_box(report);
                    });
                },
            );
        }
        indexed_group.finish();
    }
}

criterion_group!(benches, bench_ownership_report);
criterion_main!(benches);
