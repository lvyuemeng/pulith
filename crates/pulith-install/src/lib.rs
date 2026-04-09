//! Composable installation workflow primitives for Pulith.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pulith_archive::entry::ArchiveReport;
use pulith_fetch::FetchReceipt;
use pulith_fs::{
    DEFAULT_COPY_ONLY_THRESHOLD_BYTES, FallBack, HardlinkOrCopyOptions, Workspace, atomic_symlink,
    copy_dir_all,
};
use pulith_resource::{Metadata, ResolvedResource};
use pulith_shim::TargetResolver;
use pulith_state::{ActivationRecord, ResourceLifecycle, ResourceRecord, StateReady};
use pulith_store::{ExtractedArtifact, StoreKey, StoreReady, StoredArtifact};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, InstallError>;

#[derive(Debug, Error)]
pub enum InstallError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fs(#[from] pulith_fs::Error),
    #[error(transparent)]
    State(#[from] pulith_state::StateError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
    #[error(transparent)]
    Store(#[from] pulith_store::StoreError),
    #[error(transparent)]
    Resource(#[from] pulith_resource::ResourceError),
    #[error("artifact file name must not be empty")]
    EmptyFileName,
    #[error("extracted artifact path does not exist: {0}")]
    MissingExtractedArtifact(PathBuf),
    #[error("stored artifact path does not exist: {0}")]
    MissingStoredArtifact(PathBuf),
    #[error("activation target was not configured")]
    MissingActivationTarget,
    #[error("install root already exists: {0}")]
    ExistingInstall(PathBuf),
    #[error("no rollback snapshot is available")]
    RollbackUnavailable,
    #[error("shim command must not be empty")]
    EmptyShimCommand,
    #[error("shim target `{0}` is not resolvable")]
    UnresolvedShimTarget(String),
    #[error("install root does not exist for backup: {0}")]
    MissingInstallForBackup(PathBuf),
}

#[derive(Debug, Clone)]
pub struct InstallReady {
    state: StateReady,
}

impl InstallReady {
    pub fn new(state: StateReady) -> Self {
        Self { state }
    }

    pub fn state(&self) -> &StateReady {
        &self.state
    }

    pub fn create_backup(
        &self,
        id: &pulith_resource::ResourceId,
        install_root: impl AsRef<Path>,
        backup_root: impl AsRef<Path>,
    ) -> Result<BackupReceipt> {
        let install_root = install_root.as_ref().to_path_buf();
        if !install_root.exists() {
            return Err(InstallError::MissingInstallForBackup(install_root));
        }

        let created_at_unix = now_unix();
        let backup_root = backup_root
            .as_ref()
            .join(sanitize_component(&id.as_string()))
            .join(created_at_unix.to_string());
        let install_snapshot = backup_root.join("install");
        let state_snapshot = backup_root.join("state.json");

        std::fs::create_dir_all(&backup_root)?;
        copy_dir_all(&install_root, &install_snapshot)?;

        let snapshot = self.state.load()?;
        let payload = BackupState {
            resource: snapshot
                .resources
                .into_iter()
                .find(|record| &record.id == id),
            activations: snapshot
                .activations
                .into_iter()
                .filter(|record| &record.id == id)
                .collect(),
        };
        std::fs::write(&state_snapshot, serde_json::to_vec_pretty(&payload)?)?;

        Ok(BackupReceipt {
            resource: id.clone(),
            install_root,
            backup_root,
            install_snapshot,
            state_snapshot,
            created_at_unix,
        })
    }

    pub fn restore_backup(&self, backup: &BackupReceipt) -> Result<RestoreReceipt> {
        if backup.install_root.exists() {
            remove_existing_target(&backup.install_root)?;
        }
        copy_dir_all(&backup.install_snapshot, &backup.install_root)?;

        let payload: BackupState = serde_json::from_slice(&std::fs::read(&backup.state_snapshot)?)?;
        self.state.remove_resource_record(&backup.resource)?;
        self.state.remove_activation_records(&backup.resource)?;
        if let Some(record) = payload.resource {
            self.state.upsert_resource_record(record)?;
        }
        for activation in payload.activations {
            self.state.append_activation(activation)?;
        }

        Ok(RestoreReceipt {
            resource: backup.resource.clone(),
            restored_install_root: backup.install_root.clone(),
            backup_root: backup.backup_root.clone(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BackupReceipt {
    pub resource: pulith_resource::ResourceId,
    pub install_root: PathBuf,
    pub backup_root: PathBuf,
    pub install_snapshot: PathBuf,
    pub state_snapshot: PathBuf,
    pub created_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreReceipt {
    pub resource: pulith_resource::ResourceId,
    pub restored_install_root: PathBuf,
    pub backup_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct BackupState {
    resource: Option<ResourceRecord>,
    activations: Vec<ActivationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationTarget {
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationReceipt {
    pub target: PathBuf,
    pub installed_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShimCommand {
    pub command: String,
    pub relative_target: PathBuf,
}

impl ShimCommand {
    pub fn new(command: impl Into<String>, relative_target: impl Into<PathBuf>) -> Result<Self> {
        let command = command.into();
        if command.is_empty() {
            return Err(InstallError::EmptyShimCommand);
        }
        Ok(Self {
            command,
            relative_target: relative_target.into(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct InstalledShimResolver {
    install_root: PathBuf,
    commands: Vec<ShimCommand>,
}

impl InstalledShimResolver {
    pub fn new(install_root: impl Into<PathBuf>, commands: Vec<ShimCommand>) -> Self {
        Self {
            install_root: install_root.into(),
            commands,
        }
    }
}

impl TargetResolver for InstalledShimResolver {
    fn resolve(&self, command: &str) -> Option<PathBuf> {
        self.commands.iter().find_map(|binding| {
            (binding.command == command).then(|| self.install_root.join(&binding.relative_target))
        })
    }
}

#[derive(Debug, Clone)]
pub struct ShimLinkActivator {
    command: ShimCommand,
}

impl ShimLinkActivator {
    pub fn new(command: ShimCommand) -> Self {
        Self { command }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InstallMode {
    #[default]
    CreateOnly,
    Replace,
    Upgrade,
}

#[derive(Debug, Clone)]
pub enum InstallInput {
    FetchedArtifact {
        receipt: FetchReceipt,
        file_name: Option<String>,
    },
    StoredArtifact {
        artifact: StoredArtifact,
        file_name: String,
    },
    ExtractedArtifact(ExtractedArtifact),
    ExtractedTree {
        root: PathBuf,
        report: Option<ArchiveReport>,
    },
}

impl InstallInput {
    pub fn from_fetch_receipt(receipt: FetchReceipt) -> Self {
        Self::FetchedArtifact {
            receipt,
            file_name: None,
        }
    }

    pub fn from_stored_artifact(artifact: StoredArtifact) -> Result<Self> {
        let file_name = file_name_from_path(&artifact.path).ok_or(InstallError::EmptyFileName)?;
        Ok(Self::StoredArtifact {
            artifact,
            file_name,
        })
    }

    pub fn store_fetched_artifact(
        store: &StoreReady,
        key: &StoreKey,
        receipt: &FetchReceipt,
    ) -> Result<Self> {
        let artifact = store.import_artifact(key, &receipt.destination)?;
        Self::from_stored_artifact(artifact)
    }

    pub fn from_archive_extraction(root: PathBuf, report: ArchiveReport) -> Self {
        Self::ExtractedTree {
            root,
            report: Some(report),
        }
    }

    fn store_key(&self) -> Option<&StoreKey> {
        match self {
            Self::FetchedArtifact { .. } => None,
            Self::StoredArtifact { artifact, .. } => Some(&artifact.key),
            Self::ExtractedArtifact(artifact) => Some(&artifact.key),
            Self::ExtractedTree { .. } => None,
        }
    }

    fn stage_into(&self, workspace: &Workspace) -> Result<()> {
        match self {
            Self::FetchedArtifact { receipt, file_name } => {
                if !receipt.destination.exists() {
                    return Err(InstallError::MissingStoredArtifact(
                        receipt.destination.clone(),
                    ));
                }

                let target_name = file_name
                    .clone()
                    .or_else(|| file_name_from_path(&receipt.destination));
                let target_name = target_name.ok_or(InstallError::EmptyFileName)?;

                stage_workspace_file(workspace, &receipt.destination, Path::new(&target_name))?;
            }
            Self::ExtractedArtifact(artifact) => {
                if !artifact.path.exists() {
                    return Err(InstallError::MissingExtractedArtifact(
                        artifact.path.clone(),
                    ));
                }
                copy_directory_into_workspace(workspace, &artifact.path, Path::new(""))?;
            }
            Self::ExtractedTree { root, .. } => {
                if !root.exists() {
                    return Err(InstallError::MissingExtractedArtifact(root.clone()));
                }
                copy_directory_into_workspace(workspace, root, Path::new(""))?;
            }
            Self::StoredArtifact {
                artifact,
                file_name,
            } => {
                if file_name.is_empty() {
                    return Err(InstallError::EmptyFileName);
                }
                if !artifact.path.exists() {
                    return Err(InstallError::MissingStoredArtifact(artifact.path.clone()));
                }
                stage_workspace_file(workspace, &artifact.path, Path::new(file_name))?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct InstallSpec {
    pub resource: ResolvedResource,
    pub input: InstallInput,
    pub install_root: PathBuf,
    pub mode: InstallMode,
    pub activation: Option<ActivationTarget>,
    pub metadata: Metadata,
}

impl InstallSpec {
    pub fn new(resource: ResolvedResource, input: InstallInput, install_root: PathBuf) -> Self {
        Self {
            resource,
            input,
            install_root,
            mode: InstallMode::CreateOnly,
            activation: None,
            metadata: Metadata::new(),
        }
    }

    pub fn replace_existing(mut self) -> Self {
        self.mode = InstallMode::Replace;
        self
    }

    pub fn upgrade_existing(mut self) -> Self {
        self.mode = InstallMode::Upgrade;
        self
    }

    pub fn activation(mut self, activation: ActivationTarget) -> Self {
        self.activation = Some(activation);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Planned;

pub struct Staged {
    _temp_dir: tempfile::TempDir,
    workspace: Workspace,
}

pub struct Installed {
    pub install_root: PathBuf,
    rollback: Option<RollbackState>,
}

struct RollbackState {
    _temp_dir: tempfile::TempDir,
    backup_path: PathBuf,
    previous_record: Option<ResourceRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activated {
    pub install_root: PathBuf,
    pub activation: ActivationReceipt,
    pub replaced_previous: bool,
}

#[derive(Debug)]
pub struct InstallFlow<S> {
    ready: InstallReady,
    spec: InstallSpec,
    state: S,
}

pub type PlannedInstall = InstallFlow<Planned>;
pub type StagedInstall = InstallFlow<Staged>;
pub type InstalledInstall = InstallFlow<Installed>;
pub type ActivatedInstall = InstallFlow<Activated>;

impl PlannedInstall {
    pub fn new(ready: InstallReady, spec: InstallSpec) -> Self {
        Self {
            ready,
            spec,
            state: Planned,
        }
    }

    pub fn stage(self) -> Result<StagedInstall> {
        self.spec.resource.validate_version_selection()?;
        let temp = tempfile::tempdir()?;
        let workspace =
            Workspace::new(temp.path().join("staging"), self.spec.install_root.clone())?;
        self.spec.input.stage_into(&workspace)?;

        Ok(InstallFlow {
            ready: self.ready,
            spec: self.spec,
            state: Staged {
                _temp_dir: temp,
                workspace,
            },
        })
    }
}

impl StagedInstall {
    pub fn commit(self) -> Result<InstalledInstall> {
        let install_root = self.spec.install_root.clone();
        if let Some(parent) = install_root.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let rollback = self.prepare_rollback_state()?;
        if let Err(error) = self.state.workspace.commit() {
            if let Some(rollback) = rollback.as_ref() {
                restore_backup(&rollback.backup_path, &install_root)?;
            }
            return Err(error.into());
        }

        let record = self
            .spec
            .resource_record(ResourceLifecycle::Installed, Some(install_root.clone()));
        self.ready.state().upsert_resource_record(record)?;

        Ok(InstallFlow {
            ready: self.ready,
            spec: self.spec,
            state: Installed {
                install_root,
                rollback,
            },
        })
    }

    fn prepare_rollback_state(&self) -> Result<Option<RollbackState>> {
        if !self.spec.install_root.exists() {
            return Ok(None);
        }

        if self.spec.mode == InstallMode::CreateOnly {
            return Err(InstallError::ExistingInstall(
                self.spec.install_root.clone(),
            ));
        }

        let temp_dir = tempfile::tempdir()?;
        let backup_path = temp_dir.path().join("previous-install");
        std::fs::rename(&self.spec.install_root, &backup_path)?;

        let previous_record = self
            .ready
            .state()
            .load()?
            .resources
            .into_iter()
            .find(|record| record.id == self.spec.resource.spec().id);

        Ok(Some(RollbackState {
            _temp_dir: temp_dir,
            backup_path,
            previous_record,
        }))
    }
}

impl InstalledInstall {
    pub fn activate<A: Activator>(self, activator: &A) -> Result<ActivatedInstall> {
        let target = self
            .spec
            .activation
            .clone()
            .ok_or(InstallError::MissingActivationTarget)?;

        let request = ActivationRequest {
            resource: self.spec.resource.spec().id.clone(),
            installed_path: self.state.install_root.clone(),
            target: target.path.clone(),
        };
        let activation = activator.activate(&request)?;

        self.ready
            .state()
            .upsert_resource_record(self.spec.resource_record(
                ResourceLifecycle::Active,
                Some(self.state.install_root.clone()),
            ))?;

        self.ready.state().append_activation(ActivationRecord {
            id: self.spec.resource.spec().id.clone(),
            target: activation.target.clone(),
            activated_at_unix: now_unix(),
        })?;

        Ok(InstallFlow {
            ready: self.ready,
            spec: self.spec,
            state: Activated {
                install_root: self.state.install_root,
                activation,
                replaced_previous: self.state.rollback.is_some(),
            },
        })
    }

    pub fn rollback(self) -> Result<RollbackReceipt> {
        let rollback = self
            .state
            .rollback
            .ok_or(InstallError::RollbackUnavailable)?;

        if self.state.install_root.exists() {
            remove_existing_target(&self.state.install_root)?;
        }
        restore_backup(&rollback.backup_path, &self.state.install_root)?;

        if let Some(previous_record) = rollback.previous_record {
            self.ready.state().upsert_resource_record(previous_record)?;
        } else {
            self.ready
                .state()
                .remove_resource_record(&self.spec.resource.spec().id)?;
        }

        Ok(RollbackReceipt {
            resource: self.spec.resource.spec().id.clone(),
            restored_path: self.state.install_root,
        })
    }

    pub fn finish(self) -> InstallReceipt {
        InstallReceipt {
            resource: self.spec.resource.spec().id.clone(),
            install_root: self.state.install_root,
            activation: None,
            replaced_previous: self.state.rollback.is_some(),
        }
    }
}

impl ActivatedInstall {
    pub fn finish(self) -> InstallReceipt {
        InstallReceipt {
            resource: self.spec.resource.spec().id.clone(),
            install_root: self.state.install_root,
            activation: Some(self.state.activation),
            replaced_previous: self.state.replaced_previous,
        }
    }
}

impl<S> InstallFlow<S> {
    pub fn spec(&self) -> &InstallSpec {
        &self.spec
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallReceipt {
    pub resource: pulith_resource::ResourceId,
    pub install_root: PathBuf,
    pub activation: Option<ActivationReceipt>,
    pub replaced_previous: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RollbackReceipt {
    pub resource: pulith_resource::ResourceId,
    pub restored_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivationRequest {
    pub resource: pulith_resource::ResourceId,
    pub installed_path: PathBuf,
    pub target: PathBuf,
}

pub trait Activator {
    fn activate(&self, request: &ActivationRequest) -> Result<ActivationReceipt>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SymlinkActivator;

impl Activator for SymlinkActivator {
    fn activate(&self, request: &ActivationRequest) -> Result<ActivationReceipt> {
        link_activation_target(&request.installed_path, &request.target)
    }
}

impl Activator for ShimLinkActivator {
    fn activate(&self, request: &ActivationRequest) -> Result<ActivationReceipt> {
        let resolver =
            InstalledShimResolver::new(request.installed_path.clone(), vec![self.command.clone()]);
        let resolved = resolver
            .resolve(&self.command.command)
            .ok_or_else(|| InstallError::UnresolvedShimTarget(self.command.command.clone()))?;

        link_activation_target(&resolved, &request.target)
    }
}

impl InstallSpec {
    fn resource_record(
        &self,
        lifecycle: ResourceLifecycle,
        install_path: Option<PathBuf>,
    ) -> ResourceRecord {
        ResourceRecord {
            id: self.resource.spec().id.clone(),
            selector: self.resource.spec().version.clone(),
            resolved_version: Some(self.resource.version().clone()),
            locator: Some(self.resource.locator().clone()),
            artifact_key: self.input.store_key().cloned(),
            install_path,
            lifecycle,
            metadata: self.metadata.clone(),
        }
    }
}

fn remove_existing_target(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn link_activation_target(installed_path: &Path, target: &Path) -> Result<ActivationReceipt> {
    if target.exists() {
        remove_existing_target(target)?;
    }
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent)?;
    }
    atomic_symlink(installed_path, target)?;
    Ok(ActivationReceipt {
        target: target.to_path_buf(),
        installed_path: installed_path.to_path_buf(),
    })
}

fn restore_backup(backup_path: &Path, install_root: &Path) -> Result<()> {
    if let Some(parent) = install_root.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::rename(backup_path, install_root)?;
    Ok(())
}

fn file_name_from_path(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn default_link_options() -> HardlinkOrCopyOptions {
    HardlinkOrCopyOptions::new().fallback(FallBack::Copy)
}

fn stage_workspace_file(workspace: &Workspace, source: &Path, relative_path: &Path) -> Result<()> {
    workspace.stage_file_by_size(
        source,
        relative_path,
        DEFAULT_COPY_ONLY_THRESHOLD_BYTES,
        default_link_options(),
    )?;
    Ok(())
}

fn copy_directory_into_workspace(
    workspace: &Workspace,
    source: &Path,
    relative: &Path,
) -> Result<()> {
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let relative_path = relative.join(name);
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            workspace.create_dir_all(&relative_path)?;
            copy_directory_into_workspace(workspace, &path, &relative_path)?;
        } else {
            stage_workspace_file(workspace, &path, &relative_path)?;
        }
    }
    Ok(())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn sanitize_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulith_archive::{ArchiveFormat, ArchiveReport};
    use pulith_fetch::{FetchReceipt, FetchSource};
    use pulith_resource::{
        RequestedResource, ResolvedLocator, ResolvedVersion, ResourceId, ResourceLocator,
        ResourceSpec, ValidUrl,
    };
    use pulith_state::StateSnapshot;
    use pulith_store::StoreRoots;

    fn resolved_resource() -> ResolvedResource {
        RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
        ))
        .resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(
                ValidUrl::parse("https://mirror.example.com/runtime.zip").unwrap(),
            ),
            None,
        )
    }

    #[test]
    fn extracted_artifact_install_commits_and_updates_state() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("extract");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("tool.exe"), b"payload").unwrap();

        let extracted = ExtractedArtifact {
            key: StoreKey::logical("runtime").unwrap(),
            path: source_dir,
            provenance: None,
        };
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let ready = InstallReady::new(state.clone());
        let spec = InstallSpec::new(
            resolved_resource(),
            InstallInput::ExtractedArtifact(extracted),
            temp.path().join("install/runtime"),
        );

        let receipt = PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .finish();

        assert!(receipt.install_root.join("tool.exe").exists());
        let snapshot = state.load().unwrap();
        assert_eq!(
            snapshot.resources[0].lifecycle,
            ResourceLifecycle::Installed
        );
    }

    #[test]
    fn stored_artifact_install_places_named_file() {
        let temp = tempfile::tempdir().unwrap();
        let artifact_path = temp.path().join("archive.bin");
        std::fs::write(&artifact_path, b"payload").unwrap();

        let stored = StoredArtifact {
            key: StoreKey::logical("archive").unwrap(),
            path: artifact_path,
            provenance: None,
        };

        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let ready = InstallReady::new(state);
        let spec = InstallSpec::new(
            resolved_resource(),
            InstallInput::StoredArtifact {
                artifact: stored,
                file_name: "runtime.zip".to_string(),
            },
            temp.path().join("install/archive"),
        );

        let receipt = PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .finish();

        assert!(receipt.install_root.join("runtime.zip").exists());
    }

    #[test]
    fn install_input_from_stored_artifact_uses_artifact_file_name() {
        let temp = tempfile::tempdir().unwrap();
        let artifact_path = temp.path().join("archive.bin");
        std::fs::write(&artifact_path, b"payload").unwrap();

        let stored = StoredArtifact {
            key: StoreKey::logical("archive").unwrap(),
            path: artifact_path,
            provenance: None,
        };

        let input = InstallInput::from_stored_artifact(stored).unwrap();
        match input {
            InstallInput::StoredArtifact { file_name, .. } => assert_eq!(file_name, "archive.bin"),
            _ => panic!("expected stored artifact install input"),
        }
    }

    #[test]
    fn install_input_store_fetched_artifact_bridges_fetch_to_store() {
        let temp = tempfile::tempdir().unwrap();
        let fetched_path = temp.path().join("downloads/runtime.bin");
        std::fs::create_dir_all(fetched_path.parent().unwrap()).unwrap();
        std::fs::write(&fetched_path, b"payload").unwrap();

        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();
        let key = StoreKey::logical("runtime").unwrap();

        let input = InstallInput::store_fetched_artifact(
            &store,
            &key,
            &FetchReceipt {
                source: FetchSource::Url("https://example.com/runtime.bin".to_string()),
                destination: fetched_path,
                bytes_downloaded: 7,
                total_bytes: Some(7),
                sha256_hex: None,
            },
        )
        .unwrap();

        match input {
            InstallInput::StoredArtifact {
                artifact,
                file_name,
            } => {
                assert!(artifact.path.exists());
                assert_eq!(artifact.key, key);
                assert_eq!(file_name, "runtime.bin");
            }
            _ => panic!("expected stored artifact install input"),
        }
    }

    #[test]
    fn fetched_artifact_install_places_receipt_file() {
        let temp = tempfile::tempdir().unwrap();
        let fetched_path = temp.path().join("fetched.bin");
        std::fs::write(&fetched_path, b"payload").unwrap();

        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let ready = InstallReady::new(state);
        let spec = InstallSpec::new(
            resolved_resource(),
            InstallInput::from_fetch_receipt(FetchReceipt {
                source: FetchSource::Url("https://example.com/runtime.bin".to_string()),
                destination: fetched_path,
                bytes_downloaded: 7,
                total_bytes: Some(7),
                sha256_hex: None,
            }),
            temp.path().join("install/fetched"),
        );

        let receipt = PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .finish();

        assert!(receipt.install_root.join("fetched.bin").exists());
    }

    #[test]
    fn install_stage_rejects_version_selector_mismatch() {
        let temp = tempfile::tempdir().unwrap();
        let fetched_path = temp.path().join("fetched.bin");
        std::fs::write(&fetched_path, b"payload").unwrap();

        let resource = RequestedResource::new(
            ResourceSpec::new(
                ResourceId::parse("example/runtime").unwrap(),
                ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.bin").unwrap()),
            )
            .version(pulith_resource::VersionSelector::requirement("^1.2").unwrap()),
        )
        .resolve(
            ResolvedVersion::new("2.0.0").unwrap(),
            ResolvedLocator::Url(ValidUrl::parse("https://example.com/runtime.bin").unwrap()),
            None,
        );

        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let ready = InstallReady::new(state);
        let spec = InstallSpec::new(
            resource,
            InstallInput::from_fetch_receipt(FetchReceipt {
                source: FetchSource::Url("https://example.com/runtime.bin".to_string()),
                destination: fetched_path,
                bytes_downloaded: 7,
                total_bytes: Some(7),
                sha256_hex: None,
            }),
            temp.path().join("install/fetched"),
        );

        assert!(matches!(
            PlannedInstall::new(ready, spec).stage(),
            Err(InstallError::Resource(
                pulith_resource::ResourceError::ResolvedVersionMismatch { .. }
            ))
        ));
    }

    #[test]
    fn archive_extraction_input_installs_tree() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("extract-tree");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("tool.exe"), b"payload").unwrap();

        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 1,
            total_bytes: 7,
            entries: vec![],
        };

        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let ready = InstallReady::new(state);
        let spec = InstallSpec::new(
            resolved_resource(),
            InstallInput::from_archive_extraction(root, report),
            temp.path().join("install/from-archive"),
        );

        let receipt = PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .finish();

        assert!(receipt.install_root.join("tool.exe").exists());
    }

    #[test]
    fn replace_existing_install_marks_receipt_and_swaps_content() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::write(install_root.join("tool.exe"), b"old").unwrap();

        let source_dir = temp.path().join("extract-new");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("tool.exe"), b"new").unwrap();

        let extracted = ExtractedArtifact {
            key: StoreKey::logical("runtime").unwrap(),
            path: source_dir,
            provenance: None,
        };
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let ready = InstallReady::new(state);
        let spec = InstallSpec::new(
            resolved_resource(),
            InstallInput::ExtractedArtifact(extracted),
            install_root,
        )
        .replace_existing();

        let receipt = PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .finish();

        assert!(receipt.replaced_previous);
        assert_eq!(
            std::fs::read(receipt.install_root.join("tool.exe")).unwrap(),
            b"new"
        );
    }

    #[test]
    fn rollback_restores_previous_install() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::write(install_root.join("tool.exe"), b"old").unwrap();

        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        state
            .upsert_resource_record(ResourceRecord {
                id: ResourceId::parse("example/runtime").unwrap(),
                selector: resolved_resource().spec().version.clone(),
                resolved_version: Some(resolved_resource().version().clone()),
                locator: Some(resolved_resource().locator().clone()),
                artifact_key: None,
                install_path: Some(install_root.clone()),
                lifecycle: ResourceLifecycle::Installed,
                metadata: Metadata::new(),
            })
            .unwrap();

        let source_dir = temp.path().join("extract-new");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("tool.exe"), b"new").unwrap();

        let extracted = ExtractedArtifact {
            key: StoreKey::logical("runtime").unwrap(),
            path: source_dir,
            provenance: None,
        };
        let ready = InstallReady::new(state.clone());
        let spec = InstallSpec::new(
            resolved_resource(),
            InstallInput::ExtractedArtifact(extracted),
            install_root.clone(),
        )
        .replace_existing();

        let rollback = PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .rollback()
            .unwrap();

        assert_eq!(rollback.restored_path, install_root);
        assert_eq!(
            std::fs::read(rollback.restored_path.join("tool.exe")).unwrap(),
            b"old"
        );
    }

    #[test]
    fn activation_records_state() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("extract");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("bin"), b"payload").unwrap();

        let extracted = ExtractedArtifact {
            key: StoreKey::logical("runtime").unwrap(),
            path: source_dir,
            provenance: None,
        };
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let ready = InstallReady::new(state.clone());
        let spec = InstallSpec::new(
            resolved_resource(),
            InstallInput::ExtractedArtifact(extracted),
            temp.path().join("install/runtime"),
        )
        .activation(ActivationTarget {
            path: temp.path().join("active/runtime"),
        });

        let receipt = PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .activate(&SymlinkActivator)
            .unwrap()
            .finish();

        assert!(receipt.activation.is_some());
        let snapshot: StateSnapshot = state.load().unwrap();
        assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Active);
        assert_eq!(snapshot.activations.len(), 1);
    }

    #[test]
    fn shim_link_activator_targets_relative_executable() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(install_root.join("bin")).unwrap();
        std::fs::write(install_root.join("bin/tool.exe"), b"payload").unwrap();

        let request = ActivationRequest {
            resource: ResourceId::parse("example/runtime").unwrap(),
            installed_path: install_root.clone(),
            target: temp.path().join("active/tool"),
        };

        let activator = ShimLinkActivator::new(ShimCommand::new("tool", "bin/tool.exe").unwrap());
        let receipt = activator.activate(&request).unwrap();

        assert_eq!(receipt.installed_path, install_root.join("bin/tool.exe"));
        assert!(receipt.target.exists());
    }

    #[test]
    fn backup_and_restore_round_trip_install_and_state() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::write(install_root.join("tool.exe"), b"v1").unwrap();

        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let resource_id = ResourceId::parse("example/runtime").unwrap();
        state
            .upsert_resource_record(ResourceRecord {
                id: resource_id.clone(),
                selector: resolved_resource().spec().version.clone(),
                resolved_version: Some(resolved_resource().version().clone()),
                locator: Some(resolved_resource().locator().clone()),
                artifact_key: None,
                install_path: Some(install_root.clone()),
                lifecycle: ResourceLifecycle::Installed,
                metadata: Metadata::new(),
            })
            .unwrap();
        state
            .record_activation(&resource_id, temp.path().join("active/runtime"))
            .unwrap();

        let ready = InstallReady::new(state.clone());
        let backup = ready
            .create_backup(&resource_id, &install_root, temp.path().join("backups"))
            .unwrap();

        std::fs::write(install_root.join("tool.exe"), b"v2").unwrap();
        state.remove_resource_record(&resource_id).unwrap();
        state.remove_activation_records(&resource_id).unwrap();

        let restore = ready.restore_backup(&backup).unwrap();

        assert_eq!(restore.restored_install_root, install_root);
        assert_eq!(
            std::fs::read(restore.restored_install_root.join("tool.exe")).unwrap(),
            b"v1"
        );
        assert_eq!(
            state
                .get_resource_record(&resource_id)
                .unwrap()
                .unwrap()
                .lifecycle,
            ResourceLifecycle::Installed
        );
        assert_eq!(
            state.list_activation_records(&resource_id).unwrap().len(),
            1
        );
    }
}
