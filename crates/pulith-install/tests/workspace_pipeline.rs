use std::fs;
use std::io::{Cursor, Write};
use std::path::Path;

use pulith_archive::{ArchiveFormat, ArchiveReport, ExtractOptions, extract_from_reader};
use pulith_fetch::{Fetcher, MultiSourceFetcher, ReqwestClient};
use pulith_install::{
    ActivationReceipt, ActivationRequest, ActivationTarget, Activator, InstallInput, InstallReady,
    InstallSpec, PlannedInstall, ShimCommand, ShimCopyActivator, SymlinkActivator,
};
use pulith_resource::{
    RequestedResource, ResolvedLocator, ResolvedVersion, ResourceId, ResourceLocator, ResourceSpec,
    ValidUrl,
};
use pulith_source::PlannedSources;
use pulith_state::{
    OwnershipReason, OwnershipSeverity, ResourceInspectionFinding, ResourceLifecycle,
    ResourceRecordPatch, StateReady, StoreRetentionPolicy,
};
use pulith_store::{StoreKey, StoreMetadataRecord, StoreReady, StoreRoots};

fn resolved_resource(locator: ResourceLocator) -> pulith_resource::ResolvedResource {
    resolved_resource_version(locator, "1.0.0")
}

fn resolved_resource_version(
    locator: ResourceLocator,
    version: &str,
) -> pulith_resource::ResolvedResource {
    RequestedResource::new(
        ResourceSpec::new(ResourceId::parse("example/runtime").unwrap(), locator)
            .version(pulith_resource::VersionSelector::exact(version).unwrap()),
    )
    .resolve(
        ResolvedVersion::new(version).unwrap(),
        ResolvedLocator::LocalPath(std::path::PathBuf::from("/local/runtime")),
        None,
    )
}

fn archive_report(total_bytes: u64) -> ArchiveReport {
    ArchiveReport {
        format: ArchiveFormat::Zip,
        entry_count: 1,
        total_bytes,
        entries: vec![],
    }
}

fn write_runtime_tree(root: &Path, relative_path: &str, bytes: &[u8]) {
    let file_path = root.join(relative_path);
    fs::create_dir_all(file_path.parent().unwrap()).unwrap();
    fs::write(file_path, bytes).unwrap();
}

fn create_runtime_zip(archive_path: &Path, relative_path: &str, bytes: &[u8]) {
    let file = fs::File::create(archive_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file(relative_path, zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(bytes).unwrap();
    writer.finish().unwrap();
}

fn fetch_local_resource_to(source_path: &Path, destination: &Path) -> pulith_fetch::FetchReceipt {
    let resource = resolved_resource(ResourceLocator::LocalPath(source_path.to_path_buf()));
    let planned = PlannedSources::from_resolved_resource(
        &resource,
        pulith_source::SelectionStrategy::OrderedFallback,
    )
    .unwrap();
    let fetcher = Fetcher::new(
        ReqwestClient::new().unwrap(),
        destination.parent().unwrap().join("fetch-workspace"),
    );
    let multi = MultiSourceFetcher::new(std::sync::Arc::new(fetcher));
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            multi
                .fetch_planned_sources_with_receipt(
                    &planned,
                    destination,
                    &pulith_fetch::FetchOptions::default(),
                )
                .await
        })
        .unwrap()
}

#[derive(Debug, Default)]
struct FileActivator;

impl Activator for FileActivator {
    fn activate(&self, request: &ActivationRequest) -> pulith_install::Result<ActivationReceipt> {
        if let Some(parent) = request.target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            &request.target,
            request.installed_path.to_string_lossy().as_bytes(),
        )?;
        Ok(ActivationReceipt {
            target: request.target.clone(),
            installed_path: request.installed_path.clone(),
        })
    }
}

#[test]
fn local_source_fetch_store_install_activate_pipeline() {
    let temp = tempfile::tempdir().unwrap();
    let local_source_path = temp.path().join("source.bin");
    fs::write(&local_source_path, b"runtime-binary").unwrap();

    let resource = resolved_resource(ResourceLocator::LocalPath(local_source_path.clone()));
    let fetched = fetch_local_resource_to(
        &local_source_path,
        &temp.path().join("downloads/runtime.bin"),
    );

    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let key = StoreKey::logical("runtime-bin").unwrap();
    let install_input = InstallInput::store_fetched_artifact(&store, &key, &fetched).unwrap();

    let metadata_path = store.metadata_path(&key);
    let metadata_record: StoreMetadataRecord =
        serde_json::from_slice(&fs::read(&metadata_path).unwrap()).unwrap();
    let provenance = metadata_record.provenance.unwrap();
    assert_eq!(
        provenance.origin.as_deref(),
        Some(local_source_path.to_string_lossy().as_ref())
    );

    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state.clone());
    let receipt = PlannedInstall::new(
        ready,
        InstallSpec::new(
            resource,
            install_input,
            temp.path().join("installs/runtime"),
        )
        .activation(ActivationTarget {
            path: temp.path().join("active/runtime"),
        }),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&SymlinkActivator)
    .unwrap()
    .finish();

    assert!(receipt.install_root.join("runtime.bin").exists());
    assert!(receipt.activation.unwrap().target.exists());

    let snapshot = state.load().unwrap();
    assert_eq!(snapshot.resources.len(), 1);
    assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Active);
}

