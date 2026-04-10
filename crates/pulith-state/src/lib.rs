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
    ) -> Result<ResourceInspectionReport> {
        let full_snapshot = self.load()?;
        let snapshot = capture_resource_state_from_snapshot(&full_snapshot, id);
        Ok(ResourceInspectionReport::from_snapshot(
            snapshot,
            &full_snapshot.activations,
            store,
        ))
    }

    pub fn inspect_resource_legacy(
        &self,
        id: &ResourceId,
        store: Option<&StoreReady>,
    ) -> Result<ResourceInspection> {
        Ok(self.inspect_resource(id, store)?.into_legacy())
    }

    pub fn list_activation_conflicts(&self) -> Result<Vec<ActivationOwnershipConflict>> {
        let report = self.activation_ownership_report()?;
        Ok(report
            .entries
            .into_iter()
            .filter(|entry| entry.owners.len() > 1)
            .map(|entry| ActivationOwnershipConflict {
                target: entry.target,
                owners: entry.owners,
            })
            .collect())
    }

    pub fn activation_ownership_report(&self) -> Result<ActivationOwnershipReport> {
        let snapshot = self.load()?;
        Ok(ActivationOwnershipReport::from_activations(
            &snapshot.activations,
        ))
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
        let reasoned = self.plan_store_metadata_retention_reasoned(store, policy)?;

        let protected_keys = reasoned
            .protected_keys
            .iter()
            .map(|entry| entry.key.clone())
            .collect();
        let removable_metadata = reasoned
            .removable_metadata
            .iter()
            .map(|entry| entry.record.clone())
            .collect();
        let protected_metadata = reasoned
            .protected_metadata
            .iter()
            .map(|entry| entry.record.clone())
            .collect();

        Ok(StoreRetentionPlan {
            policy,
            protected_keys,
            removable_metadata,
            protected_metadata,
        })
    }

    pub fn plan_store_metadata_retention_reasoned(
        &self,
        store: &StoreReady,
        policy: StoreRetentionPolicy,
    ) -> Result<ReasonedStoreRetentionPlan> {
        let snapshot = self.load()?;
        let protected_keys = protected_store_keys_with_reasons(&snapshot.resources, policy);
        let protected_key_values = protected_keys
            .iter()
            .map(|entry| entry.key.clone())
            .collect::<Vec<_>>();

        let mut metadata_plan = store.plan_metadata_prune(&protected_key_values)?;
        metadata_plan
            .protected
            .sort_by_key(|record| record.key.relative_name());
        metadata_plan
            .removable
            .sort_by_key(|record| record.key.relative_name());

        let mut protected_metadata = metadata_plan
            .protected
            .into_iter()
            .map(|record| {
                let reasons = protected_metadata_reasons(&record, &protected_keys);
                ProtectedStoreMetadata { record, reasons }
            })
            .collect::<Vec<_>>();
        protected_metadata.sort_by_key(|entry| entry.record.key.relative_name());

        let mut removable_metadata = metadata_plan
            .removable
            .into_iter()
            .map(|record| {
                let reasons = removable_metadata_reasons(&record, &snapshot.resources, policy);
                RemovableStoreMetadata { record, reasons }
            })
            .collect::<Vec<_>>();
        removable_metadata.sort_by_key(|entry| entry.record.key.relative_name());

        Ok(ReasonedStoreRetentionPlan {
            policy,
            protected_keys,
            protected_metadata,
            removable_metadata,
        })
    }

    pub fn plan_ownership_and_retention(
        &self,
        store: &StoreReady,
        policy: StoreRetentionPolicy,
    ) -> Result<OwnershipRetentionPlan> {
        Ok(OwnershipRetentionPlan {
            ownership: self.activation_ownership_report()?,
            retention: self.plan_store_metadata_retention_reasoned(store, policy)?,
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
            if let Some(install_path) = &record.install_path
                && !install_path.exists()
            {
                issues.push(ResourceInspectionIssue::MissingInstallPath {
                    resource: snapshot.resource.clone(),
                    path: install_path.clone(),
                });
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InspectionSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum InspectionCategory {
    ResourceRecord,
    InstallPath,
    ActivationTarget,
    ActivationOwnership,
    StoreEntry,
    StoreMetadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceInspectionFinding {
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

impl ResourceInspectionFinding {
    pub fn severity(&self) -> InspectionSeverity {
        match self {
            Self::MissingResourceRecord { .. }
            | Self::MissingInstallPath { .. }
            | Self::MissingActivationTarget { .. }
            | Self::MissingStoreEntry { .. } => InspectionSeverity::Error,
            Self::ActivationTargetConflict { .. } | Self::MissingStoreMetadata { .. } => {
                InspectionSeverity::Warning
            }
        }
    }

    pub fn category(&self) -> InspectionCategory {
        match self {
            Self::MissingResourceRecord { .. } => InspectionCategory::ResourceRecord,
            Self::MissingInstallPath { .. } => InspectionCategory::InstallPath,
            Self::MissingActivationTarget { .. } => InspectionCategory::ActivationTarget,
            Self::ActivationTargetConflict { .. } => InspectionCategory::ActivationOwnership,
            Self::MissingStoreEntry { .. } => InspectionCategory::StoreEntry,
            Self::MissingStoreMetadata { .. } => InspectionCategory::StoreMetadata,
        }
    }

    fn sort_key(&self) -> (InspectionSeverity, InspectionCategory, String, String) {
        let detail = match self {
            Self::MissingResourceRecord { resource } => (resource.as_string(), String::new()),
            Self::MissingInstallPath { resource, path } => {
                (resource.as_string(), path.display().to_string())
            }
            Self::MissingActivationTarget { resource, target } => {
                (resource.as_string(), target.display().to_string())
            }
            Self::ActivationTargetConflict {
                resource,
                target,
                conflicting_owners,
            } => (
                resource.as_string(),
                format!(
                    "{}:{}",
                    target.display(),
                    conflicting_owners
                        .iter()
                        .map(ResourceId::as_string)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            ),
            Self::MissingStoreEntry { resource, key }
            | Self::MissingStoreMetadata { resource, key } => {
                (resource.as_string(), key.relative_name())
            }
        };

        (self.severity(), self.category(), detail.0, detail.1)
    }

    pub fn summary_label(&self) -> &'static str {
        match self {
            Self::MissingResourceRecord { .. } => "missing-resource-record",
            Self::MissingInstallPath { .. } => "missing-install-path",
            Self::MissingActivationTarget { .. } => "missing-activation-target",
            Self::ActivationTargetConflict { .. } => "activation-target-conflict",
            Self::MissingStoreEntry { .. } => "missing-store-entry",
            Self::MissingStoreMetadata { .. } => "missing-store-metadata",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ResourceInspectionSummary {
    pub total_findings: usize,
    pub info_count: usize,
    pub warning_count: usize,
    pub error_count: usize,
}

impl ResourceInspectionSummary {
    fn from_findings(findings: &[ResourceInspectionFinding]) -> Self {
        let mut summary = Self {
            total_findings: findings.len(),
            ..Self::default()
        };

        for finding in findings {
            match finding.severity() {
                InspectionSeverity::Info => summary.info_count += 1,
                InspectionSeverity::Warning => summary.warning_count += 1,
                InspectionSeverity::Error => summary.error_count += 1,
            }
        }

        summary
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceInspectionReport {
    pub snapshot: ResourceStateSnapshot,
    pub findings: Vec<ResourceInspectionFinding>,
    pub summary: ResourceInspectionSummary,
}

impl ResourceInspectionReport {
    pub fn from_snapshot(
        snapshot: ResourceStateSnapshot,
        all_activations: &[ActivationRecord],
        store: Option<&StoreReady>,
    ) -> Self {
        let legacy = ResourceInspection::from_snapshot(snapshot, all_activations, store);
        Self::from_legacy(legacy)
    }

    pub fn from_legacy(inspection: ResourceInspection) -> Self {
        let mut findings = inspection
            .issues
            .iter()
            .cloned()
            .map(ResourceInspectionFinding::from)
            .collect::<Vec<_>>();
        findings.sort_by_key(ResourceInspectionFinding::sort_key);
        let summary = ResourceInspectionSummary::from_findings(&findings);

        Self {
            snapshot: inspection.snapshot,
            findings,
            summary,
        }
    }

    pub fn is_clean(&self) -> bool {
        self.findings.is_empty()
    }

    pub fn into_legacy(self) -> ResourceInspection {
        ResourceInspection {
            snapshot: self.snapshot,
            issues: self
                .findings
                .into_iter()
                .map(ResourceInspectionIssue::from)
                .collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceRepairPlan {
    pub inspection: ResourceInspectionReport,
    pub actions: Vec<ResourceRepairAction>,
}

impl ResourceRepairPlan {
    pub fn from_inspection(inspection: ResourceInspectionReport) -> Self {
        let mut actions = Vec::new();

        for finding in &inspection.findings {
            match finding {
                ResourceInspectionFinding::MissingInstallPath { resource, .. } => {
                    push_unique_action(
                        &mut actions,
                        ResourceRepairAction::ClearInstallPath {
                            resource: resource.clone(),
                        },
                    );
                }
                ResourceInspectionFinding::MissingActivationTarget { resource, target } => {
                    push_unique_action(
                        &mut actions,
                        ResourceRepairAction::RemoveActivationRecord {
                            resource: resource.clone(),
                            target: target.clone(),
                        },
                    );
                }
                ResourceInspectionFinding::MissingStoreEntry { resource, .. } => {
                    push_unique_action(
                        &mut actions,
                        ResourceRepairAction::ClearArtifactKey {
                            resource: resource.clone(),
                        },
                    );
                }
                ResourceInspectionFinding::MissingResourceRecord { .. }
                | ResourceInspectionFinding::MissingStoreMetadata { .. }
                | ResourceInspectionFinding::ActivationTargetConflict { .. } => {}
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

impl From<ResourceInspectionIssue> for ResourceInspectionFinding {
    fn from(issue: ResourceInspectionIssue) -> Self {
        match issue {
            ResourceInspectionIssue::MissingResourceRecord { resource } => {
                Self::MissingResourceRecord { resource }
            }
            ResourceInspectionIssue::MissingInstallPath { resource, path } => {
                Self::MissingInstallPath { resource, path }
            }
            ResourceInspectionIssue::MissingActivationTarget { resource, target } => {
                Self::MissingActivationTarget { resource, target }
            }
            ResourceInspectionIssue::ActivationTargetConflict {
                resource,
                target,
                conflicting_owners,
            } => Self::ActivationTargetConflict {
                resource,
                target,
                conflicting_owners,
            },
            ResourceInspectionIssue::MissingStoreEntry { resource, key } => {
                Self::MissingStoreEntry { resource, key }
            }
            ResourceInspectionIssue::MissingStoreMetadata { resource, key } => {
                Self::MissingStoreMetadata { resource, key }
            }
        }
    }
}

impl From<ResourceInspectionFinding> for ResourceInspectionIssue {
    fn from(finding: ResourceInspectionFinding) -> Self {
        match finding {
            ResourceInspectionFinding::MissingResourceRecord { resource } => {
                Self::MissingResourceRecord { resource }
            }
            ResourceInspectionFinding::MissingInstallPath { resource, path } => {
                Self::MissingInstallPath { resource, path }
            }
            ResourceInspectionFinding::MissingActivationTarget { resource, target } => {
                Self::MissingActivationTarget { resource, target }
            }
            ResourceInspectionFinding::ActivationTargetConflict {
                resource,
                target,
                conflicting_owners,
            } => Self::ActivationTargetConflict {
                resource,
                target,
                conflicting_owners,
            },
            ResourceInspectionFinding::MissingStoreEntry { resource, key } => {
                Self::MissingStoreEntry { resource, key }
            }
            ResourceInspectionFinding::MissingStoreMetadata { resource, key } => {
                Self::MissingStoreMetadata { resource, key }
            }
        }
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum OwnershipSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OwnershipReason {
    SharedActivationTarget {
        target: PathBuf,
        owners: Vec<ResourceId>,
    },
    StateStoreReference {
        key: StoreKey,
        owner: ResourceId,
        lifecycle: ResourceLifecycle,
    },
    RetentionPolicyExcludesLifecycle {
        policy: StoreRetentionPolicy,
        resource: ResourceId,
        lifecycle: ResourceLifecycle,
    },
    UnreferencedStoreMetadata {
        key: StoreKey,
    },
}

impl OwnershipReason {
    fn sort_key(&self) -> (u8, String, String) {
        match self {
            Self::SharedActivationTarget { target, owners } => (
                0,
                target.display().to_string(),
                owners
                    .iter()
                    .map(ResourceId::as_string)
                    .collect::<Vec<_>>()
                    .join(","),
            ),
            Self::StateStoreReference {
                key,
                owner,
                lifecycle,
            } => (
                1,
                key.relative_name(),
                format!("{}:{lifecycle:?}", owner.as_string()),
            ),
            Self::RetentionPolicyExcludesLifecycle {
                policy,
                resource,
                lifecycle,
            } => (2, resource.as_string(), format!("{policy:?}:{lifecycle:?}")),
            Self::UnreferencedStoreMetadata { key } => (3, key.relative_name(), String::new()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivationOwnershipEntry {
    pub target: PathBuf,
    pub owners: Vec<ResourceId>,
    pub severity: OwnershipSeverity,
    pub reasons: Vec<OwnershipReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ActivationOwnershipReport {
    pub entries: Vec<ActivationOwnershipEntry>,
}

impl ActivationOwnershipReport {
    pub fn from_activations(activations: &[ActivationRecord]) -> Self {
        let mut entries = activation_ownership_entries(activations);
        entries.sort_by_key(|entry| {
            (
                entry.severity,
                entry.target.display().to_string(),
                entry
                    .owners
                    .iter()
                    .map(ResourceId::as_string)
                    .collect::<Vec<_>>()
                    .join(","),
            )
        });
        Self { entries }
    }

    pub fn is_clean(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreKeyReference {
    pub key: StoreKey,
    pub owners: Vec<ResourceId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedStoreKey {
    pub key: StoreKey,
    pub reasons: Vec<OwnershipReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtectedStoreMetadata {
    pub record: StoreMetadataRecord,
    pub reasons: Vec<OwnershipReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemovableStoreMetadata {
    pub record: StoreMetadataRecord,
    pub reasons: Vec<OwnershipReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReasonedStoreRetentionPlan {
    pub policy: StoreRetentionPolicy,
    pub protected_keys: Vec<ProtectedStoreKey>,
    pub protected_metadata: Vec<ProtectedStoreMetadata>,
    pub removable_metadata: Vec<RemovableStoreMetadata>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OwnershipRetentionPlan {
    pub ownership: ActivationOwnershipReport,
    pub retention: ReasonedStoreRetentionPlan,
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
    owners.sort_by_key(ResourceId::as_string);
    owners
}

fn activation_ownership_entries(activations: &[ActivationRecord]) -> Vec<ActivationOwnershipEntry> {
    let mut entries = Vec::new();

    for activation in activations {
        if entries
            .iter()
            .any(|entry: &ActivationOwnershipEntry| entry.target == activation.target)
        {
            continue;
        }

        let mut owners = activations
            .iter()
            .filter(|other| other.target == activation.target)
            .map(|other| other.id.clone())
            .collect::<Vec<_>>();
        owners.sort_by_key(ResourceId::as_string);
        owners.dedup();

        if owners.len() <= 1 {
            continue;
        }

        entries.push(ActivationOwnershipEntry {
            target: activation.target.clone(),
            owners: owners.clone(),
            severity: OwnershipSeverity::Warning,
            reasons: vec![OwnershipReason::SharedActivationTarget {
                target: activation.target.clone(),
                owners,
            }],
        });
    }

    entries
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

    for reference in &mut references {
        reference.owners.sort_by_key(ResourceId::as_string);
        reference.owners.dedup();
    }
    references.sort_by_key(|reference| reference.key.relative_name());

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

fn protected_store_keys_with_reasons(
    records: &[ResourceRecord],
    policy: StoreRetentionPolicy,
) -> Vec<ProtectedStoreKey> {
    let mut entries = Vec::<ProtectedStoreKey>::new();

    for record in records {
        let Some(key) = &record.artifact_key else {
            continue;
        };

        if !retention_matches(record.lifecycle.clone(), policy) {
            continue;
        }

        let reason = OwnershipReason::StateStoreReference {
            key: key.clone(),
            owner: record.id.clone(),
            lifecycle: record.lifecycle.clone(),
        };
        if let Some(existing) = entries.iter_mut().find(|entry| entry.key == *key) {
            existing.reasons.push(reason);
        } else {
            entries.push(ProtectedStoreKey {
                key: key.clone(),
                reasons: vec![reason],
            });
        }
    }

    for entry in &mut entries {
        entry.reasons.sort_by_key(OwnershipReason::sort_key);
        entry.reasons.dedup();
    }
    entries.sort_by_key(|entry| entry.key.relative_name());

    entries
}

fn protected_metadata_reasons(
    record: &StoreMetadataRecord,
    protected_keys: &[ProtectedStoreKey],
) -> Vec<OwnershipReason> {
    protected_keys
        .iter()
        .find(|entry| entry.key == record.key)
        .map(|entry| entry.reasons.clone())
        .unwrap_or_default()
}

fn removable_metadata_reasons(
    record: &StoreMetadataRecord,
    resources: &[ResourceRecord],
    policy: StoreRetentionPolicy,
) -> Vec<OwnershipReason> {
    let mut reasons = Vec::new();

    let referencing_records = resources
        .iter()
        .filter(|resource| resource.artifact_key.as_ref() == Some(&record.key))
        .collect::<Vec<_>>();

    if referencing_records.is_empty() {
        reasons.push(OwnershipReason::UnreferencedStoreMetadata {
            key: record.key.clone(),
        });
    } else {
        for resource in referencing_records {
            reasons.push(OwnershipReason::RetentionPolicyExcludesLifecycle {
                policy,
                resource: resource.id.clone(),
                lifecycle: resource.lifecycle.clone(),
            });
        }
    }

    reasons.sort_by_key(OwnershipReason::sort_key);
    reasons.dedup();
    reasons
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
                .findings
                .contains(&ResourceInspectionFinding::MissingInstallPath {
                    resource: id.clone(),
                    path: temp.path().join("missing-install"),
                })
        );
        assert!(inspection.findings.contains(
            &ResourceInspectionFinding::MissingActivationTarget {
                resource: id.clone(),
                target: temp.path().join("active/runtime"),
            }
        ));
        assert!(
            inspection
                .findings
                .contains(&ResourceInspectionFinding::MissingStoreEntry {
                    resource: id.clone(),
                    key: key.clone(),
                })
        );
        assert!(
            inspection
                .findings
                .contains(&ResourceInspectionFinding::MissingStoreMetadata { resource: id, key })
        );
        assert_eq!(inspection.summary.error_count, 3);
        assert_eq!(inspection.summary.warning_count, 1);
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
        assert!(inspection.is_clean());
        assert_eq!(inspection.summary.total_findings, 0);
    }

    #[test]
    fn resource_inspection_is_read_only() {
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

        let before = state.load().unwrap();
        let inspection = state.inspect_resource(&id, Some(&store)).unwrap();
        let after = state.load().unwrap();

        assert!(!inspection.is_clean());
        assert_eq!(before, after);
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
        assert!(inspection.findings.contains(
            &ResourceInspectionFinding::ActivationTargetConflict {
                resource: first.clone(),
                target: shared_target.clone(),
                conflicting_owners: vec![second.clone()],
            }
        ));

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

    #[test]
    fn activation_ownership_report_detects_shared_targets() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let shared_target = temp.path().join("active/shared-runtime");
        std::fs::create_dir_all(shared_target.parent().unwrap()).unwrap();
        std::fs::write(&shared_target, b"active").unwrap();

        let first = ResourceId::parse("example/runtime-a").unwrap();
        let second = ResourceId::parse("example/runtime-b").unwrap();

        state
            .record_activation(&first, shared_target.clone())
            .unwrap();
        state
            .record_activation(&second, shared_target.clone())
            .unwrap();

        let report = state.activation_ownership_report().unwrap();
        assert_eq!(report.entries.len(), 1);
        let entry = &report.entries[0];
        assert_eq!(entry.target, shared_target);
        assert_eq!(entry.severity, OwnershipSeverity::Warning);
        assert_eq!(entry.owners, vec![first.clone(), second.clone()]);
        assert_eq!(
            entry.reasons,
            vec![OwnershipReason::SharedActivationTarget {
                target: entry.target.clone(),
                owners: vec![first, second],
            }]
        );
    }

    #[test]
    fn reasoned_retention_plan_explains_lifecycle_based_protection_and_removal() {
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
        let orphaned_key = StoreKey::logical("runtime-orphaned").unwrap();
        let active_key = StoreKey::logical("runtime-active").unwrap();
        let fetched_key = StoreKey::logical("runtime-fetched").unwrap();

        for key in [&active_key, &fetched_key, &orphaned_key] {
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
            .plan_store_metadata_retention_reasoned(&store, StoreRetentionPolicy::ActiveOnly)
            .unwrap();

        assert_eq!(plan.protected_keys.len(), 1);
        assert_eq!(plan.protected_keys[0].key, active_key);
        assert_eq!(
            plan.protected_keys[0].reasons,
            vec![OwnershipReason::StateStoreReference {
                key: plan.protected_keys[0].key.clone(),
                owner: active_id,
                lifecycle: ResourceLifecycle::Active,
            }]
        );

        assert_eq!(plan.removable_metadata.len(), 2);
        assert_eq!(plan.removable_metadata[0].record.key, fetched_key);
        assert_eq!(
            plan.removable_metadata[0].reasons,
            vec![OwnershipReason::RetentionPolicyExcludesLifecycle {
                policy: StoreRetentionPolicy::ActiveOnly,
                resource: fetched_id,
                lifecycle: ResourceLifecycle::Fetched,
            }]
        );
        assert_eq!(plan.removable_metadata[1].record.key, orphaned_key);
        assert_eq!(
            plan.removable_metadata[1].reasons,
            vec![OwnershipReason::UnreferencedStoreMetadata {
                key: plan.removable_metadata[1].record.key.clone(),
            }]
        );
    }

    #[test]
    fn ownership_and_retention_plans_are_deterministic() {
        let temp = tempfile::tempdir().unwrap();
        let state = StateReady::initialize(temp.path().join("state.json")).unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("store/artifacts"),
            temp.path().join("store/extracts"),
            temp.path().join("store/metadata"),
        ))
        .unwrap();

        let id_a = ResourceId::parse("example/runtime-a").unwrap();
        let id_b = ResourceId::parse("example/runtime-b").unwrap();
        let key_a = StoreKey::logical("runtime-a").unwrap();
        let key_b = StoreKey::logical("runtime-b").unwrap();
        let shared_target = temp.path().join("active/shared");
        std::fs::create_dir_all(shared_target.parent().unwrap()).unwrap();
        std::fs::write(&shared_target, b"active").unwrap();

        for key in [&key_a, &key_b] {
            store.put_artifact_bytes(key, b"payload").unwrap();
            std::fs::remove_file(store.artifact_path(key)).unwrap();
        }

        state
            .ensure_resource_record(id_b.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id_b,
                ResourceRecordPatch::artifact_key(Some(key_b.clone()))
                    .with_lifecycle(ResourceLifecycle::Fetched),
            )
            .unwrap();
        state
            .ensure_resource_record(id_a.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id_a,
                ResourceRecordPatch::artifact_key(Some(key_a.clone()))
                    .with_lifecycle(ResourceLifecycle::Active),
            )
            .unwrap();
        state
            .record_activation(&id_b, shared_target.clone())
            .unwrap();
        state.record_activation(&id_a, shared_target).unwrap();

        let plan_one = state
            .plan_ownership_and_retention(&store, StoreRetentionPolicy::ActiveOnly)
            .unwrap();
        let plan_two = state
            .plan_ownership_and_retention(&store, StoreRetentionPolicy::ActiveOnly)
            .unwrap();

        assert_eq!(plan_one, plan_two);
    }

    #[test]
    fn ownership_and_retention_planning_is_read_only() {
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

        store.put_artifact_bytes(&key, b"payload").unwrap();
        std::fs::remove_file(store.artifact_path(&key)).unwrap();

        state
            .ensure_resource_record(id.clone(), VersionSelector::alias("lts").unwrap())
            .unwrap();
        state
            .patch_resource_record(
                &id,
                ResourceRecordPatch::artifact_key(Some(key))
                    .with_lifecycle(ResourceLifecycle::Fetched),
            )
            .unwrap();
        state
            .record_activation(&id, temp.path().join("active/runtime"))
            .unwrap();

        let before = state.load().unwrap();
        let _ = state.activation_ownership_report().unwrap();
        let _ = state
            .plan_store_metadata_retention_reasoned(&store, StoreRetentionPolicy::ActiveOnly)
            .unwrap();
        let _ = state
            .plan_ownership_and_retention(&store, StoreRetentionPolicy::ActiveOnly)
            .unwrap();
        let after = state.load().unwrap();

        assert_eq!(before, after);
    }
}
