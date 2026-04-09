//! Transaction-backed persistent state for Pulith resources.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pulith_fs::{Transaction, atomic_write};
use pulith_resource::{Metadata, ResolvedLocator, ResolvedVersion, ResourceId, VersionSelector};
use pulith_store::{StoreKey, StoreMetadataRecord, StoreReady};
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
    Store(#[from] pulith_store::StoreError),
    #[error(transparent)]
    Serde(#[from] serde_json::Error),
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRecordPatch {
    pub selector: Option<VersionSelector>,
    pub resolved_version: Option<Option<ResolvedVersion>>,
    pub locator: Option<Option<ResolvedLocator>>,
    pub artifact_key: Option<Option<StoreKey>>,
    pub install_path: Option<Option<PathBuf>>,
    pub lifecycle: Option<ResourceLifecycle>,
    pub metadata: Option<Metadata>,
}

impl ResourceRecordPatch {
    pub fn lifecycle(lifecycle: ResourceLifecycle) -> Self {
        Self {
            lifecycle: Some(lifecycle),
            ..Self::default()
        }
    }

    pub fn artifact_key(artifact_key: Option<StoreKey>) -> Self {
        Self {
            artifact_key: Some(artifact_key),
            ..Self::default()
        }
    }

    pub fn install_path(install_path: Option<PathBuf>) -> Self {
        Self {
            install_path: Some(install_path),
            ..Self::default()
        }
    }

    pub fn metadata(metadata: Metadata) -> Self {
        Self {
            metadata: Some(metadata),
            ..Self::default()
        }
    }

    pub fn with_lifecycle(mut self, lifecycle: ResourceLifecycle) -> Self {
        self.lifecycle = Some(lifecycle);
        self
    }

    pub fn with_artifact_key(mut self, artifact_key: Option<StoreKey>) -> Self {
        self.artifact_key = Some(artifact_key);
        self
    }

    pub fn with_install_path(mut self, install_path: Option<PathBuf>) -> Self {
        self.install_path = Some(install_path);
        self
    }

    pub fn with_metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = Some(metadata);
        self
    }
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

    pub fn get_resource_record(&self, id: &ResourceId) -> Result<Option<ResourceRecord>> {
        Ok(self
            .load()?
            .resources
            .into_iter()
            .find(|record| &record.id == id))
    }

    pub fn list_activation_records(&self, id: &ResourceId) -> Result<Vec<ActivationRecord>> {
        Ok(self
            .load()?
            .activations
            .into_iter()
            .filter(|record| &record.id == id)
            .collect())
    }

    pub fn inspect_resource(
        &self,
        id: &ResourceId,
        store: Option<&StoreReady>,
    ) -> Result<ResourceInspection> {
        let full_snapshot = self.load()?;
        let snapshot = capture_resource_state_from_snapshot(&full_snapshot, id);
        Ok(ResourceInspection::from_snapshot(
            snapshot,
            &full_snapshot.activations,
            store,
        ))
    }

    pub fn list_activation_conflicts(&self) -> Result<Vec<ActivationOwnershipConflict>> {
        let snapshot = self.load()?;
        Ok(activation_conflicts(&snapshot.activations))
    }

    pub fn list_store_references(&self) -> Result<Vec<StoreKeyReference>> {
        let snapshot = self.load()?;
        Ok(store_key_references(&snapshot.resources))
    }

    pub fn protected_store_keys(&self, policy: StoreRetentionPolicy) -> Result<Vec<StoreKey>> {
        let snapshot = self.load()?;
        Ok(
            store_key_references_for_retention(&snapshot.resources, policy)
                .into_iter()
                .map(|reference| reference.key)
                .collect(),
        )
    }

    pub fn retained_store_references(
        &self,
        policy: StoreRetentionPolicy,
    ) -> Result<Vec<StoreKeyReference>> {
        let snapshot = self.load()?;
        Ok(store_key_references_for_retention(
            &snapshot.resources,
            policy,
        ))
    }

    pub fn plan_store_metadata_retention(
        &self,
        store: &StoreReady,
        policy: StoreRetentionPolicy,
    ) -> Result<StoreRetentionPlan> {
        let protected_keys = self.protected_store_keys(policy)?;
        let metadata_plan = store.plan_metadata_prune(&protected_keys)?;

        Ok(StoreRetentionPlan {
            policy,
            protected_keys,
            removable_metadata: metadata_plan.removable,
            protected_metadata: metadata_plan.protected,
        })
    }

    pub fn plan_resource_state_repair(
        &self,
        id: &ResourceId,
        store: Option<&StoreReady>,
    ) -> Result<ResourceRepairPlan> {
        let inspection = self.inspect_resource(id, store)?;
        Ok(ResourceRepairPlan::from_inspection(inspection))
    }

    pub fn apply_resource_state_repair(&self, plan: &ResourceRepairPlan) -> Result<StateSnapshot> {
        self.update(|mut snapshot| {
            for action in &plan.actions {
                match action {
                    ResourceRepairAction::ClearInstallPath { resource } => {
                        if let Some(record) = snapshot
                            .resources
                            .iter_mut()
                            .find(|record| &record.id == resource)
                        {
                            record.install_path = None;
                        }
                    }
                    ResourceRepairAction::ClearArtifactKey { resource } => {
                        if let Some(record) = snapshot
                            .resources
                            .iter_mut()
                            .find(|record| &record.id == resource)
                        {
                            record.artifact_key = None;
                        }
                    }
                    ResourceRepairAction::RemoveActivationRecord { resource, target } => {
                        snapshot
                            .activations
                            .retain(|record| &record.id != resource || &record.target != target);
                    }
                }
            }

            Ok(snapshot)
        })
    }

    pub fn capture_resource_state(&self, id: &ResourceId) -> Result<ResourceStateSnapshot> {
        let snapshot = self.load()?;
        Ok(capture_resource_state_from_snapshot(&snapshot, id))
    }

    pub fn restore_resource_state(
        &self,
        resource_state: &ResourceStateSnapshot,
    ) -> Result<StateSnapshot> {
        self.update(|mut snapshot| {
            snapshot
                .resources
                .retain(|record| record.id != resource_state.resource);
            if let Some(record) = &resource_state.record {
                snapshot.resources.push(record.clone());
            }

            snapshot
                .activations
                .retain(|record| record.id != resource_state.resource);
            snapshot
                .activations
                .extend(resource_state.activations.iter().cloned());

            Ok(snapshot)
        })
    }

    pub fn set_resource_lifecycle(
        &self,
        id: &ResourceId,
        lifecycle: ResourceLifecycle,
    ) -> Result<StateSnapshot> {
        self.patch_resource_record(id, ResourceRecordPatch::lifecycle(lifecycle))
    }

    pub fn patch_resource_record(
        &self,
        id: &ResourceId,
        patch: ResourceRecordPatch,
    ) -> Result<StateSnapshot> {
        self.update(|mut snapshot| {
            if let Some(record) = snapshot
                .resources
                .iter_mut()
                .find(|record| &record.id == id)
            {
                record.apply_patch(patch);
            }
            Ok(snapshot)
        })
    }

    pub fn ensure_resource_record(
        &self,
        id: ResourceId,
        selector: VersionSelector,
    ) -> Result<StateSnapshot> {
        self.update(|mut snapshot| {
            if !snapshot.resources.iter().any(|record| record.id == id) {
                snapshot.resources.push(ResourceRecord {
                    id,
                    selector,
                    resolved_version: None,
                    locator: None,
                    artifact_key: None,
                    install_path: None,
                    lifecycle: ResourceLifecycle::Declared,
                    metadata: Metadata::new(),
                });
            }
            Ok(snapshot)
        })
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

    pub fn upsert_resolved_resource(
        &self,
        resource: &pulith_resource::ResolvedResource,
        patch: ResourceRecordPatch,
    ) -> Result<StateSnapshot> {
        let resource_id = resource.spec().id.clone();
        let base_record = ResourceRecord::from_resolved_resource(resource);

        self.update(|mut snapshot| {
            if let Some(existing) = snapshot
                .resources
                .iter_mut()
                .find(|record| record.id == resource_id)
            {
                *existing = base_record.clone();
                existing.apply_patch(patch);
            } else {
                let mut record = base_record;
                record.apply_patch(patch);
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

    pub fn record_activation(&self, id: &ResourceId, target: PathBuf) -> Result<StateSnapshot> {
        self.append_activation(ActivationRecord {
            id: id.clone(),
            target,
            activated_at_unix: now_unix(),
        })
    }

    pub fn remove_resource_record(&self, id: &ResourceId) -> Result<StateSnapshot> {
        self.update(|mut snapshot| {
            snapshot.resources.retain(|record| &record.id != id);
            Ok(snapshot)
        })
    }

    pub fn remove_activation_records(&self, id: &ResourceId) -> Result<StateSnapshot> {
        self.update(|mut snapshot| {
            snapshot.activations.retain(|record| &record.id != id);
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
pub struct ResourceStateSnapshot {
    pub resource: ResourceId,
    pub record: Option<ResourceRecord>,
    pub activations: Vec<ActivationRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceInspection {
    pub snapshot: ResourceStateSnapshot,
    pub issues: Vec<ResourceInspectionIssue>,
}

impl ResourceInspection {
    pub fn from_snapshot(
        snapshot: ResourceStateSnapshot,
        all_activations: &[ActivationRecord],
        store: Option<&StoreReady>,
    ) -> Self {
        let mut issues = Vec::new();

        if snapshot.record.is_none() {
            issues.push(ResourceInspectionIssue::MissingResourceRecord {
                resource: snapshot.resource.clone(),
            });
        }

        if let Some(record) = &snapshot.record {
            if let Some(install_path) = &record.install_path {
                if !install_path.exists() {
                    issues.push(ResourceInspectionIssue::MissingInstallPath {
                        resource: snapshot.resource.clone(),
                        path: install_path.clone(),
                    });
                }
            }

            if let (Some(store), Some(key)) = (store, &record.artifact_key) {
                if !store.has_artifact(key) && !store.has_extract(key) {
                    issues.push(ResourceInspectionIssue::MissingStoreEntry {
                        resource: snapshot.resource.clone(),
                        key: key.clone(),
                    });
                }
                if !store.has_metadata(key) {
                    issues.push(ResourceInspectionIssue::MissingStoreMetadata {
                        resource: snapshot.resource.clone(),
                        key: key.clone(),
                    });
                }
            }
        }

        for activation in &snapshot.activations {
            if !activation.target.exists() {
                issues.push(ResourceInspectionIssue::MissingActivationTarget {
                    resource: snapshot.resource.clone(),
                    target: activation.target.clone(),
                });
            }

            let conflicting_owners = conflicting_activation_owners(
                &snapshot.resource,
                &activation.target,
                all_activations,
            );
            if !conflicting_owners.is_empty() {
                issues.push(ResourceInspectionIssue::ActivationTargetConflict {
                    resource: snapshot.resource.clone(),
                    target: activation.target.clone(),
                    conflicting_owners,
                });
            }
        }

        Self { snapshot, issues }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRepairPlan {
    pub inspection: ResourceInspection,
    pub actions: Vec<ResourceRepairAction>,
}

impl ResourceRepairPlan {
    pub fn from_inspection(inspection: ResourceInspection) -> Self {
        let mut actions = Vec::new();

        for issue in &inspection.issues {
            match issue {
                ResourceInspectionIssue::MissingInstallPath { resource, .. } => {
                    push_unique_action(
                        &mut actions,
                        ResourceRepairAction::ClearInstallPath {
                            resource: resource.clone(),
                        },
                    );
                }
                ResourceInspectionIssue::MissingActivationTarget { resource, target } => {
                    push_unique_action(
                        &mut actions,
                        ResourceRepairAction::RemoveActivationRecord {
                            resource: resource.clone(),
                            target: target.clone(),
                        },
                    );
                }
                ResourceInspectionIssue::MissingStoreEntry { resource, .. } => {
                    push_unique_action(
                        &mut actions,
                        ResourceRepairAction::ClearArtifactKey {
                            resource: resource.clone(),
                        },
                    );
                }
                ResourceInspectionIssue::MissingResourceRecord { .. }
                | ResourceInspectionIssue::MissingStoreMetadata { .. }
                | ResourceInspectionIssue::ActivationTargetConflict { .. } => {}
            }
        }

        Self {
            inspection,
            actions,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceRepairAction {
    ClearInstallPath {
        resource: ResourceId,
    },
    ClearArtifactKey {
        resource: ResourceId,
    },
    RemoveActivationRecord {
        resource: ResourceId,
        target: PathBuf,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceInspectionIssue {
    MissingResourceRecord {
        resource: ResourceId,
    },
    MissingInstallPath {
        resource: ResourceId,
        path: PathBuf,
    },
    MissingActivationTarget {
        resource: ResourceId,
        target: PathBuf,
    },
    ActivationTargetConflict {
        resource: ResourceId,
        target: PathBuf,
        conflicting_owners: Vec<ResourceId>,
    },
    MissingStoreEntry {
        resource: ResourceId,
        key: StoreKey,
    },
    MissingStoreMetadata {
        resource: ResourceId,
        key: StoreKey,
    },
}

fn push_unique_action(actions: &mut Vec<ResourceRepairAction>, action: ResourceRepairAction) {
    if !actions.contains(&action) {
        actions.push(action);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationOwnershipConflict {
    pub target: PathBuf,
    pub owners: Vec<ResourceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreKeyReference {
    pub key: StoreKey,
    pub owners: Vec<ResourceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreRetentionPolicy {
    AllReferenced,
    InstalledAndActive,
    ActiveOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreRetentionPlan {
    pub policy: StoreRetentionPolicy,
    pub protected_keys: Vec<StoreKey>,
    pub removable_metadata: Vec<StoreMetadataRecord>,
    pub protected_metadata: Vec<StoreMetadataRecord>,
}

fn capture_resource_state_from_snapshot(
    snapshot: &StateSnapshot,
    id: &ResourceId,
) -> ResourceStateSnapshot {
    ResourceStateSnapshot {
        resource: id.clone(),
        record: snapshot
            .resources
            .iter()
            .find(|record| &record.id == id)
            .cloned(),
        activations: snapshot
            .activations
            .iter()
            .filter(|record| &record.id == id)
            .cloned()
            .collect(),
    }
}

fn conflicting_activation_owners(
    resource: &ResourceId,
    target: &Path,
    activations: &[ActivationRecord],
) -> Vec<ResourceId> {
    let mut owners = Vec::new();
    for activation in activations {
        if activation.target == target
            && &activation.id != resource
            && !owners.contains(&activation.id)
        {
            owners.push(activation.id.clone());
        }
    }
    owners
}

fn activation_conflicts(activations: &[ActivationRecord]) -> Vec<ActivationOwnershipConflict> {
    let mut conflicts = Vec::new();

    for activation in activations {
        let mut owners = vec![activation.id.clone()];
        for other in activations {
            if other.target == activation.target && !owners.contains(&other.id) {
                owners.push(other.id.clone());
            }
        }

        if owners.len() > 1
            && !conflicts
                .iter()
                .any(|conflict: &ActivationOwnershipConflict| conflict.target == activation.target)
        {
            conflicts.push(ActivationOwnershipConflict {
                target: activation.target.clone(),
                owners,
            });
        }
    }

    conflicts
}

fn store_key_references(records: &[ResourceRecord]) -> Vec<StoreKeyReference> {
    let mut references: Vec<StoreKeyReference> = Vec::new();

    for record in records {
        let Some(key) = &record.artifact_key else {
            continue;
        };

        if let Some(existing) = references
            .iter_mut()
            .find(|reference| reference.key == *key)
        {
            if !existing.owners.contains(&record.id) {
                existing.owners.push(record.id.clone());
            }
        } else {
            references.push(StoreKeyReference {
                key: key.clone(),
                owners: vec![record.id.clone()],
            });
        }
    }

    references
}

fn store_key_references_for_retention(
    records: &[ResourceRecord],
    policy: StoreRetentionPolicy,
) -> Vec<StoreKeyReference> {
    let filtered = records
        .iter()
        .filter(|record| retention_matches(record.lifecycle.clone(), policy))
        .cloned()
        .collect::<Vec<_>>();
    store_key_references(&filtered)
}

fn retention_matches(lifecycle: ResourceLifecycle, policy: StoreRetentionPolicy) -> bool {
    match policy {
        StoreRetentionPolicy::AllReferenced => true,
        StoreRetentionPolicy::InstalledAndActive => matches!(
            lifecycle,
            ResourceLifecycle::Installed
                | ResourceLifecycle::Registered
                | ResourceLifecycle::Active
        ),
        StoreRetentionPolicy::ActiveOnly => lifecycle == ResourceLifecycle::Active,
    }
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

impl ResourceRecord {
    pub fn from_resolved_resource(resource: &pulith_resource::ResolvedResource) -> Self {
        Self {
            id: resource.spec().id.clone(),
            selector: resource.spec().version.clone(),
            resolved_version: Some(resource.version().clone()),
            locator: Some(resource.locator().clone()),
            artifact_key: None,
            install_path: None,
            lifecycle: ResourceLifecycle::Resolved,
            metadata: Metadata::new(),
        }
    }

    pub fn apply_patch(&mut self, patch: ResourceRecordPatch) {
        if let Some(selector) = patch.selector {
            self.selector = selector;
        }
        if let Some(resolved_version) = patch.resolved_version {
            self.resolved_version = resolved_version;
        }
        if let Some(locator) = patch.locator {
            self.locator = locator;
        }
        if let Some(artifact_key) = patch.artifact_key {
            self.artifact_key = artifact_key;
        }
        if let Some(install_path) = patch.install_path {
            self.install_path = install_path;
        }
        if let Some(lifecycle) = patch.lifecycle {
            self.lifecycle = lifecycle;
        }
        if let Some(metadata) = patch.metadata {
            self.metadata = metadata;
        }
    }
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

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
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
    use pulith_store::StoreRoots;

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
            .upsert_resolved_resource(&resolved, ResourceRecordPatch::default())
            .unwrap();

        let snapshot = state.load().unwrap();
        assert_eq!(snapshot.resources[0].lifecycle, ResourceLifecycle::Resolved);
    }

    #[test]
    fn upsert_resolved_resource_applies_patch_semantically() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();

        let resolved = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
        ))
        .resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(
                ValidUrl::parse("https://mirror.example.com/runtime.zip").unwrap(),
            ),
            None,
        );

        state
            .upsert_resolved_resource(
                &resolved,
                ResourceRecordPatch::install_path(Some(PathBuf::from("/opt/runtime")))
                    .with_lifecycle(ResourceLifecycle::Installed)
                    .with_metadata(Metadata::from([(
                        "source".to_string(),
                        "integration".to_string(),
                    )])),
            )
            .unwrap();

        let record = state
            .get_resource_record(&ResourceId::parse("example/runtime").unwrap())
            .unwrap()
            .unwrap();
        assert_eq!(record.lifecycle, ResourceLifecycle::Installed);
        assert_eq!(record.install_path, Some(PathBuf::from("/opt/runtime")));
        assert_eq!(
            record.metadata.get("source").map(String::as_str),
            Some("integration")
        );
    }

    #[test]
    fn ensure_patch_and_lookup_are_ergonomic() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let id = ResourceId::parse("example/runtime").unwrap();

        state
            .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id,
                ResourceRecordPatch {
                    lifecycle: Some(ResourceLifecycle::Resolved),
                    install_path: Some(Some(PathBuf::from("/opt/runtime"))),
                    ..ResourceRecordPatch::default()
                },
            )
            .unwrap();

        let record = state.get_resource_record(&id).unwrap().unwrap();
        assert_eq!(record.lifecycle, ResourceLifecycle::Resolved);
        assert_eq!(record.install_path, Some(PathBuf::from("/opt/runtime")));
    }

    #[test]
    fn record_activation_appends_entry() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let id = ResourceId::parse("example/runtime").unwrap();

        state
            .record_activation(&id, PathBuf::from("/active/runtime"))
            .unwrap();

        let activations = state.list_activation_records(&id).unwrap();
        assert_eq!(activations.len(), 1);
        assert_eq!(activations[0].target, PathBuf::from("/active/runtime"));
    }

    #[test]
    fn resource_state_can_be_captured_and_restored() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let id = ResourceId::parse("example/runtime").unwrap();

        state
            .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id,
                ResourceRecordPatch::install_path(Some(PathBuf::from("/opt/runtime")))
                    .with_lifecycle(ResourceLifecycle::Installed),
            )
            .unwrap();
        state
            .record_activation(&id, PathBuf::from("/active/runtime"))
            .unwrap();

        let captured = state.capture_resource_state(&id).unwrap();

        state.remove_resource_record(&id).unwrap();
        state.remove_activation_records(&id).unwrap();
        assert!(state.get_resource_record(&id).unwrap().is_none());

        state.restore_resource_state(&captured).unwrap();

        let restored = state.get_resource_record(&id).unwrap().unwrap();
        assert_eq!(restored.lifecycle, ResourceLifecycle::Installed);
        assert_eq!(restored.install_path, Some(PathBuf::from("/opt/runtime")));
        let activations = state.list_activation_records(&id).unwrap();
        assert_eq!(activations.len(), 1);
        assert_eq!(activations[0].target, PathBuf::from("/active/runtime"));
    }

    #[test]
    fn resource_inspection_reports_missing_runtime_state() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();
        let id = ResourceId::parse("example/runtime").unwrap();
        let key = StoreKey::logical("runtime").unwrap();

        state
            .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id,
                ResourceRecordPatch::artifact_key(Some(key.clone()))
                    .with_install_path(Some(temp.path().join("missing-install")))
                    .with_lifecycle(ResourceLifecycle::Installed),
            )
            .unwrap();
        state
            .record_activation(&id, temp.path().join("active/runtime"))
            .unwrap();

        let inspection = state.inspect_resource(&id, Some(&store)).unwrap();

        assert!(
            inspection
                .issues
                .contains(&ResourceInspectionIssue::MissingInstallPath {
                    resource: id.clone(),
                    path: temp.path().join("missing-install"),
                })
        );
        assert!(
            inspection
                .issues
                .contains(&ResourceInspectionIssue::MissingActivationTarget {
                    resource: id.clone(),
                    target: temp.path().join("active/runtime"),
                })
        );
        assert!(
            inspection
                .issues
                .contains(&ResourceInspectionIssue::MissingStoreEntry {
                    resource: id.clone(),
                    key: key.clone(),
                })
        );
        assert!(
            inspection
                .issues
                .contains(&ResourceInspectionIssue::MissingStoreMetadata { resource: id, key })
        );
    }

    #[test]
    fn resource_inspection_can_be_clean_when_state_and_store_are_consistent() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();
        let id = ResourceId::parse("example/runtime").unwrap();
        let key = StoreKey::logical("runtime").unwrap();
        let install_root = temp.path().join("install/runtime");
        let activation_target = temp.path().join("active/runtime");

        std::fs::create_dir_all(&install_root).unwrap();
        std::fs::create_dir_all(activation_target.parent().unwrap()).unwrap();
        std::fs::write(&activation_target, b"active").unwrap();
        store.put_artifact_bytes(&key, b"payload").unwrap();

        state
            .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id,
                ResourceRecordPatch::artifact_key(Some(key))
                    .with_install_path(Some(install_root))
                    .with_lifecycle(ResourceLifecycle::Installed),
            )
            .unwrap();
        state.record_activation(&id, activation_target).unwrap();

        let inspection = state.inspect_resource(&id, Some(&store)).unwrap();
        assert!(inspection.issues.is_empty());
    }

    #[test]
    fn resource_repair_plan_suggests_explicit_state_cleanup_actions() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();
        let id = ResourceId::parse("example/runtime").unwrap();
        let key = StoreKey::logical("runtime").unwrap();
        let missing_install = temp.path().join("missing-install");
        let missing_target = temp.path().join("active/runtime");

        state
            .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id,
                ResourceRecordPatch::artifact_key(Some(key.clone()))
                    .with_install_path(Some(missing_install.clone()))
                    .with_lifecycle(ResourceLifecycle::Installed),
            )
            .unwrap();
        state
            .record_activation(&id, missing_target.clone())
            .unwrap();

        let plan = state.plan_resource_state_repair(&id, Some(&store)).unwrap();

        assert!(
            plan.actions
                .contains(&ResourceRepairAction::ClearInstallPath {
                    resource: id.clone(),
                })
        );
        assert!(
            plan.actions
                .contains(&ResourceRepairAction::ClearArtifactKey {
                    resource: id.clone(),
                })
        );
        assert!(
            plan.actions
                .contains(&ResourceRepairAction::RemoveActivationRecord {
                    resource: id,
                    target: missing_target,
                })
        );
    }

    #[test]
    fn resource_repair_plan_can_be_applied_explicitly() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();
        let id = ResourceId::parse("example/runtime").unwrap();
        let key = StoreKey::logical("runtime").unwrap();

        state
            .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id,
                ResourceRecordPatch::artifact_key(Some(key))
                    .with_install_path(Some(temp.path().join("missing-install")))
                    .with_lifecycle(ResourceLifecycle::Installed),
            )
            .unwrap();
        state
            .record_activation(&id, temp.path().join("active/runtime"))
            .unwrap();

        let plan = state.plan_resource_state_repair(&id, Some(&store)).unwrap();
        state.apply_resource_state_repair(&plan).unwrap();

        let record = state.get_resource_record(&id).unwrap().unwrap();
        assert_eq!(record.install_path, None);
        assert_eq!(record.artifact_key, None);
        assert!(state.list_activation_records(&id).unwrap().is_empty());
    }

    #[test]
    fn resource_inspection_reports_activation_target_conflicts() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let shared_target = temp.path().join("active/shared-runtime");
        std::fs::create_dir_all(shared_target.parent().unwrap()).unwrap();
        std::fs::write(&shared_target, b"active").unwrap();

        let first = ResourceId::parse("example/runtime-a").unwrap();
        let second = ResourceId::parse("example/runtime-b").unwrap();

        state
            .ensure_resource_record(first.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .ensure_resource_record(second.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .record_activation(&first, shared_target.clone())
            .unwrap();
        state
            .record_activation(&second, shared_target.clone())
            .unwrap();

        let inspection = state.inspect_resource(&first, None).unwrap();
        assert!(
            inspection
                .issues
                .contains(&ResourceInspectionIssue::ActivationTargetConflict {
                    resource: first.clone(),
                    target: shared_target.clone(),
                    conflicting_owners: vec![second.clone()],
                })
        );

        let conflicts = state.list_activation_conflicts().unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].target, shared_target);
        assert!(conflicts[0].owners.contains(&first));
        assert!(conflicts[0].owners.contains(&second));
    }

    #[test]
    fn state_can_list_store_key_references() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let shared_key = StoreKey::logical("runtime-shared").unwrap();

        for resource in ["example/runtime-a", "example/runtime-b"] {
            let id = ResourceId::parse(resource).unwrap();
            state
                .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
                .unwrap();
            state
                .patch_resource_record(
                    &id,
                    ResourceRecordPatch::artifact_key(Some(shared_key.clone())),
                )
                .unwrap();
        }

        let references = state.list_store_references().unwrap();
        assert_eq!(references.len(), 1);
        assert_eq!(references[0].key, shared_key);
        assert_eq!(references[0].owners.len(), 2);
    }

    #[test]
    fn state_can_filter_protected_store_keys_by_retention_policy() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();

        let active_id = ResourceId::parse("example/runtime-active").unwrap();
        let installed_id = ResourceId::parse("example/runtime-installed").unwrap();
        let fetched_id = ResourceId::parse("example/runtime-fetched").unwrap();

        let active_key = StoreKey::logical("runtime-active").unwrap();
        let installed_key = StoreKey::logical("runtime-installed").unwrap();
        let fetched_key = StoreKey::logical("runtime-fetched").unwrap();

        for (id, key, lifecycle) in [
            (&active_id, &active_key, ResourceLifecycle::Active),
            (&installed_id, &installed_key, ResourceLifecycle::Installed),
            (&fetched_id, &fetched_key, ResourceLifecycle::Fetched),
        ] {
            state
                .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
                .unwrap();
            state
                .patch_resource_record(
                    id,
                    ResourceRecordPatch::artifact_key(Some(key.clone())).with_lifecycle(lifecycle),
                )
                .unwrap();
        }

        let all = state
            .protected_store_keys(StoreRetentionPolicy::AllReferenced)
            .unwrap();
        assert_eq!(all.len(), 3);

        let installed_and_active = state
            .protected_store_keys(StoreRetentionPolicy::InstalledAndActive)
            .unwrap();
        assert!(installed_and_active.contains(&active_key));
        assert!(installed_and_active.contains(&installed_key));
        assert!(!installed_and_active.contains(&fetched_key));

        let active_only = state
            .protected_store_keys(StoreRetentionPolicy::ActiveOnly)
            .unwrap();
        assert_eq!(active_only, vec![active_key]);
    }

    #[test]
    fn retention_policy_can_protect_store_metadata_during_prune() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();

        let active_id = ResourceId::parse("example/runtime-active").unwrap();
        let inactive_id = ResourceId::parse("example/runtime-fetched").unwrap();
        let active_key = StoreKey::logical("runtime-active").unwrap();
        let inactive_key = StoreKey::logical("runtime-fetched").unwrap();

        for key in [&active_key, &inactive_key] {
            store.put_artifact_bytes(key, b"payload").unwrap();
            std::fs::remove_file(store.artifact_path(key)).unwrap();
        }

        state
            .ensure_resource_record(active_id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &active_id,
                ResourceRecordPatch::artifact_key(Some(active_key.clone()))
                    .with_lifecycle(ResourceLifecycle::Active),
            )
            .unwrap();

        state
            .ensure_resource_record(inactive_id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &inactive_id,
                ResourceRecordPatch::artifact_key(Some(inactive_key.clone()))
                    .with_lifecycle(ResourceLifecycle::Fetched),
            )
            .unwrap();

        let protected = state
            .protected_store_keys(StoreRetentionPolicy::ActiveOnly)
            .unwrap();
        let report = store.prune_missing_with_protection(&protected).unwrap();

        assert_eq!(report.removed_metadata, 1);
        assert_eq!(report.protected_metadata, 1);
        assert!(store.has_metadata(&active_key));
        assert!(!store.has_metadata(&inactive_key));
    }

    #[test]
    fn state_can_plan_store_metadata_retention() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();

        let active_id = ResourceId::parse("example/runtime-active").unwrap();
        let fetched_id = ResourceId::parse("example/runtime-fetched").unwrap();
        let active_key = StoreKey::logical("runtime-active").unwrap();
        let fetched_key = StoreKey::logical("runtime-fetched").unwrap();

        for key in [&active_key, &fetched_key] {
            store.put_artifact_bytes(key, b"payload").unwrap();
            std::fs::remove_file(store.artifact_path(key)).unwrap();
        }

        state
            .ensure_resource_record(active_id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &active_id,
                ResourceRecordPatch::artifact_key(Some(active_key.clone()))
                    .with_lifecycle(ResourceLifecycle::Active),
            )
            .unwrap();

        state
            .ensure_resource_record(fetched_id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &fetched_id,
                ResourceRecordPatch::artifact_key(Some(fetched_key.clone()))
                    .with_lifecycle(ResourceLifecycle::Fetched),
            )
            .unwrap();

        let plan = state
            .plan_store_metadata_retention(&store, StoreRetentionPolicy::ActiveOnly)
            .unwrap();

        assert_eq!(plan.policy, StoreRetentionPolicy::ActiveOnly);
        assert_eq!(plan.protected_keys, vec![active_key.clone()]);
        assert_eq!(plan.protected_metadata.len(), 1);
        assert_eq!(plan.protected_metadata[0].key, active_key);
        assert_eq!(plan.removable_metadata.len(), 1);
        assert_eq!(plan.removable_metadata[0].key, fetched_key);
    }
}
