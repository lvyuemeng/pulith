//! Composable installation workflow primitives for Pulith.
//!
//! Contract highlights:
//! - Replace/upgrade flows capture a previous-install snapshot and can roll back within that scope.
//! - Rollback and backup/restore restore both install content and per-resource `pulith-state` facts.
//! - Activation replacement is explicit and platform-specific behavior is surfaced through typed errors.
//! - Windows file symlink privilege failures map to [`InstallError::WindowsFileSymlinkPrivilege`]
//!   instead of hidden fallback behavior.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pulith_archive::entry::ArchiveReport;
use pulith_fetch::{FetchReceipt, FetchSource};
use pulith_fs::{
    DEFAULT_COPY_ONLY_THRESHOLD_BYTES, FallBack, HardlinkOrCopyOptions, Workspace, atomic_symlink,
    copy_dir_all,
};
use pulith_resource::{Metadata, ResolvedResource};
use pulith_shim::TargetResolver;
use pulith_state::{
    ActivationRecord, ResourceLifecycle, ResourceRecord, ResourceRecordPatch,
    ResourceStateSnapshot, StateReady,
};
use pulith_store::{ExtractedArtifact, StoreKey, StoreProvenance, StoreReady, StoredArtifact};
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
    #[error("install root does not exist for upgrade: {0}")]
    MissingInstallForUpgrade(PathBuf),
    #[error("no rollback snapshot is available")]
    /// Rollback was requested for a flow that did not capture a previous-install snapshot.
    RollbackUnavailable,
    #[error("shim command must not be empty")]
    EmptyShimCommand,
    #[error("shim target `{0}` is not resolvable")]
    UnresolvedShimTarget(String),
    #[error("install root does not exist for backup: {0}")]
    /// Backup can only snapshot an existing install root.
    MissingInstallForBackup(PathBuf),
    #[error(
        "activation of file target requires symlink privilege or developer mode on Windows: {installed_path} -> {target}"
    )]
    WindowsFileSymlinkPrivilege {
        installed_path: PathBuf,
        target: PathBuf,
    },
    #[error("copy-based activation only supports files: {installed_path} -> {target}")]
    CopyActivationRequiresFile {
        installed_path: PathBuf,
        target: PathBuf,
    },
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

    /// Captures a per-resource backup receipt containing install content and matching state facts.
    ///
    /// The state payload is limited to the target resource record and activation history.
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

    /// Restores install content and captured per-resource state from a prior [`BackupReceipt`].
    ///
    /// This restore scope is limited to the install root in the receipt plus persisted facts for the
    /// same resource.
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

    fn resolve(&self, install_root: PathBuf) -> Result<PathBuf> {
        resolve_shim_target(install_root, &self.command)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct CopyFileActivator;

#[derive(Debug, Clone)]
pub struct ShimCopyActivator {
    command: ShimCommand,
}

impl ShimCopyActivator {
    pub fn new(command: ShimCommand) -> Self {
        Self { command }
    }

    fn resolve(&self, install_root: PathBuf) -> Result<PathBuf> {
        resolve_shim_target(install_root, &self.command)
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
        let artifact = store.import_artifact_with_provenance(
            key,
            &receipt.destination,
            Some(store_provenance_from_fetch_receipt(receipt)),
        )?;
        Self::from_stored_artifact(artifact)
    }

    pub fn from_archive_extraction(root: PathBuf, report: ArchiveReport) -> Self {
        Self::ExtractedTree {
            root,
            report: Some(report),
        }
    }

    pub fn store_archive_extraction(
        store: &StoreReady,
        key: &StoreKey,
        root: impl AsRef<Path>,
        report: &ArchiveReport,
    ) -> Result<Self> {
        let extracted = store.register_extract_dir_with_provenance(
            key,
            root,
            Some(store_provenance_from_archive_report(report)),
        )?;
        Ok(Self::ExtractedArtifact(extracted))
    }

    pub fn store_fetched_archive_extraction(
        store: &StoreReady,
        key: &StoreKey,
        receipt: &FetchReceipt,
        root: impl AsRef<Path>,
        report: &ArchiveReport,
    ) -> Result<Self> {
        let extracted = store.register_extract_dir_with_provenance(
            key,
            root,
            Some(store_provenance_from_fetched_archive_extraction(
                receipt, report,
            )),
        )?;
        Ok(Self::ExtractedArtifact(extracted))
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

#[derive(Debug)]
struct RollbackState {
    _temp_dir: tempfile::TempDir,
    backup_path: PathBuf,
    previous_state: ResourceStateSnapshot,
}

#[derive(Debug)]
pub struct Activated {
    pub install_root: PathBuf,
    pub activation: ActivationReceipt,
    pub replaced_previous: bool,
    rollback: Option<RollbackState>,
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

        let lifecycle = lifecycle_for_post_commit(&self.spec, rollback.as_ref());
        self.ready.state().upsert_resolved_resource(
            &self.spec.resource,
            ResourceRecordPatch::install_path(Some(install_root.clone()))
                .with_artifact_key(self.spec.input.store_key().cloned())
                .with_lifecycle(lifecycle)
                .with_metadata(self.spec.metadata.clone()),
        )?;

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
            if self.spec.mode == InstallMode::Upgrade {
                return Err(InstallError::MissingInstallForUpgrade(
                    self.spec.install_root.clone(),
                ));
            }
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

        let previous_state = self
            .ready
            .state()
            .capture_resource_state(&self.spec.resource.spec().id)?;

        Ok(Some(RollbackState {
            _temp_dir: temp_dir,
            backup_path,
            previous_state,
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

        self.ready.state().upsert_resolved_resource(
            &self.spec.resource,
            ResourceRecordPatch::install_path(Some(self.state.install_root.clone()))
                .with_artifact_key(self.spec.input.store_key().cloned())
                .with_lifecycle(ResourceLifecycle::Active)
                .with_metadata(self.spec.metadata.clone()),
        )?;

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
                rollback: self.state.rollback,
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
        restore_previous_state(self.ready.state(), &rollback)?;

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
    pub fn rollback(self) -> Result<RollbackReceipt> {
        let rollback = self
            .state
            .rollback
            .ok_or(InstallError::RollbackUnavailable)?;

        if self.state.install_root.exists() {
            remove_existing_target(&self.state.install_root)?;
        }
        restore_backup(&rollback.backup_path, &self.state.install_root)?;

        if rollback
            .previous_state
            .activations
            .iter()
            .all(|record| record.target != self.state.activation.target)
            && self.state.activation.target.exists()
        {
            remove_existing_target(&self.state.activation.target)?;
        }

        restore_previous_state(self.ready.state(), &rollback)?;

        Ok(RollbackReceipt {
            resource: self.spec.resource.spec().id.clone(),
            restored_path: self.state.install_root,
        })
    }

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

impl Activator for CopyFileActivator {
    fn activate(&self, request: &ActivationRequest) -> Result<ActivationReceipt> {
        copy_activation_target(&request.installed_path, &request.target)
    }
}

impl Activator for ShimLinkActivator {
    fn activate(&self, request: &ActivationRequest) -> Result<ActivationReceipt> {
        let resolved = self.resolve(request.installed_path.clone())?;
        link_activation_target(&resolved, &request.target)
    }
}

impl Activator for ShimCopyActivator {
    fn activate(&self, request: &ActivationRequest) -> Result<ActivationReceipt> {
        let resolved = self.resolve(request.installed_path.clone())?;
        copy_activation_target(&resolved, &request.target)
    }
}

fn remove_existing_target(path: &Path) -> Result<()> {
    if std::fs::read_link(path).is_ok() {
        match std::fs::remove_file(path) {
            Ok(()) => return Ok(()),
            Err(file_error) => match std::fs::remove_dir(path) {
                Ok(()) => return Ok(()),
                Err(_) => return Err(file_error.into()),
            },
        }
    }

    prepare_path_for_removal(path)?;

    let metadata = std::fs::symlink_metadata(path)?;
    if metadata.file_type().is_dir() {
        std::fs::remove_dir_all(path)?;
    } else {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

fn link_activation_target(installed_path: &Path, target: &Path) -> Result<ActivationReceipt> {
    prepare_activation_target(target)?;
    create_activation_link(installed_path, target)?;
    Ok(activation_receipt(installed_path, target))
}

fn copy_activation_target(installed_path: &Path, target: &Path) -> Result<ActivationReceipt> {
    if installed_path.is_dir() {
        return Err(InstallError::CopyActivationRequiresFile {
            installed_path: installed_path.to_path_buf(),
            target: target.to_path_buf(),
        });
    }

    prepare_activation_target(target)?;
    std::fs::copy(installed_path, target)?;
    Ok(activation_receipt(installed_path, target))
}

fn create_activation_link(installed_path: &Path, target: &Path) -> Result<()> {
    atomic_symlink(installed_path, target)
        .map_err(|error| map_activation_link_error(installed_path, target, error))
}

fn restore_backup(backup_path: &Path, install_root: &Path) -> Result<()> {
    ensure_parent_dir(install_root)?;
    std::fs::rename(backup_path, install_root)?;
    Ok(())
}

fn map_activation_link_error(
    installed_path: &Path,
    target: &Path,
    error: pulith_fs::Error,
) -> InstallError {
    #[cfg(windows)]
    {
        if !installed_path.is_dir()
            && matches!(
                &error,
                pulith_fs::Error::Write { source, .. }
                    if source.kind() == std::io::ErrorKind::PermissionDenied
            )
        {
            return InstallError::WindowsFileSymlinkPrivilege {
                installed_path: installed_path.to_path_buf(),
                target: target.to_path_buf(),
            };
        }
    }

    InstallError::Fs(error)
}

fn prepare_path_for_removal(path: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        clear_readonly_recursive(path)?;
    }

    Ok(())
}

#[cfg(windows)]
#[allow(clippy::permissions_set_readonly_false)]
fn clear_readonly_recursive(path: &Path) -> Result<()> {
    let metadata = std::fs::symlink_metadata(path)?;
    let file_type = metadata.file_type();

    if file_type.is_symlink() {
        return Ok(());
    }

    if metadata.permissions().readonly() {
        let mut permissions = metadata.permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(path, permissions)?;
    }

    if file_type.is_dir() {
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            clear_readonly_recursive(&entry.path())?;
        }
    }

    Ok(())
}

fn file_name_from_path(path: &Path) -> Option<String> {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
}

fn resolve_shim_target(install_root: PathBuf, command: &ShimCommand) -> Result<PathBuf> {
    InstalledShimResolver::new(install_root, vec![command.clone()])
        .resolve(&command.command)
        .ok_or_else(|| InstallError::UnresolvedShimTarget(command.command.clone()))
}

fn activation_receipt(installed_path: &Path, target: &Path) -> ActivationReceipt {
    ActivationReceipt {
        target: target.to_path_buf(),
        installed_path: installed_path.to_path_buf(),
    }
}

fn prepare_activation_target(target: &Path) -> Result<()> {
    if target.exists() {
        remove_existing_target(target)?;
    }
    ensure_parent_dir(target)
}

fn ensure_parent_dir(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    Ok(())
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

fn store_provenance_from_fetch_receipt(receipt: &FetchReceipt) -> StoreProvenance {
    let origin = match &receipt.source {
        FetchSource::Url(url) => Some(url.clone()),
        FetchSource::LocalPath(path) => Some(path.to_string_lossy().into_owned()),
    };

    let mut metadata = Metadata::new();
    if let Some(sha256_hex) = &receipt.sha256_hex {
        insert_metadata(&mut metadata, "fetch.sha256", sha256_hex.clone());
    }

    StoreProvenance { origin, metadata }
}

fn store_provenance_from_archive_report(report: &ArchiveReport) -> StoreProvenance {
    let mut metadata = Metadata::new();
    insert_archive_metadata(&mut metadata, report);

    StoreProvenance {
        origin: None,
        metadata,
    }
}

fn store_provenance_from_fetched_archive_extraction(
    receipt: &FetchReceipt,
    report: &ArchiveReport,
) -> StoreProvenance {
    let mut provenance = store_provenance_from_fetch_receipt(receipt);
    insert_archive_metadata(&mut provenance.metadata, report);
    provenance
}

fn insert_archive_metadata(metadata: &mut Metadata, report: &ArchiveReport) {
    insert_metadata(metadata, "archive.format", format!("{:?}", report.format));
    insert_metadata(
        metadata,
        "archive.entry_count",
        report.entry_count.to_string(),
    );
    insert_metadata(
        metadata,
        "archive.total_bytes",
        report.total_bytes.to_string(),
    );
}

fn insert_metadata(metadata: &mut Metadata, key: &str, value: String) {
    metadata.insert(key.to_string(), value);
}

fn lifecycle_for_post_commit(
    spec: &InstallSpec,
    rollback: Option<&RollbackState>,
) -> ResourceLifecycle {
    if spec.mode == InstallMode::Upgrade
        && rollback
            .and_then(|rollback| rollback.previous_state.record.as_ref())
            .is_some_and(|record| record.lifecycle == ResourceLifecycle::Active)
    {
        ResourceLifecycle::Active
    } else {
        ResourceLifecycle::Installed
    }
}

fn restore_previous_state(state: &StateReady, rollback: &RollbackState) -> Result<()> {
    state.restore_resource_state(&rollback.previous_state)?;
    Ok(())
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

    #[derive(Debug, Default)]
    struct FileActivator;

    impl Activator for FileActivator {
        fn activate(&self, request: &ActivationRequest) -> Result<ActivationReceipt> {
            if let Some(parent) = request.target.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(
                &request.target,
                request.installed_path.to_string_lossy().as_bytes(),
            )?;
            Ok(ActivationReceipt {
                target: request.target.clone(),
                installed_path: request.installed_path.clone(),
            })
        }
    }

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
                sha256_hex: Some("abc123".to_string()),
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
                let provenance = artifact.provenance.unwrap();
                assert_eq!(
                    provenance.origin.as_deref(),
                    Some("https://example.com/runtime.bin")
                );
                assert_eq!(
                    provenance.metadata.get("fetch.sha256").map(String::as_str),
                    Some("abc123")
                );

                let looked_up = store.get_artifact(&key).unwrap();
                let looked_up_provenance = looked_up.provenance.unwrap();
                assert_eq!(
                    looked_up_provenance.origin.as_deref(),
                    Some("https://example.com/runtime.bin")
                );
                assert_eq!(
                    looked_up_provenance
                        .metadata
                        .get("fetch.sha256")
                        .map(String::as_str),
                    Some("abc123")
                );
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
    fn install_input_store_archive_extraction_persists_archive_provenance() {
        let temp = tempfile::tempdir().unwrap();
        let extract_root = temp.path().join("extract-tree");
        std::fs::create_dir_all(extract_root.join("bin")).unwrap();
        std::fs::write(extract_root.join("bin/tool.exe"), b"payload").unwrap();

        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();
        let key = StoreKey::logical("runtime-extract").unwrap();
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 1,
            total_bytes: 7,
            entries: vec![],
        };

        let input =
            InstallInput::store_archive_extraction(&store, &key, &extract_root, &report).unwrap();

        match input {
            InstallInput::ExtractedArtifact(artifact) => {
                assert!(artifact.path.join("bin/tool.exe").exists());
                let provenance = artifact.provenance.unwrap();
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
                assert_eq!(
                    provenance
                        .metadata
                        .get("archive.total_bytes")
                        .map(String::as_str),
                    Some("7")
                );

                let looked_up = store.get_extract(&key).unwrap();
                let looked_up_provenance = looked_up.provenance.unwrap();
                assert_eq!(
                    looked_up_provenance
                        .metadata
                        .get("archive.format")
                        .map(String::as_str),
                    Some("Zip")
                );
            }
            _ => panic!("expected extracted artifact install input"),
        }
    }

    #[test]
    fn install_input_store_fetched_archive_extraction_merges_fetch_and_archive_provenance() {
        let temp = tempfile::tempdir().unwrap();
        let extract_root = temp.path().join("extract-tree");
        std::fs::create_dir_all(extract_root.join("bin")).unwrap();
        std::fs::write(extract_root.join("bin/tool.exe"), b"payload").unwrap();

        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();
        let key = StoreKey::logical("runtime-fetch-extract").unwrap();
        let receipt = FetchReceipt {
            source: FetchSource::Url("https://example.com/runtime.zip".to_string()),
            destination: temp.path().join("downloads/runtime.zip"),
            bytes_downloaded: 7,
            total_bytes: Some(7),
            sha256_hex: Some("abc123".to_string()),
        };
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 1,
            total_bytes: 7,
            entries: vec![],
        };

        let input = InstallInput::store_fetched_archive_extraction(
            &store,
            &key,
            &receipt,
            &extract_root,
            &report,
        )
        .unwrap();

        match input {
            InstallInput::ExtractedArtifact(artifact) => {
                let provenance = artifact.provenance.unwrap();
                assert_eq!(
                    provenance.origin.as_deref(),
                    Some("https://example.com/runtime.zip")
                );
                assert_eq!(
                    provenance.metadata.get("fetch.sha256").map(String::as_str),
                    Some("abc123")
                );
                assert_eq!(
                    provenance
                        .metadata
                        .get("archive.format")
                        .map(String::as_str),
                    Some("Zip")
                );
            }
            _ => panic!("expected extracted artifact install input"),
        }
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
    fn upgrade_existing_requires_previous_install() {
        let temp = tempfile::tempdir().unwrap();

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
            temp.path().join("install/runtime"),
        )
        .upgrade_existing();

        assert!(matches!(
            PlannedInstall::new(ready, spec).stage().unwrap().commit(),
            Err(InstallError::MissingInstallForUpgrade(_))
        ));
    }

    #[test]
    fn upgrade_existing_preserves_active_lifecycle() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::write(install_root.join("tool.exe"), b"old").unwrap();

        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let active_target = temp.path().join("active/runtime");
        state
            .upsert_resource_record(ResourceRecord {
                id: ResourceId::parse("example/runtime").unwrap(),
                selector: resolved_resource().spec().version.clone(),
                resolved_version: Some(resolved_resource().version().clone()),
                locator: Some(resolved_resource().locator().clone()),
                artifact_key: None,
                install_path: Some(install_root.clone()),
                lifecycle: ResourceLifecycle::Active,
                metadata: Metadata::new(),
            })
            .unwrap();
        state
            .record_activation(
                &ResourceId::parse("example/runtime").unwrap(),
                active_target,
            )
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
        let receipt = PlannedInstall::new(
            ready,
            InstallSpec::new(
                resolved_resource(),
                InstallInput::ExtractedArtifact(extracted),
                install_root.clone(),
            )
            .upgrade_existing(),
        )
        .stage()
        .unwrap()
        .commit()
        .unwrap()
        .finish();

        assert!(receipt.replaced_previous);
        assert_eq!(
            std::fs::read(install_root.join("tool.exe")).unwrap(),
            b"new"
        );
        assert_eq!(
            state
                .get_resource_record(&ResourceId::parse("example/runtime").unwrap())
                .unwrap()
                .unwrap()
                .lifecycle,
            ResourceLifecycle::Active
        );
        assert_eq!(
            state
                .list_activation_records(&ResourceId::parse("example/runtime").unwrap())
                .unwrap()
                .len(),
            1
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
    fn rollback_after_activation_restores_previous_activation_state() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::write(install_root.join("tool.exe"), b"old").unwrap();

        let old_target = temp.path().join("active/runtime-old");
        std::fs::create_dir_all(old_target.parent().unwrap()).unwrap();
        std::fs::write(&old_target, install_root.to_string_lossy().as_bytes()).unwrap();

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
                lifecycle: ResourceLifecycle::Active,
                metadata: Metadata::new(),
            })
            .unwrap();
        state
            .record_activation(&resource_id, old_target.clone())
            .unwrap();

        let source_dir = temp.path().join("extract-new");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("tool.exe"), b"new").unwrap();

        let new_target = temp.path().join("active/runtime-new");
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
        .replace_existing()
        .activation(ActivationTarget {
            path: new_target.clone(),
        });

        let rollback = PlannedInstall::new(ready, spec)
            .stage()
            .unwrap()
            .commit()
            .unwrap()
            .activate(&FileActivator)
            .unwrap()
            .rollback()
            .unwrap();

        assert_eq!(rollback.restored_path, install_root);
        assert_eq!(
            std::fs::read(rollback.restored_path.join("tool.exe")).unwrap(),
            b"old"
        );
        assert!(old_target.exists());
        assert!(!new_target.exists());

        let snapshot: StateSnapshot = state.load().unwrap();
        assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Active);
        assert_eq!(snapshot.activations.len(), 1);
        assert_eq!(snapshot.activations[0].target, old_target);
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
    fn symlink_activator_replaces_existing_file_target() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        let installed_file = install_root.join("tool.exe");
        std::fs::write(&installed_file, b"payload").unwrap();

        let existing_target = temp.path().join("active/tool");
        std::fs::create_dir_all(existing_target.parent().unwrap()).unwrap();
        std::fs::write(&existing_target, b"old-target").unwrap();

        let receipt = SymlinkActivator
            .activate(&ActivationRequest {
                resource: ResourceId::parse("example/runtime").unwrap(),
                installed_path: installed_file.clone(),
                target: existing_target.clone(),
            })
            .unwrap();

        assert_eq!(receipt.installed_path, installed_file);
        assert_eq!(receipt.target, existing_target);
        assert_eq!(std::fs::read(&existing_target).unwrap(), b"payload");
    }

    #[test]
    fn copy_file_activator_copies_payload_to_target() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        let installed_file = install_root.join("tool.exe");
        std::fs::write(&installed_file, b"payload").unwrap();

        let target = temp.path().join("active/tool.exe");
        let receipt = CopyFileActivator
            .activate(&ActivationRequest {
                resource: ResourceId::parse("example/runtime").unwrap(),
                installed_path: installed_file.clone(),
                target: target.clone(),
            })
            .unwrap();

        assert_eq!(receipt.installed_path, installed_file);
        assert_eq!(receipt.target, target.clone());
        assert_eq!(std::fs::read(target).unwrap(), b"payload");
    }

    #[test]
    fn copy_file_activator_rejects_directory_targets() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();

        let target = temp.path().join("active/runtime");
        assert!(matches!(
            CopyFileActivator.activate(&ActivationRequest {
                resource: ResourceId::parse("example/runtime").unwrap(),
                installed_path: install_root.clone(),
                target: target.clone(),
            }),
            Err(InstallError::CopyActivationRequiresFile {
                installed_path,
                target: activation_target,
            }) if installed_path == install_root && activation_target == target
        ));
    }

    #[test]
    fn symlink_activator_replaces_existing_directory_target() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::write(install_root.join("tool.exe"), b"payload").unwrap();

        let existing_target = temp.path().join("active/runtime");
        std::fs::create_dir_all(&existing_target).unwrap();
        std::fs::write(existing_target.join("stale.txt"), b"stale").unwrap();

        let receipt = SymlinkActivator
            .activate(&ActivationRequest {
                resource: ResourceId::parse("example/runtime").unwrap(),
                installed_path: install_root.clone(),
                target: existing_target.clone(),
            })
            .unwrap();

        assert_eq!(receipt.installed_path, install_root);
        assert_eq!(receipt.target, existing_target);
        assert!(existing_target.join("tool.exe").exists());

        #[cfg(unix)]
        assert!(
            std::fs::symlink_metadata(&existing_target)
                .unwrap()
                .file_type()
                .is_symlink()
        );

        #[cfg(windows)]
        assert!(existing_target.is_dir());
    }

    #[cfg(windows)]
    #[test]
    fn symlink_activator_replaces_readonly_file_target() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(&install_root).unwrap();
        let installed_file = install_root.join("tool.exe");
        std::fs::write(&installed_file, b"payload").unwrap();

        let existing_target = temp.path().join("active/tool.exe");
        std::fs::create_dir_all(existing_target.parent().unwrap()).unwrap();
        std::fs::write(&existing_target, b"old-target").unwrap();
        let mut permissions = std::fs::metadata(&existing_target).unwrap().permissions();
        permissions.set_readonly(true);
        std::fs::set_permissions(&existing_target, permissions).unwrap();

        let receipt = SymlinkActivator
            .activate(&ActivationRequest {
                resource: ResourceId::parse("example/runtime").unwrap(),
                installed_path: installed_file.clone(),
                target: existing_target.clone(),
            })
            .unwrap();

        assert_eq!(receipt.installed_path, installed_file);
        assert_eq!(receipt.target, existing_target);
        assert_eq!(std::fs::read(&existing_target).unwrap(), b"payload");
    }

    #[cfg(windows)]
    #[test]
    fn file_activation_permission_denied_maps_to_symlink_privilege_error() {
        let installed_path = PathBuf::from(r"C:\runtime\tool.exe");
        let target = PathBuf::from(r"C:\bin\tool.exe");
        let error = pulith_fs::Error::Write {
            path: target.clone(),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "access denied"),
        };

        assert!(matches!(
            map_activation_link_error(&installed_path, &target, error),
            InstallError::WindowsFileSymlinkPrivilege {
                installed_path: mapped_installed_path,
                target: mapped_target,
            } if mapped_installed_path == installed_path && mapped_target == target
        ));
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
    fn shim_copy_activator_copies_relative_executable() {
        let temp = tempfile::tempdir().unwrap();
        let install_root = temp.path().join("install/runtime");
        std::fs::create_dir_all(install_root.join("bin")).unwrap();
        std::fs::write(install_root.join("bin/tool.exe"), b"payload").unwrap();

        let request = ActivationRequest {
            resource: ResourceId::parse("example/runtime").unwrap(),
            installed_path: install_root.clone(),
            target: temp.path().join("active/tool.exe"),
        };

        let activator = ShimCopyActivator::new(ShimCommand::new("tool", "bin/tool.exe").unwrap());
        let receipt = activator.activate(&request).unwrap();

        assert_eq!(receipt.installed_path, install_root.join("bin/tool.exe"));
        assert_eq!(std::fs::read(receipt.target).unwrap(), b"payload");
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
