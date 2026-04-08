//! Transaction-backed persistent state for Pulith resources.

use std::path::{Path, PathBuf};

use pulith_fs::{Transaction, atomic_write};
use pulith_resource::{Metadata, ResolvedLocator, ResolvedVersion, ResourceId, VersionSelector};
use pulith_store::StoreKey;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StateError>;

#[derive(Debug, Error)]
pub enum StateError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fs(#[from] pulith_fs::Error),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone)]
pub struct StateReady {
    path: PathBuf,
}

impl StateReady {
    pub fn initialize(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        if !path.exists() {
            let initial = serde_json::to_vec_pretty(&StateSnapshot::default())?;
            atomic_write(&path, &initial, Default::default())?;
        }
        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn load(&self) -> Result<StateSnapshot> {
        let tx = Transaction::open(&self.path)?;
        let snapshot = load_from_transaction(&tx)?;
        Ok(snapshot)
    }

    pub fn save(&self, snapshot: &StateSnapshot) -> Result<()> {
        let tx = Transaction::open(&self.path)?;
        save_to_transaction(&tx, snapshot)
    }

    pub fn update<F>(&self, update: F) -> Result<StateSnapshot>
    where
        F: FnOnce(StateSnapshot) -> Result<StateSnapshot>,
    {
        let tx = Transaction::open(&self.path)?;
        let current = load_from_transaction(&tx)?;
        let next = update(current)?;
        save_to_transaction(&tx, &next)?;
        Ok(next)
    }

    pub fn upsert_resource_record(&self, record: ResourceRecord) -> Result<StateSnapshot> {
        self.update(|mut snapshot| {
            if let Some(existing) = snapshot
                .resources
                .iter_mut()
                .find(|item| item.id == record.id)
            {
                *existing = record;
            } else {
                snapshot.resources.push(record);
            }
            Ok(snapshot)
        })
    }

    pub fn append_activation(&self, activation: ActivationRecord) -> Result<StateSnapshot> {
        self.update(|mut snapshot| {
            snapshot.activations.push(activation);
            Ok(snapshot)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct StateSnapshot {
    pub resources: Vec<ResourceRecord>,
    pub activations: Vec<ActivationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRecord {
    pub id: ResourceId,
    pub selector: VersionSelector,
    pub resolved_version: Option<ResolvedVersion>,
    pub locator: Option<ResolvedLocator>,
    pub artifact_key: Option<StoreKey>,
    pub install_path: Option<PathBuf>,
    pub lifecycle: ResourceLifecycle,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceLifecycle {
    Declared,
    Resolved,
    Fetched,
    Materialized,
    Installed,
    Registered,
    Active,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationRecord {
    pub id: ResourceId,
    pub target: PathBuf,
    pub activated_at_unix: u64,
}

fn load_from_transaction(tx: &Transaction) -> Result<StateSnapshot> {
    let bytes = tx.read()?;
    if bytes.is_empty() {
        return Ok(StateSnapshot::default());
    }
    Ok(serde_json::from_slice(&bytes)?)
}

fn save_to_transaction(tx: &Transaction, snapshot: &StateSnapshot) -> Result<()> {
    let encoded = serde_json::to_vec_pretty(snapshot)?;
    tx.write(&encoded)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulith_resource::{RequestedResource, ResourceLocator, ResourceSpec, ValidUrl};

    #[test]
    fn state_initializes_and_loads_default_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let snapshot = state.load().unwrap();
        assert!(snapshot.resources.is_empty());
    }

    #[test]
    fn state_updates_records_transactionally() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();

        let id = ResourceId::parse("nodejs.org/node").unwrap();
        let updated = state
            .update(|mut snapshot| {
                snapshot.resources.push(ResourceRecord {
                    id: id.clone(),
                    selector: VersionSelector::alias("lts").unwrap(),
                    resolved_version: None,
                    locator: None,
                    artifact_key: None,
                    install_path: None,
                    lifecycle: ResourceLifecycle::Declared,
                    metadata: Metadata::new(),
                });
                Ok(snapshot)
            })
            .unwrap();

        assert_eq!(updated.resources.len(), 1);
        assert_eq!(state.load().unwrap().resources.len(), 1);
    }

    #[test]
    fn state_can_store_resolved_resource_facts() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();

        let requested = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
        ));
        let resolved = requested.resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(
                ValidUrl::parse("https://mirror.example.com/runtime.zip").unwrap(),
            ),
            None,
        );

        state
            .save(&StateSnapshot {
                resources: vec![ResourceRecord {
                    id: resolved.spec().id.clone(),
                    selector: resolved.spec().version.clone(),
                    resolved_version: Some(resolved.version().clone()),
                    locator: Some(resolved.locator().clone()),
                    artifact_key: None,
                    install_path: None,
                    lifecycle: ResourceLifecycle::Resolved,
                    metadata: Metadata::new(),
                }],
                activations: vec![],
            })
            .unwrap();

        let snapshot = state.load().unwrap();
        assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Resolved);
    }
}