#[test]
fn copy_file_activation_pipeline_copies_executable_bytes() {
    let temp = tempfile::tempdir().unwrap();
    let source_dir = temp.path().join("runtime-src");
    write_runtime_tree(&source_dir, "bin/runtime.exe", b"runtime-binary");

    let resource = resolved_resource(ResourceLocator::LocalPath(source_dir.clone()));
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state.clone());
    let receipt = PlannedInstall::new(
        ready,
        InstallSpec::new(
            resource,
            InstallInput::from_archive_extraction(source_dir.clone(), archive_report(14)),
            temp.path().join("installs/runtime"),
        )
        .activation(ActivationTarget {
            path: temp.path().join("active/runtime.exe"),
        }),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&ShimCopyActivator::new(
        ShimCommand::new("runtime", "bin/runtime.exe").unwrap(),
    ))
    .unwrap()
    .finish();

    assert_eq!(
        fs::read(&receipt.activation.unwrap().target).unwrap(),
        b"runtime-binary"
    );
    let snapshot = state.load().unwrap();
    assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Active);
}

#[test]
fn repeated_copy_activation_replaces_existing_file_target() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let activation_target = temp.path().join("active/runtime.exe");

    for (version, payload) in [
        ("1.0.0", b"runtime-v1".as_slice()),
        ("1.1.0", b"runtime-v2".as_slice()),
    ] {
        let source_dir = temp.path().join(format!("runtime-src-{version}"));
        write_runtime_tree(&source_dir, "bin/runtime.exe", payload);

        let resource =
            resolved_resource_version(ResourceLocator::LocalPath(source_dir.clone()), version);

        PlannedInstall::new(
            InstallReady::new(state.clone()),
            InstallSpec::new(
                resource,
                InstallInput::from_archive_extraction(
                    source_dir.clone(),
                    archive_report(payload.len() as u64),
                ),
                temp.path().join("installs/runtime"),
            )
            .replace_existing()
            .activation(ActivationTarget {
                path: activation_target.clone(),
            }),
        )
        .stage()
        .unwrap()
        .commit()
        .unwrap()
        .activate(&ShimCopyActivator::new(
            ShimCommand::new("runtime", "bin/runtime.exe").unwrap(),
        ))
        .unwrap()
        .finish();
    }

    assert_eq!(fs::read(&activation_target).unwrap(), b"runtime-v2");
    let snapshot = state.load().unwrap();
    assert_eq!(snapshot.resources.len(), 1);
    assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Active);
}

#[test]
fn repeated_symlink_activation_replaces_existing_file_target() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let activation_target = temp.path().join("active/runtime-link.exe");

    for (version, payload) in [
        ("1.0.0", b"runtime-v1".as_slice()),
        ("1.1.0", b"runtime-v2".as_slice()),
    ] {
        let source_path = temp.path().join(format!("runtime-{version}.bin"));
        fs::write(&source_path, payload).unwrap();

        let resource =
            resolved_resource_version(ResourceLocator::LocalPath(source_path.clone()), version);
        let fetched = fetch_local_resource_to(
            &source_path,
            &temp.path().join(format!("downloads/runtime-{version}.bin")),
        );

        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();
        let key = StoreKey::logical(format!("runtime-bin-{version}")).unwrap();
        let install_input = InstallInput::store_fetched_artifact(&store, &key, &fetched).unwrap();

        PlannedInstall::new(
            InstallReady::new(state.clone()),
            InstallSpec::new(
                resource,
                install_input,
                temp.path().join("installs/runtime-link"),
            )
            .replace_existing()
            .activation(ActivationTarget {
                path: activation_target.clone(),
            }),
        )
        .stage()
        .unwrap()
        .commit()
        .unwrap()
        .activate(&SymlinkActivator)
        .unwrap()
        .finish();
    }

    assert!(activation_target.exists());
    #[cfg(not(windows))]
    assert_eq!(fs::read(&activation_target).unwrap(), b"runtime-v2");
    let snapshot = state.load().unwrap();
    assert_eq!(snapshot.resources.len(), 1);
    assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Active);
}

