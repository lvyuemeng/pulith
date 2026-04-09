use std::fs;
use std::io::{Cursor, Write};

use pulith_archive::{ExtractOptions, extract_from_reader};
use pulith_fetch::{Fetcher, MultiSourceFetcher, ReqwestClient};
use pulith_install::{
    ActivationReceipt, ActivationRequest, ActivationTarget, Activator, InstallInput, InstallReady,
    InstallSpec, PlannedInstall, SymlinkActivator,
};
use pulith_resource::{
    RequestedResource, ResolvedLocator, ResolvedVersion, ResourceId, ResourceLocator, ResourceSpec,
    ValidUrl,
};
use pulith_source::{SelectionStrategy, SourceSpec};
use pulith_state::{ResourceLifecycle, StateReady};
use pulith_store::{StoreKey, StoreReady, StoreRoots};

fn resolved_resource(locator: ResourceLocator) -> pulith_resource::ResolvedResource {
    RequestedResource::new(
        ResourceSpec::new(ResourceId::parse("example/runtime").unwrap(), locator)
            .version(pulith_resource::VersionSelector::exact("1.0.0").unwrap()),
    )
    .resolve(
        ResolvedVersion::new("1.0.0").unwrap(),
        ResolvedLocator::LocalPath(std::path::PathBuf::from("/local/runtime")),
        None,
    )
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
    let planned = SourceSpec::from_locator(&resource.spec().locator)
        .unwrap()
        .plan(SelectionStrategy::OrderedFallback);

    let fetcher = Fetcher::new(
        ReqwestClient::new().unwrap(),
        temp.path().join("fetch-workspace"),
    );
    let multi = MultiSourceFetcher::new(std::sync::Arc::new(fetcher));
    let fetched = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(async {
            multi
                .fetch_planned_sources_with_receipt(
                    &planned,
                    &temp.path().join("downloads/runtime.bin"),
                    &pulith_fetch::FetchOptions::default(),
                )
                .await
        })
        .unwrap();

    let store = StoreReady::initialize(StoreRoots::new(
        temp.path().join("store/artifacts"),
        temp.path().join("store/extracts"),
        temp.path().join("store/metadata"),
    ))
    .unwrap();
    let stored = store
        .import_artifact(
            &StoreKey::logical("runtime-bin").unwrap(),
            &fetched.destination,
        )
        .unwrap();

    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state.clone());
    let receipt = PlannedInstall::new(
        ready,
        InstallSpec::new(
            resource,
            InstallInput::StoredArtifact {
                artifact: stored,
                file_name: "runtime.bin".to_string(),
            },
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
    let extracted = store
        .register_extract_dir(
            &StoreKey::logical("runtime-extract").unwrap(),
            &extract_root,
        )
        .unwrap();

    let state = StateReady::initialize(temp.path().join("state/state.json")).unwrap();
    let ready = InstallReady::new(state.clone());
    let receipt = PlannedInstall::new(
        ready,
        InstallSpec::new(
            resolved_resource(ResourceLocator::Url(
                ValidUrl::parse("https://example.com/runtime.zip").unwrap(),
            )),
            InstallInput::ExtractedArtifact(extracted),
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
