use std::hint::black_box;
use std::path::{Path, PathBuf};

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use pulith_fs::{DEFAULT_COPY_ONLY_THRESHOLD_BYTES, FallBack, HardlinkOrCopyOptions, Workspace};
use pulith_store::{StoreKey, StoreReady, StoreRoots, StoredArtifact};

struct TransitionContext {
    _temp: tempfile::TempDir,
    source_path: PathBuf,
    store: StoreReady,
    install_root: PathBuf,
}

#[derive(Clone, Copy)]
enum TransitionMode {
    LinkOrCopy,
    CopyOnly,
    Adaptive { threshold_bytes: u64 },
}

impl TransitionMode {
    fn label(self) -> &'static str {
        match self {
            Self::LinkOrCopy => "link_or_copy",
            Self::CopyOnly => "copy_only",
            Self::Adaptive { threshold_bytes } => match threshold_bytes {
                ADAPTIVE_1M_THRESHOLD_BYTES => "adaptive_1m",
                DEFAULT_COPY_ONLY_THRESHOLD_BYTES => "adaptive_4m",
                ADAPTIVE_8M_THRESHOLD_BYTES => "adaptive_8m",
                _ => "adaptive_custom",
            },
        }
    }
}

const ADAPTIVE_1M_THRESHOLD_BYTES: u64 = 1024 * 1024;
const ADAPTIVE_8M_THRESHOLD_BYTES: u64 = 8 * 1024 * 1024;

fn setup_context(size: usize) -> TransitionContext {
    let temp = tempfile::tempdir().unwrap();
    let source_path = temp.path().join("source/runtime.bin");
    std::fs::create_dir_all(source_path.parent().unwrap()).unwrap();
    std::fs::write(&source_path, vec![0x7a; size]).unwrap();

    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();

    TransitionContext {
        _temp: temp,
        source_path,
        store,
        install_root: PathBuf::from("install/runtime"),
    }
}

fn transition_artifact_pipeline(context: &TransitionContext, mode: TransitionMode) {
    let key = StoreKey::logical("transition-artifact").unwrap();
    let stored = import_artifact(context, &key, mode);
    install_artifact(context, &stored.path, mode);
}

fn import_artifact(
    context: &TransitionContext,
    key: &StoreKey,
    mode: TransitionMode,
) -> StoredArtifact {
    match mode {
        TransitionMode::LinkOrCopy => context
            .store
            .import_artifact(key, &context.source_path)
            .unwrap(),
        TransitionMode::CopyOnly => {
            import_artifact_copy_only(&context.store, key, &context.source_path).unwrap()
        }
        TransitionMode::Adaptive { threshold_bytes } => {
            import_artifact_adaptive(&context.store, key, &context.source_path, threshold_bytes)
                .unwrap()
        }
    }
}

fn import_artifact_copy_only(
    store: &StoreReady,
    key: &StoreKey,
    source: &Path,
) -> pulith_store::Result<StoredArtifact> {
    let file_name = source
        .file_name()
        .ok_or_else(|| pulith_store::StoreError::MissingFileName(source.to_path_buf()))?;
    let workspace_root = tempfile::tempdir()?;
    let workspace = Workspace::new(
        workspace_root.path().join("artifact"),
        store.roots().artifacts.clone(),
    )?;
    let relative = PathBuf::from(key.relative_name()).join(file_name);
    workspace.copy_file(source, &relative)?;
    workspace.commit()?;

    Ok(StoredArtifact {
        key: key.clone(),
        path: store.artifact_path(key).join(file_name),
        provenance: None,
    })
}

fn import_artifact_adaptive(
    store: &StoreReady,
    key: &StoreKey,
    source: &Path,
    threshold_bytes: u64,
) -> pulith_store::Result<StoredArtifact> {
    if std::fs::metadata(source)?.len() < threshold_bytes {
        import_artifact_copy_only(store, key, source)
    } else {
        store.import_artifact(key, source)
    }
}

fn install_artifact(context: &TransitionContext, stored_path: &Path, mode: TransitionMode) {
    let install_root = context._temp.path().join(&context.install_root);
    if let Some(parent) = install_root.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    let workspace_root = tempfile::tempdir().unwrap();
    let workspace = Workspace::new(workspace_root.path().join("install"), &install_root).unwrap();

    match mode {
        TransitionMode::LinkOrCopy => {
            workspace
                .link_or_copy_file(
                    stored_path,
                    "runtime.bin",
                    HardlinkOrCopyOptions::new().fallback(FallBack::Copy),
                )
                .unwrap();
        }
        TransitionMode::CopyOnly => {
            let _ = workspace.copy_file(stored_path, "runtime.bin").unwrap();
        }
        TransitionMode::Adaptive { threshold_bytes } => {
            if std::fs::metadata(stored_path).unwrap().len() < threshold_bytes {
                let _ = workspace.copy_file(stored_path, "runtime.bin").unwrap();
            } else {
                workspace
                    .link_or_copy_file(
                        stored_path,
                        "runtime.bin",
                        HardlinkOrCopyOptions::new().fallback(FallBack::Copy),
                    )
                    .unwrap();
            }
        }
    }

    workspace.commit().unwrap();
}

fn bench_copy_transition(c: &mut Criterion) {
    let mut group = c.benchmark_group("artifact_transition_store_install");

    for size in [
        64 * 1024usize,
        256 * 1024,
        1024 * 1024,
        4 * 1024 * 1024,
        8 * 1024 * 1024,
        16 * 1024 * 1024,
    ] {
        group.throughput(Throughput::Bytes(size as u64));
        for mode in [
            TransitionMode::LinkOrCopy,
            TransitionMode::CopyOnly,
            TransitionMode::Adaptive {
                threshold_bytes: ADAPTIVE_1M_THRESHOLD_BYTES,
            },
            TransitionMode::Adaptive {
                threshold_bytes: DEFAULT_COPY_ONLY_THRESHOLD_BYTES,
            },
            TransitionMode::Adaptive {
                threshold_bytes: ADAPTIVE_8M_THRESHOLD_BYTES,
            },
        ] {
            group.bench_with_input(BenchmarkId::new(mode.label(), size), &size, |b, &size| {
                b.iter_batched(
                    || setup_context(size),
                    |context| {
                        transition_artifact_pipeline(&context, mode);
                        black_box(context);
                    },
                    BatchSize::LargeInput,
                );
            });
        }
    }

    group.finish();
}

criterion_group!(copy_transition_benches, bench_copy_transition);
criterion_main!(copy_transition_benches);