#[test]
fn installed_resource_can_be_inspected_for_drift_without_mutation() {
    let temp = tempfile::tempdir().unwrap();
    let local_source_path = temp.path().join("source.bin");
    fs::write(&local_source_path, b"runtime-binary").unwrap();

    let resource = resolved_resource(ResourceLocator::LocalPath(local_source_path.clone()));
    let fetched = fetch_local_resource_to(
        &local_source_path,
        &temp.path().join("downloads/runtime.bin"),
    );

    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let key = StoreKey::logical("runtime-bin").unwrap();
    let install_input = InstallInput::store_fetched_artifact(&store, &key, &fetched).unwrap();

    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let install_root = temp.path().join("installs/runtime");
    let activation_target = temp.path().join("active/runtime");
    PlannedInstall::new(
        InstallReady::new(state.clone()),
        InstallSpec::new(resource.clone(), install_input, install_root.clone()).activation(
            ActivationTarget {
                path: activation_target.clone(),
            },
        ),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&SymlinkActivator)
    .unwrap()
    .finish();

    fs::remove_dir_all(&install_root).unwrap();
    fs::remove_file(store.metadata_path(&key)).unwrap();
    let missing_activation_target = temp.path().join("active/missing-runtime");
    state
        .remove_activation_records(&resource.spec().id)
        .unwrap();
    state
        .record_activation(&resource.spec().id, missing_activation_target.clone())
        .unwrap();

    let resource_id = resource.spec().id.clone();
    let before = state.load().unwrap();
    let inspection = state.inspect_resource(&resource_id, Some(&store)).unwrap();
    let after = state.load().unwrap();

    assert_eq!(before, after);
    assert_eq!(inspection.summary.total_findings, 3);
    assert_eq!(inspection.summary.error_count, 2);
    assert_eq!(inspection.summary.warning_count, 1);
    assert!(
        inspection
            .findings
            .contains(&ResourceInspectionFinding::MissingInstallPath {
                resource: resource_id.clone(),
                path: install_root,
            })
    );
    assert!(
        inspection
            .findings
            .contains(&ResourceInspectionFinding::MissingActivationTarget {
                resource: resource_id.clone(),
                target: missing_activation_target,
            })
    );
    assert!(
        inspection
            .findings
            .contains(&ResourceInspectionFinding::MissingStoreMetadata {
                resource: resource_id,
                key,
            })
    );
}

#[test]
fn ownership_and_retention_plan_is_explicit_and_stable() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();

    let runtime_a = ResourceId::parse("example/runtime-a").unwrap();
    let runtime_b = ResourceId::parse("example/runtime-b").unwrap();
    let active_key = StoreKey::logical("runtime-active").unwrap();
    let fetched_key = StoreKey::logical("runtime-fetched").unwrap();
    let orphaned_key = StoreKey::logical("runtime-orphaned").unwrap();
    let shared_target = temp.path().join("active/shared-runtime");

    fs::create_dir_all(shared_target.parent().unwrap()).unwrap();
    fs::write(&shared_target, b"active").unwrap();

    for key in [&active_key, &fetched_key, &orphaned_key] {
        store.put_artifact_bytes(key, b"payload").unwrap();
        fs::remove_file(store.artifact_path(key)).unwrap();
    }

    state
        .ensure_resource_record(
            runtime_a.clone(),
            pulith_resource::VersionSelector::alias("lts").unwrap(),
        )
        .unwrap();
    state
        .patch_resource_record(
            &runtime_a,
            ResourceRecordPatch::artifact_key(Some(active_key.clone()))
                .with_lifecycle(ResourceLifecycle::Active),
        )
        .unwrap();
    state
        .ensure_resource_record(
            runtime_b.clone(),
            pulith_resource::VersionSelector::alias("lts").unwrap(),
        )
        .unwrap();
    state
        .patch_resource_record(
            &runtime_b,
            ResourceRecordPatch::artifact_key(Some(fetched_key.clone()))
                .with_lifecycle(ResourceLifecycle::Fetched),
        )
        .unwrap();

    state
        .record_activation(&runtime_b, shared_target.clone())
        .unwrap();
    state
        .record_activation(&runtime_a, shared_target.clone())
        .unwrap();

    let plan = state
        .plan_ownership_and_retention(&store, StoreRetentionPolicy::ActiveOnly)
        .unwrap();
    let stable_plan = state
        .plan_ownership_and_retention(&store, StoreRetentionPolicy::ActiveOnly)
        .unwrap();

    assert_eq!(plan, stable_plan);

    assert_eq!(plan.ownership.entries.len(), 1);
    assert_eq!(
        plan.ownership.entries[0].severity,
        OwnershipSeverity::Warning
    );
    assert_eq!(plan.ownership.entries[0].target, shared_target);
    assert_eq!(
        plan.ownership.entries[0].reasons,
        vec![OwnershipReason::SharedActivationTarget {
            target: plan.ownership.entries[0].target.clone(),
            owners: vec![runtime_a.clone(), runtime_b.clone()],
        }]
    );

    assert_eq!(plan.retention.protected_keys.len(), 1);
    assert_eq!(plan.retention.protected_keys[0].key, active_key);
    assert_eq!(
        plan.retention.protected_keys[0].reasons,
        vec![OwnershipReason::StateStoreReference {
            key: plan.retention.protected_keys[0].key.clone(),
            owner: runtime_a,
            lifecycle: ResourceLifecycle::Active,
        }]
    );

    assert_eq!(plan.retention.removable_metadata.len(), 2);
    assert_eq!(plan.retention.removable_metadata[0].record.key, fetched_key);
    assert_eq!(
        plan.retention.removable_metadata[0].reasons,
        vec![OwnershipReason::RetentionPolicyExcludesLifecycle {
            policy: StoreRetentionPolicy::ActiveOnly,
            resource: runtime_b,
            lifecycle: ResourceLifecycle::Fetched,
        }]
    );
    assert_eq!(
        plan.retention.removable_metadata[1].record.key,
        orphaned_key
    );
    assert_eq!(
        plan.retention.removable_metadata[1].reasons,
        vec![OwnershipReason::UnreferencedStoreMetadata {
            key: plan.retention.removable_metadata[1].record.key.clone(),
        }]
    );
}

