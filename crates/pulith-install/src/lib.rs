//! Composable installation workflow primitives for Pulith.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pulith_fs::{Workspace, atomic_symlink};
use pulith_resource::{Metadata, ResolvedResource};
use pulith_state::{ActivationRecord, ResourceLifecycle, ResourceRecord, StateReady};
use pulith_store::{ExtractedArtifact, StoreKey, StoredArtifact};
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
    #[error("artifact file name must not be empty")]
    EmptyFileName,
    #[error("extracted artifact path does not exist: {0}")]
    MissingExtractedArtifact(PathBuf),
    #[error("stored artifact path does not exist: {0}")]
    MissingStoredArtifact(PathBuf),
    #[error("activation target was not configured")]
    MissingActivationTarget,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstallInput {
    StoredArtifact {
        artifact: StoredArtifact,
        file_name: String,
    },
    ExtractedArtifact(ExtractedArtifact),
}

impl InstallInput {
    fn store_key(&self) -> &StoreKey {
        match self {
            Self::StoredArtifact { artifact, .. } => &artifact.key,
            Self::ExtractedArtifact(artifact) => &artifact.key,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallSpec {
    pub resource: ResolvedResource,
    pub input: InstallInput,
    pub install_root: PathBuf,
    pub activation: Option<ActivationTarget>,
    pub metadata: Metadata,
}

impl InstallSpec {
    pub fn new(resource: ResolvedResource, input: InstallInput, install_root: PathBuf) -> Self {
        Self {
            resource,
            input,
            install_root,
            activation: None,
            metadata: Metadata::new(),
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    pub install_root: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Activated {
    pub install_root: PathBuf,
    pub activation: ActivationReceipt,
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
        let temp = tempfile::tempdir()?;
        let workspace =
            Workspace::new(temp.path().join("staging"), self.spec.install_root.clone())?;

        match &self.spec.input {
            InstallInput::ExtractedArtifact(artifact) => {
                if !artifact.path.exists() {
                    return Err(InstallError::MissingExtractedArtifact(
                        artifact.path.clone(),
                    ));
                }
                copy_directory_into_workspace(&workspace, &artifact.path, Path::new(""))?;
            }
            InstallInput::StoredArtifact {
                artifact,
                file_name,
            } => {
                if file_name.is_empty() {
                    return Err(InstallError::EmptyFileName);
                }
                if !artifact.path.exists() {
                    return Err(InstallError::MissingStoredArtifact(artifact.path.clone()));
                }
                workspace.copy_file(&artifact.path, file_name)?;
            }
        }

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
        self.state.workspace.commit()?;

        let record = ResourceRecord {
            id: self.spec.resource.spec().id.clone(),
            selector: self.spec.resource.spec().version.clone(),
            resolved_version: Some(self.spec.resource.version().clone()),
            locator: Some(self.spec.resource.locator().clone()),
            artifact_key: Some(self.spec.input.store_key().clone()),
            install_path: Some(install_root.clone()),
            lifecycle: ResourceLifecycle::Installed,
            metadata: self.spec.metadata.clone(),
        };
        self.ready.state().upsert_resource_record(record)?;

        Ok(InstallFlow {
            ready: self.ready,
            spec: self.spec,
            state: Installed { install_root },
        })
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

        self.ready.state().upsert_resource_record(ResourceRecord {
            id: self.spec.resource.spec().id.clone(),
            selector: self.spec.resource.spec().version.clone(),
            resolved_version: Some(self.spec.resource.version().clone()),
            locator: Some(self.spec.resource.locator().clone()),
            artifact_key: Some(self.spec.input.store_key().clone()),
            install_path: Some(self.state.install_root.clone()),
            lifecycle: ResourceLifecycle::Active,
            metadata: self.spec.metadata.clone(),
        })?;

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
            },
        })
    }

    pub fn finish(self) -> InstallReceipt {
        InstallReceipt {
            resource: self.spec.resource.spec().id.clone(),
            install_root: self.state.install_root,
            activation: None,
        }
    }
}

impl ActivatedInstall {
    pub fn finish(self) -> InstallReceipt {
        InstallReceipt {
            resource: self.spec.resource.spec().id.clone(),
            install_root: self.state.install_root,
            activation: Some(self.state.activation),
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
        if request.target.exists() {
            remove_existing_target(&request.target)?;
        }
        if let Some(parent) = request.target.parent() {
            std::fs::create_dir_all(parent)?;
        }
        atomic_symlink(&request.installed_path, &request.target)?;
        Ok(ActivationReceipt {
            target: request.target.clone(),
            installed_path: request.installed_path.clone(),
        })
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
            workspace.copy_file(&path, &relative_path)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use pulith_resource::{
        RequestedResource, ResolvedLocator, ResolvedVersion, ResourceId, ResourceLocator,
        ResourceSpec, ValidUrl,
    };
    use pulith_state::StateSnapshot;

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
    fn activation_records_state() {
        let temp = tempfile::tempdir().unwrap();
        let source_dir = temp.path().join("extract");
        std::fs::create_dir_all(&source_dir).unwrap();
        std::fs::write(source_dir.join("bin"), b"payload").unwrap();

        let extracted = ExtractedArtifact {
            key: StoreKey::logical("runtime").unwrap(),
            path: source_dir,
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
}