#[test]
fn manager_like_reconcile_apply_cycle_repairs_and_prunes_statefully() {
    let temp = tempfile::tempdir().unwrap();
    let local_source_path = temp.path().join("source.bin");
    fs::write(&local_source_path, b"runtime-binary").unwrap();

    let resource = resolved_resource(ResourceLocator::LocalPath(local_source_path.clone()));
    let fetched = fetch_local_resource_to(
        &local_source_path,
        &temp.path().join("downloads/runtime.bin"),
    );

    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let key = StoreKey::logical("runtime-reconcile").unwrap();
    let install_input = InstallInput::store_fetched_artifact(&store, &key, &fetched).unwrap();

    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let install_root = temp.path().join("installs/runtime");
    let activation_target = temp.path().join("active/runtime");

    PlannedInstall::new(
        InstallReady::new(state.clone()),
        InstallSpec::new(resource.clone(), install_input, install_root.clone()).activation(
            ActivationTarget {
                path: activation_target.clone(),
            },
        ),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&SymlinkActivator)
    .unwrap()
    .finish();

    if install_root.exists() {
        fs::remove_dir_all(&install_root).unwrap();
    }
    if activation_target.exists() {
        let metadata = fs::symlink_metadata(&activation_target).unwrap();
        if metadata.file_type().is_symlink() || metadata.file_type().is_file() {
            fs::remove_file(&activation_target).unwrap();
        } else {
            fs::remove_dir_all(&activation_target).unwrap();
        }
    }
    let artifact_root = store.artifact_path(&key);
    if artifact_root.exists() {
        fs::remove_dir_all(&artifact_root).unwrap();
    }

    let resource_id = resource.spec().id.clone();
    let before = state.inspect_resource(&resource_id, Some(&store)).unwrap();
    assert!(before.summary.total_findings >= 2);

    let repair = state
        .plan_resource_state_repair(&resource_id, Some(&store))
        .unwrap();
    assert!(!repair.actions.is_empty());
    state.apply_resource_state_repair(&repair).unwrap();

    let after = state.inspect_resource(&resource_id, Some(&store)).unwrap();
    assert_eq!(after.summary.total_findings, 0);

    let ownership_retention = state
        .plan_ownership_and_retention(&store, StoreRetentionPolicy::InstalledAndActive)
        .unwrap();
    let protected_keys = ownership_retention
        .retention
        .protected_keys
        .iter()
        .map(|entry| entry.key.clone())
        .collect::<Vec<_>>();
    let prune = store
        .prune_missing_with_protection(&protected_keys)
        .unwrap();
    assert!(prune.removed_metadata >= 1);
}

#[test]
fn archive_extract_store_install_pipeline() {
    let temp = tempfile::tempdir().unwrap();

    let mut cursor = Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut cursor);
        writer
            .start_file("bin/tool.exe", zip::write::SimpleFileOptions::default())
            .unwrap();
        writer.write_all(b"payload").unwrap();
        writer.finish().unwrap();
    }
    cursor.set_position(0);

    let extract_root = temp.path().join("extracted");
    fs::create_dir_all(&extract_root).unwrap();
    let report = extract_from_reader(cursor, &extract_root, &ExtractOptions::default()).unwrap();

    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let key = StoreKey::logical("runtime-extract").unwrap();
    let install_input =
        InstallInput::store_archive_extraction(&store, &key, &extract_root, &report).unwrap();

    let metadata_record: StoreMetadataRecord =
        serde_json::from_slice(&fs::read(store.metadata_path(&key)).unwrap()).unwrap();
    let provenance = metadata_record.provenance.unwrap();
    assert_eq!(provenance.origin, None);
    assert_eq!(
        provenance
            .metadata
            .get("archive.format")
            .map(String::as_str),
        Some("Zip")
    );
    assert_eq!(
        provenance
            .metadata
            .get("archive.entry_count")
            .map(String::as_str),
        Some("1")
    );

    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state.clone());
    let receipt = PlannedInstall::new(
        ready,
        InstallSpec::new(
            resolved_resource(ResourceLocator::Url(
                ValidUrl::parse("https://example.com/runtime.zip").unwrap(),
            )),
            install_input,
            temp.path().join("installs/archive-runtime"),
        ),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .finish();

    assert_eq!(report.entry_count, 1);
    assert!(receipt.install_root.join("bin/tool.exe").exists());
    assert_eq!(
        state.load().unwrap().resources[0].lifecycle,
        ResourceLifecycle::Installed
    );
}

#[test]
fn local_archive_fetch_extract_store_install_pipeline() {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("runtime.zip");

    create_runtime_zip(&archive_path, "bin/tool.exe", b"payload");

    let resource = resolved_resource(ResourceLocator::LocalPath(archive_path.clone()));
    let fetched =
        fetch_local_resource_to(&archive_path, &temp.path().join("downloads/runtime.zip"));

    let extract_root = temp.path().join("extracted");
    fs::create_dir_all(&extract_root).unwrap();
    let fetched_file = fs::File::open(&fetched.destination).unwrap();
    let report =
        extract_from_reader(fetched_file, &extract_root, &ExtractOptions::default()).unwrap();

    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let key = StoreKey::logical("runtime-archive-extract").unwrap();
    let install_input = InstallInput::store_fetched_archive_extraction(
        &store,
        &key,
        &fetched,
        &extract_root,
        &report,
    )
    .unwrap();

    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state.clone());
    let receipt = PlannedInstall::new(
        ready,
        InstallSpec::new(
            resource,
            install_input,
            temp.path().join("installs/archive-runtime"),
        ),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .finish();

    assert!(receipt.install_root.join("bin/tool.exe").exists());

    let metadata_record: StoreMetadataRecord =
        serde_json::from_slice(&fs::read(store.metadata_path(&key)).unwrap()).unwrap();
    let provenance = metadata_record.provenance.unwrap();
    assert_eq!(
        provenance
            .metadata
            .get("archive.format")
            .map(String::as_str),
        Some("Zip")
    );
    assert_eq!(
        provenance.origin.as_deref(),
        Some(archive_path.to_string_lossy().as_ref())
    );
    assert_eq!(
        state.load().unwrap().resources[0].lifecycle,
        ResourceLifecycle::Installed
    );
}

#[test]
fn archive_replace_activate_rollback_restores_previous_activation_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let install_root = temp.path().join("installs/runtime");
    let old_target = temp.path().join("active/runtime-old");
    let new_target = temp.path().join("active/runtime-new");

    let initial_tree = temp.path().join("src-initial");
    write_runtime_tree(&initial_tree, "bin/tool.exe", b"v1");
    let initial_resource = resolved_resource(ResourceLocator::LocalPath(initial_tree.clone()));

    PlannedInstall::new(
        InstallReady::new(state.clone()),
        InstallSpec::new(
            initial_resource,
            InstallInput::from_archive_extraction(initial_tree.clone(), archive_report(2)),
            install_root.clone(),
        )
        .activation(ActivationTarget {
            path: old_target.clone(),
        }),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&FileActivator)
    .unwrap()
    .finish();

    let archive_path = temp.path().join("runtime.zip");
    create_runtime_zip(&archive_path, "bin/tool.exe", b"v2");
    let archive_resource = resolved_resource(ResourceLocator::LocalPath(archive_path.clone()));
    let fetched =
        fetch_local_resource_to(&archive_path, &temp.path().join("downloads/runtime.zip"));

    let extract_root = temp.path().join("extracted");
    fs::create_dir_all(&extract_root).unwrap();
    let fetched_file = fs::File::open(&fetched.destination).unwrap();
    let report =
        extract_from_reader(fetched_file, &extract_root, &ExtractOptions::default()).unwrap();

    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let key = StoreKey::logical("runtime-rollback-archive").unwrap();
    let install_input = InstallInput::store_fetched_archive_extraction(
        &store,
        &key,
        &fetched,
        &extract_root,
        &report,
    )
    .unwrap();

    let rollback = PlannedInstall::new(
        InstallReady::new(state.clone()),
        InstallSpec::new(archive_resource, install_input, install_root.clone())
            .replace_existing()
            .activation(ActivationTarget {
                path: new_target.clone(),
            }),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&FileActivator)
    .unwrap()
    .rollback()
    .unwrap();

    assert_eq!(rollback.restored_path, install_root);
    assert_eq!(fs::read(install_root.join("bin/tool.exe")).unwrap(), b"v1");
    assert!(old_target.exists());
    assert!(!new_target.exists());

    let record = state
        .get_resource_record(&ResourceId::parse("example/runtime").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(record.lifecycle, ResourceLifecycle::Active);
    assert_eq!(state.list_activation_records(&record.id).unwrap().len(), 1);
    assert_eq!(
        state.list_activation_records(&record.id).unwrap()[0].target,
        old_target
    );
}

#[test]
fn repeated_activation_switches_active_install() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let active_target = temp.path().join("active/runtime");

    for version in ["1.0.0", "1.1.0"] {
        let source_dir = temp.path().join(format!("src-{version}"));
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("runtime.bin"), version.as_bytes()).unwrap();

        let resource = RequestedResource::new(
            ResourceSpec::new(
                ResourceId::parse("example/runtime").unwrap(),
                ResourceLocator::LocalPath(source_dir.clone()),
            )
            .version(pulith_resource::VersionSelector::exact(version).unwrap()),
        )
        .resolve(
            ResolvedVersion::new(version).unwrap(),
            ResolvedLocator::LocalPath(source_dir.clone()),
            None,
        );

        let ready = InstallReady::new(state.clone());
        let spec = InstallSpec::new(
            resource,
            InstallInput::from_archive_extraction(
                source_dir.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: version.len() as u64,
                    entries: vec![],
                },
            ),
            temp.path().join("installs/runtime"),
        )
        .replace_existing()
        .activation(ActivationTarget {
            path: active_target.clone(),
        });

        PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .activate(&FileActivator)
            .unwrap()
            .finish();
    }

    let snapshot = state.load().unwrap();
    assert_eq!(snapshot.activations.len(), 2);
    assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Active);
    assert!(active_target.exists());
}

#[test]
fn symlink_activation_replaces_existing_directory_target_across_reinstalls() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let active_target = temp.path().join("active/runtime");

    for (version, payload) in [("1.0.0", b"v1".as_slice()), ("1.1.0", b"v2".as_slice())] {
        let source_dir = temp.path().join(format!("src-{version}"));
        fs::create_dir_all(&source_dir).unwrap();
        fs::write(source_dir.join("runtime.bin"), payload).unwrap();

        let resource = RequestedResource::new(
            ResourceSpec::new(
                ResourceId::parse("example/runtime").unwrap(),
                ResourceLocator::LocalPath(source_dir.clone()),
            )
            .version(pulith_resource::VersionSelector::exact(version).unwrap()),
        )
        .resolve(
            ResolvedVersion::new(version).unwrap(),
            ResolvedLocator::LocalPath(source_dir.clone()),
            None,
        );

        let spec = InstallSpec::new(
            resource,
            InstallInput::from_archive_extraction(
                source_dir.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: payload.len() as u64,
                    entries: vec![],
                },
            ),
            temp.path().join("installs/runtime"),
        )
        .replace_existing()
        .activation(ActivationTarget {
            path: active_target.clone(),
        });

        PlannedInstall::new(InstallReady::new(state.clone()), spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .activate(&SymlinkActivator)
            .unwrap()
            .finish();
    }

    assert_eq!(fs::read(active_target.join("runtime.bin")).unwrap(), b"v2");
}

#[cfg(windows)]
#[test]
fn replace_install_over_readonly_previous_content() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let install_root = temp.path().join("installs/runtime");

    fs::create_dir_all(&install_root).unwrap();
    let old_file = install_root.join("runtime.bin");
    fs::write(&old_file, b"v1").unwrap();
    let mut permissions = fs::metadata(&old_file).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&old_file, permissions).unwrap();

    let replacement_source = temp.path().join("src-replace");
    fs::create_dir_all(&replacement_source).unwrap();
    fs::write(replacement_source.join("runtime.bin"), b"v2").unwrap();

    let replacement_resource = RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(replacement_source.clone()),
        )
        .version(pulith_resource::VersionSelector::exact("1.1.0").unwrap()),
    )
    .resolve(
        ResolvedVersion::new("1.1.0").unwrap(),
        ResolvedLocator::LocalPath(replacement_source.clone()),
        None,
    );

    let receipt = PlannedInstall::new(
        InstallReady::new(state),
        InstallSpec::new(
            replacement_resource,
            InstallInput::from_archive_extraction(
                replacement_source.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: 2,
                    entries: vec![],
                },
            ),
            install_root.clone(),
        )
        .replace_existing(),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .finish();

    assert_eq!(receipt.install_root, install_root);
    assert_eq!(
        fs::read(receipt.install_root.join("runtime.bin")).unwrap(),
        b"v2"
    );
}

#[test]
fn upgrade_install_preserves_existing_active_state() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let active_target = temp.path().join("active/runtime");
    let install_root = temp.path().join("installs/runtime");

    let initial_source = temp.path().join("src-initial");
    fs::create_dir_all(&initial_source).unwrap();
    fs::write(initial_source.join("runtime.bin"), b"v1").unwrap();

    let initial_resource = RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(initial_source.clone()),
        )
        .version(pulith_resource::VersionSelector::exact("1.0.0").unwrap()),
    )
    .resolve(
        ResolvedVersion::new("1.0.0").unwrap(),
        ResolvedLocator::LocalPath(initial_source.clone()),
        None,
    );

    PlannedInstall::new(
        InstallReady::new(state.clone()),
        InstallSpec::new(
            initial_resource,
            InstallInput::from_archive_extraction(
                initial_source.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: 2,
                    entries: vec![],
                },
            ),
            install_root.clone(),
        )
        .activation(ActivationTarget {
            path: active_target.clone(),
        }),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&FileActivator)
    .unwrap()
    .finish();

    let upgraded_source = temp.path().join("src-upgrade");
    fs::create_dir_all(&upgraded_source).unwrap();
    fs::write(upgraded_source.join("runtime.bin"), b"v2").unwrap();

    let upgraded_resource = RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(upgraded_source.clone()),
        )
        .version(pulith_resource::VersionSelector::exact("1.1.0").unwrap()),
    )
    .resolve(
        ResolvedVersion::new("1.1.0").unwrap(),
        ResolvedLocator::LocalPath(upgraded_source.clone()),
        None,
    );

    PlannedInstall::new(
        InstallReady::new(state.clone()),
        InstallSpec::new(
            upgraded_resource,
            InstallInput::from_archive_extraction(
                upgraded_source.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: 2,
                    entries: vec![],
                },
            ),
            install_root.clone(),
        )
        .upgrade_existing(),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .finish();

    assert_eq!(fs::read(install_root.join("runtime.bin")).unwrap(), b"v2");
    assert_eq!(
        fs::read(&active_target).unwrap(),
        install_root.to_string_lossy().as_bytes()
    );

    let record = state
        .get_resource_record(&ResourceId::parse("example/runtime").unwrap())
        .unwrap()
        .unwrap();
    assert_eq!(record.lifecycle, ResourceLifecycle::Active);
    assert_eq!(state.list_activation_records(&record.id).unwrap().len(), 1);
}

#[test]
fn activated_replace_rollback_restores_previous_activation_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let install_root = temp.path().join("installs/runtime");
    let old_target = temp.path().join("active/runtime-old");
    let new_target = temp.path().join("active/runtime-new");

    let initial_source = temp.path().join("src-initial");
    fs::create_dir_all(&initial_source).unwrap();
    fs::write(initial_source.join("runtime.bin"), b"v1").unwrap();

    let initial_resource = RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(initial_source.clone()),
        )
        .version(pulith_resource::VersionSelector::exact("1.0.0").unwrap()),
    )
    .resolve(
        ResolvedVersion::new("1.0.0").unwrap(),
        ResolvedLocator::LocalPath(initial_source.clone()),
        None,
    );

    PlannedInstall::new(
        InstallReady::new(state.clone()),
        InstallSpec::new(
            initial_resource,
            InstallInput::from_archive_extraction(
                initial_source.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: 2,
                    entries: vec![],
                },
            ),
            install_root.clone(),
        )
        .activation(ActivationTarget {
            path: old_target.clone(),
        }),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&FileActivator)
    .unwrap()
    .finish();

    let replacement_source = temp.path().join("src-replace");
    fs::create_dir_all(&replacement_source).unwrap();
    fs::write(replacement_source.join("runtime.bin"), b"v2").unwrap();

    let replacement_resource = RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(replacement_source.clone()),
        )
        .version(pulith_resource::VersionSelector::exact("1.1.0").unwrap()),
    )
    .resolve(
        ResolvedVersion::new("1.1.0").unwrap(),
        ResolvedLocator::LocalPath(replacement_source.clone()),
        None,
    );

    let rollback = PlannedInstall::new(
        InstallReady::new(state.clone()),
        InstallSpec::new(
            replacement_resource,
            InstallInput::from_archive_extraction(
                replacement_source.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: 2,
                    entries: vec![],
                },
            ),
            install_root.clone(),
        )
        .replace_existing()
        .activation(ActivationTarget {
            path: new_target.clone(),
        }),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&FileActivator)
    .unwrap()
    .rollback()
    .unwrap();

    assert_eq!(rollback.restored_path, install_root);
    assert_eq!(fs::read(install_root.join("runtime.bin")).unwrap(), b"v1");
    assert!(old_target.exists());
    assert!(!new_target.exists());
    assert_eq!(
        fs::read(&old_target).unwrap(),
        install_root.to_string_lossy().as_bytes()
    );

    let resource_id = ResourceId::parse("example/runtime").unwrap();
    let record = state.get_resource_record(&resource_id).unwrap().unwrap();
    assert_eq!(record.lifecycle, ResourceLifecycle::Active);

    let activations = state.list_activation_records(&resource_id).unwrap();
    assert_eq!(activations.len(), 1);
    assert_eq!(activations[0].target, old_target);
}

#[test]
fn interrupted_install_recovery_restores_previous_snapshot() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state.clone());
    let install_root = temp.path().join("installs/runtime");

    let initial_source = temp.path().join("src-initial");
    fs::create_dir_all(&initial_source).unwrap();
    fs::write(initial_source.join("runtime.bin"), b"v1").unwrap();

    let initial_resource = RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(initial_source.clone()),
        )
        .version(pulith_resource::VersionSelector::exact("1.0.0").unwrap()),
    )
    .resolve(
        ResolvedVersion::new("1.0.0").unwrap(),
        ResolvedLocator::LocalPath(initial_source.clone()),
        None,
    );

    PlannedInstall::new(
        ready.clone(),
        InstallSpec::new(
            initial_resource,
            InstallInput::from_archive_extraction(
                initial_source.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: 2,
                    entries: vec![],
                },
            ),
            install_root.clone(),
        ),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .finish();

    let resource_id = ResourceId::parse("example/runtime").unwrap();
    let backup = ready
        .create_backup(&resource_id, &install_root, temp.path().join("backups"))
        .unwrap();

    fs::remove_dir_all(&install_root).unwrap();
    fs::create_dir_all(&install_root).unwrap();
    fs::write(install_root.join("runtime.bin"), b"partial").unwrap();
    state
        .patch_resource_record(
            &resource_id,
            ResourceRecordPatch::lifecycle(ResourceLifecycle::Failed),
        )
        .unwrap();

    ready.restore_backup(&backup).unwrap();

    assert_eq!(fs::read(install_root.join("runtime.bin")).unwrap(), b"v1");
    assert_eq!(
        state
            .get_resource_record(&resource_id)
            .unwrap()
            .unwrap()
            .lifecycle,
        ResourceLifecycle::Installed
    );
}

#[test]
fn recovery_contract_backup_restore_recovers_install_and_state_facts() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state.clone());
    let install_root = temp.path().join("installs/runtime");
    let activation_target = temp.path().join("active/runtime");

    let initial_source = temp.path().join("src-initial");
    fs::create_dir_all(&initial_source).unwrap();
    fs::write(initial_source.join("runtime.bin"), b"v1").unwrap();

    let initial_resource = RequestedResource::new(
        ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(initial_source.clone()),
        )
        .version(pulith_resource::VersionSelector::exact("1.0.0").unwrap()),
    )
    .resolve(
        ResolvedVersion::new("1.0.0").unwrap(),
        ResolvedLocator::LocalPath(initial_source.clone()),
        None,
    );

    PlannedInstall::new(
        ready.clone(),
        InstallSpec::new(
            initial_resource,
            InstallInput::from_archive_extraction(
                initial_source.clone(),
                pulith_archive::ArchiveReport {
                    format: pulith_archive::ArchiveFormat::Zip,
                    entry_count: 1,
                    total_bytes: 2,
                    entries: vec![],
                },
            ),
            install_root.clone(),
        )
        .activation(ActivationTarget {
            path: activation_target.clone(),
        }),
    )
    .stage()
    .unwrap()
    .commit()
    .unwrap()
    .activate(&FileActivator)
    .unwrap()
    .finish();

    let resource_id = ResourceId::parse("example/runtime").unwrap();
    let before = state.load().unwrap();
    let backup = ready
        .create_backup(&resource_id, &install_root, temp.path().join("backups"))
        .unwrap();

    fs::remove_dir_all(&install_root).unwrap();
    fs::create_dir_all(&install_root).unwrap();
    fs::write(install_root.join("runtime.bin"), b"partial").unwrap();
    state
        .patch_resource_record(
            &resource_id,
            ResourceRecordPatch::lifecycle(ResourceLifecycle::Failed),
        )
        .unwrap();
    state.remove_activation_records(&resource_id).unwrap();

    ready.restore_backup(&backup).unwrap();

    let after = state.load().unwrap();
    assert_eq!(fs::read(install_root.join("runtime.bin")).unwrap(), b"v1");
    assert_eq!(after, before);
}

#[test]
fn activation_contract_replace_updates_target_and_history_cross_platform() {
    let temp = tempfile::tempdir().unwrap();
    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let activation_target = temp.path().join("active/runtime.exe");

    for (version, payload) in [
        ("1.0.0", b"runtime-v1".as_slice()),
        ("1.1.0", b"runtime-v2".as_slice()),
    ] {
        let source_dir = temp.path().join(format!("runtime-src-{version}"));
        write_runtime_tree(&source_dir, "bin/runtime.exe", payload);

        let resource =
            resolved_resource_version(ResourceLocator::LocalPath(source_dir.clone()), version);

        PlannedInstall::new(
            InstallReady::new(state.clone()),
            InstallSpec::new(
                resource,
                InstallInput::from_archive_extraction(
                    source_dir.clone(),
                    archive_report(payload.len() as u64),
                ),
                temp.path().join("installs/runtime"),
            )
            .replace_existing()
            .activation(ActivationTarget {
                path: activation_target.clone(),
            }),
        )
        .stage()
        .unwrap()
        .commit()
        .unwrap()
        .activate(&ShimCopyActivator::new(
            ShimCommand::new("runtime", "bin/runtime.exe").unwrap(),
        ))
        .unwrap()
        .finish();
    }

    let resource_id = ResourceId::parse("example/runtime").unwrap();
    let activations = state.list_activation_records(&resource_id).unwrap();
    assert_eq!(fs::read(&activation_target).unwrap(), b"runtime-v2");
    assert_eq!(activations.len(), 2);
    assert!(
        activations
            .iter()
            .all(|record| record.target == activation_target)
    );
    assert_eq!(
        state
            .get_resource_record(&resource_id)
            .unwrap()
            .unwrap()
            .lifecycle,
        ResourceLifecycle::Active
    );
}
