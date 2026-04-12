//! Composable local artifact storage for Pulith.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pulith_archive::entry::ArchiveReport;
use pulith_fetch::{FetchReceipt, FetchSource};
use pulith_fs::{
    DEFAULT_COPY_ONLY_THRESHOLD_BYTES, FallBack, HardlinkOrCopyOptions, Workspace, atomic_write,
    copy_dir_all,
};
use pulith_resource::{Metadata, ResolvedResource, ResolvedVersion, ResourceId, ValidDigest};
use pulith_serde_backend::{JsonTextCodec, decode_slice, encode_pretty_vec};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StoreError>;
pub const STORE_METADATA_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone)]
pub struct ArtifactRegistration {
    pub source: PathBuf,
    pub provenance: Option<StoreProvenance>,
}

pub trait IntoArtifactRegistration {
    fn into_artifact_registration(self) -> ArtifactRegistration;
}

impl IntoArtifactRegistration for PathBuf {
    fn into_artifact_registration(self) -> ArtifactRegistration {
        ArtifactRegistration {
            source: self,
            provenance: None,
        }
    }
}

impl IntoArtifactRegistration for &Path {
    fn into_artifact_registration(self) -> ArtifactRegistration {
        ArtifactRegistration {
            source: self.to_path_buf(),
            provenance: None,
        }
    }
}

impl IntoArtifactRegistration for (&Path, StoreProvenance) {
    fn into_artifact_registration(self) -> ArtifactRegistration {
        ArtifactRegistration {
            source: self.0.to_path_buf(),
            provenance: Some(self.1),
        }
    }
}

impl IntoArtifactRegistration for (&Path, Option<StoreProvenance>) {
    fn into_artifact_registration(self) -> ArtifactRegistration {
        ArtifactRegistration {
            source: self.0.to_path_buf(),
            provenance: self.1,
        }
    }
}

impl IntoArtifactRegistration for &FetchReceipt {
    fn into_artifact_registration(self) -> ArtifactRegistration {
        ArtifactRegistration {
            source: self.destination.clone(),
            provenance: Some(StoreProvenance::from_fetch_receipt(self)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExtractRegistration {
    pub source_dir: PathBuf,
    pub provenance: Option<StoreProvenance>,
}

pub trait IntoExtractRegistration {
    fn into_extract_registration(self) -> ExtractRegistration;
}

impl IntoExtractRegistration for PathBuf {
    fn into_extract_registration(self) -> ExtractRegistration {
        ExtractRegistration {
            source_dir: self,
            provenance: None,
        }
    }
}

impl IntoExtractRegistration for &Path {
    fn into_extract_registration(self) -> ExtractRegistration {
        ExtractRegistration {
            source_dir: self.to_path_buf(),
            provenance: None,
        }
    }
}

impl IntoExtractRegistration for (&Path, StoreProvenance) {
    fn into_extract_registration(self) -> ExtractRegistration {
        ExtractRegistration {
            source_dir: self.0.to_path_buf(),
            provenance: Some(self.1),
        }
    }
}

impl IntoExtractRegistration for (&Path, Option<StoreProvenance>) {
    fn into_extract_registration(self) -> ExtractRegistration {
        ExtractRegistration {
            source_dir: self.0.to_path_buf(),
            provenance: self.1,
        }
    }
}

impl IntoExtractRegistration for (&Path, &ArchiveReport) {
    fn into_extract_registration(self) -> ExtractRegistration {
        ExtractRegistration {
            source_dir: self.0.to_path_buf(),
            provenance: Some(StoreProvenance::from_archive_report(self.1)),
        }
    }
}

impl IntoExtractRegistration for (&FetchReceipt, &Path, &ArchiveReport) {
    fn into_extract_registration(self) -> ExtractRegistration {
        ExtractRegistration {
            source_dir: self.1.to_path_buf(),
            provenance: Some(StoreProvenance::from_fetched_archive_extraction(
                self.0, self.2,
            )),
        }
    }
}

#[derive(Debug, Error)]
pub enum StoreError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Fs(#[from] pulith_fs::Error),
    #[error("store root is missing: {0}")]
    MissingRoot(&'static str),
    #[error("logical key must not be empty")]
    EmptyLogicalKey,
    #[error("file name is missing from source path {0}")]
    MissingFileName(PathBuf),
    #[error("invalid metadata file name for key {0}")]
    InvalidMetadataFileName(String),
    #[error("unsupported store metadata schema version: expected {expected}, got {actual}")]
    UnsupportedMetadataSchemaVersion { expected: u32, actual: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreRoots {
    pub artifacts: PathBuf,
    pub extracts: PathBuf,
    pub metadata: PathBuf,
}

impl StoreRoots {
    pub fn new(artifacts: PathBuf, extracts: PathBuf, metadata: PathBuf) -> Self {
        Self {
            artifacts,
            extracts,
            metadata,
        }
    }
}

#[derive(Debug, Clone)]
pub struct StoreReady {
    roots: StoreRoots,
}

impl StoreReady {
    pub fn initialize(roots: StoreRoots) -> Result<Self> {
        std::fs::create_dir_all(&roots.artifacts)?;
        std::fs::create_dir_all(&roots.extracts)?;
        std::fs::create_dir_all(&roots.metadata)?;
        Ok(Self { roots })
    }

    pub fn roots(&self) -> &StoreRoots {
        &self.roots
    }

    pub fn has_artifact(&self, key: &StoreKey) -> bool {
        self.artifact_path(key).exists()
    }

    pub fn has_extract(&self, key: &StoreKey) -> bool {
        self.extract_path(key).exists()
    }

    pub fn has_metadata(&self, key: &StoreKey) -> bool {
        self.metadata_path(key).exists()
    }

    pub fn artifact_path(&self, key: &StoreKey) -> PathBuf {
        self.roots.artifacts.join(key.relative_name())
    }

    pub fn extract_path(&self, key: &StoreKey) -> PathBuf {
        self.roots.extracts.join(key.relative_name())
    }

    pub fn metadata_path(&self, key: &StoreKey) -> PathBuf {
        self.roots
            .metadata
            .join(format!("{}.json", key.relative_name()))
    }

    pub fn get_artifact(&self, key: &StoreKey) -> Option<StoredArtifact> {
        self.lookup_stored(key, StoredKind::Artifact)
            .map(|artifact| StoredArtifact {
                key: artifact.key,
                path: artifact.path,
                provenance: artifact.provenance,
            })
    }

    pub fn get_extract(&self, key: &StoreKey) -> Option<ExtractedArtifact> {
        self.lookup_stored(key, StoredKind::Extract)
            .map(|extract| ExtractedArtifact {
                key: extract.key,
                path: extract.path,
                provenance: extract.provenance,
            })
    }

    pub fn get_artifact_for<K: KeyDerivation>(
        &self,
        resource: &ResolvedResource,
        derivation: &K,
    ) -> Option<StoredArtifact> {
        derivation
            .derive(resource)
            .as_ref()
            .and_then(|key| self.get_artifact(key))
    }

    pub fn get_extract_for<K: KeyDerivation>(
        &self,
        resource: &ResolvedResource,
        derivation: &K,
    ) -> Option<ExtractedArtifact> {
        derivation
            .derive(resource)
            .as_ref()
            .and_then(|key| self.get_extract(key))
    }

    pub fn get_metadata(&self, key: &StoreKey) -> Result<Option<StoreMetadataRecord>> {
        self.load_metadata_record(key)
    }

    pub fn get_metadata_for<K: KeyDerivation>(
        &self,
        resource: &ResolvedResource,
        derivation: &K,
    ) -> Result<Option<StoreMetadataRecord>> {
        derivation
            .derive(resource)
            .as_ref()
            .map_or(Ok(None), |key| self.get_metadata(key))
    }

    pub fn list_metadata(&self) -> Result<Vec<StoreMetadataRecord>> {
        let mut records = Vec::new();
        for entry in std::fs::read_dir(&self.roots.metadata)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let record = Self::decode_metadata_file(&entry.path())?;
            records.push(record);
        }
        Ok(records)
    }

    pub fn list_orphaned_metadata(&self) -> Result<Vec<StoreMetadataRecord>> {
        Ok(self
            .list_metadata()?
            .into_iter()
            .filter(|record| !self.record_target_exists(record))
            .collect())
    }

    pub fn get_orphaned_metadata_for<K: KeyDerivation>(
        &self,
        resource: &ResolvedResource,
        derivation: &K,
    ) -> Result<Option<StoreMetadataRecord>> {
        let Some(key) = derivation.derive(resource) else {
            return Ok(None);
        };

        let Some(record) = self.get_metadata(&key)? else {
            return Ok(None);
        };

        Ok((!self.record_target_exists(&record)).then_some(record))
    }

    pub fn plan_metadata_prune(&self, protected_keys: &[StoreKey]) -> Result<MetadataPrunePlan> {
        let mut plan = MetadataPrunePlan::default();

        for record in self.list_orphaned_metadata()? {
            if protected_keys.contains(&record.key) {
                plan.protected.push(record);
            } else {
                plan.removable.push(record);
            }
        }

        Ok(plan)
    }

    pub fn prune_missing(&self) -> Result<PruneReport> {
        self.prune_missing_with_protection(&[])
    }

    pub fn prune_missing_with_protection(
        &self,
        protected_keys: &[StoreKey],
    ) -> Result<PruneReport> {
        let mut report = PruneReport::default();
        let plan = self.plan_metadata_prune(protected_keys)?;
        report.protected_metadata = plan.protected.len();

        for record in plan.removable {
            let metadata_path = self.metadata_path(&record.key);
            if metadata_path.exists() {
                std::fs::remove_file(&metadata_path)?;
                report.removed_metadata += 1;
            }
        }
        Ok(report)
    }

    pub fn put_artifact_bytes(&self, key: &StoreKey, bytes: &[u8]) -> Result<StoredArtifact> {
        let path = self.artifact_path(key);
        atomic_write(&path, bytes, Default::default())?;
        let artifact = StoredArtifact {
            key: key.clone(),
            path,
            provenance: None,
        };
        self.persist_provenance(
            &artifact.key,
            StoredKind::Artifact,
            artifact.provenance.as_ref(),
        )?;
        Ok(artifact)
    }

    pub fn import_artifact(
        &self,
        key: &StoreKey,
        source: impl AsRef<Path>,
    ) -> Result<StoredArtifact> {
        self.import_artifact_with_provenance(key, source, None)
    }

    pub fn import_artifact_with_provenance(
        &self,
        key: &StoreKey,
        source: impl AsRef<Path>,
        provenance: Option<StoreProvenance>,
    ) -> Result<StoredArtifact> {
        let source = source.as_ref();
        let file_name = source
            .file_name()
            .ok_or_else(|| StoreError::MissingFileName(source.to_path_buf()))?;
        let artifact_root = self.artifact_path(key);
        if artifact_root.exists() {
            std::fs::remove_dir_all(&artifact_root)?;
        }
        let workspace_root = tempfile::tempdir()?;
        let workspace = Workspace::new(
            workspace_root.path().join("artifact"),
            artifact_root.clone(),
        )?;
        stage_artifact_file(&workspace, source, PathBuf::from(file_name))?;
        workspace.commit()?;

        let artifact = StoredArtifact {
            key: key.clone(),
            path: self.artifact_path(key).join(file_name),
            provenance,
        };
        self.persist_provenance(
            &artifact.key,
            StoredKind::Artifact,
            artifact.provenance.as_ref(),
        )?;
        Ok(artifact)
    }

    pub fn register_artifact(
        &self,
        key: &StoreKey,
        registration: impl IntoArtifactRegistration,
    ) -> Result<StoredArtifact> {
        let registration = registration.into_artifact_registration();
        self.import_artifact_with_provenance(key, registration.source, registration.provenance)
    }

    pub fn register_extract_dir(
        &self,
        key: &StoreKey,
        source_dir: impl AsRef<Path>,
    ) -> Result<ExtractedArtifact> {
        self.register_extract_dir_with_provenance(key, source_dir, None)
    }

    pub fn register_extract_dir_with_provenance(
        &self,
        key: &StoreKey,
        source_dir: impl AsRef<Path>,
        provenance: Option<StoreProvenance>,
    ) -> Result<ExtractedArtifact> {
        let source_dir = source_dir.as_ref();
        let target = self.extract_path(key);

        if target.exists() {
            std::fs::remove_dir_all(&target)?;
        }

        copy_dir_all(source_dir, &target)?;
        let artifact = ExtractedArtifact {
            key: key.clone(),
            path: target,
            provenance,
        };
        self.persist_provenance(
            &artifact.key,
            StoredKind::Extract,
            artifact.provenance.as_ref(),
        )?;
        Ok(artifact)
    }

    pub fn register_extract(
        &self,
        key: &StoreKey,
        registration: impl IntoExtractRegistration,
    ) -> Result<ExtractedArtifact> {
        let registration = registration.into_extract_registration();
        self.register_extract_dir_with_provenance(
            key,
            registration.source_dir,
            registration.provenance,
        )
    }

    fn persist_provenance(
        &self,
        key: &StoreKey,
        kind: StoredKind,
        provenance: Option<&StoreProvenance>,
    ) -> Result<()> {
        let record = StoreMetadataRecord {
            schema_version: STORE_METADATA_SCHEMA_VERSION,
            key: key.clone(),
            kind,
            provenance: provenance.cloned(),
            updated_at_unix: now_unix(),
        };
        let bytes = encode_pretty_vec(&JsonTextCodec, &record).map_err(|error| {
            StoreError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        atomic_write(self.metadata_path(key), &bytes, Default::default())?;
        Ok(())
    }

    fn load_provenance(&self, key: &StoreKey) -> Result<Option<StoreProvenance>> {
        Ok(self
            .load_metadata_record(key)?
            .and_then(|record| record.provenance))
    }

    fn load_metadata_record(&self, key: &StoreKey) -> Result<Option<StoreMetadataRecord>> {
        let path = self.metadata_path(key);
        if !path.exists() {
            return Ok(None);
        }
        Ok(Some(Self::decode_metadata_file(&path)?))
    }

    fn decode_metadata_file(path: &Path) -> Result<StoreMetadataRecord> {
        let bytes = std::fs::read(path)?;
        let record: StoreMetadataRecord =
            decode_slice(&JsonTextCodec, &bytes).map_err(|error| {
                StoreError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            })?;
        record.validate()?;
        Ok(record)
    }

    fn lookup_stored(&self, key: &StoreKey, kind: StoredKind) -> Option<StoredEntry> {
        let path = match kind {
            StoredKind::Artifact => self.artifact_path(key),
            StoredKind::Extract => self.extract_path(key),
        };
        if !path.exists() {
            return None;
        }

        Some(StoredEntry {
            key: key.clone(),
            path,
            provenance: self.load_provenance(key).ok().flatten(),
        })
    }

    fn record_target_exists(&self, record: &StoreMetadataRecord) -> bool {
        match record.kind {
            StoredKind::Artifact => self.has_artifact(&record.key),
            StoredKind::Extract => self.has_extract(&record.key),
        }
    }
}

fn stage_artifact_file(workspace: &Workspace, source: &Path, relative_path: PathBuf) -> Result<()> {
    workspace.stage_file_by_size(
        source,
        &relative_path,
        DEFAULT_COPY_ONLY_THRESHOLD_BYTES,
        HardlinkOrCopyOptions::new().fallback(FallBack::Copy),
    )?;
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoreKey {
    Digest(ValidDigest),
    NamedVersion {
        id: ResourceId,
        version: ResolvedVersion,
    },
    Logical(String),
}

impl StoreKey {
    pub fn logical(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(StoreError::EmptyLogicalKey);
        }
        Ok(Self::Logical(value))
    }

    pub fn relative_name(&self) -> String {
        match self {
            Self::Digest(digest) => format!(
                "digest-{}-{}",
                algorithm_name(&digest.algorithm),
                digest.hex()
            ),
            Self::NamedVersion { id, version } => {
                format!(
                    "named-{}-{}",
                    sanitize(&id.as_string()),
                    sanitize(version.as_str())
                )
            }
            Self::Logical(value) => format!("logical-{}", sanitize(value)),
        }
    }
}

pub trait KeyDerivation {
    fn derive(&self, resource: &ResolvedResource) -> Option<StoreKey>;
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreProvenance {
    pub origin: Option<String>,
    pub metadata: Metadata,
}

impl StoreProvenance {
    pub fn from_fetch_receipt(receipt: &FetchReceipt) -> Self {
        let origin = match &receipt.source {
            FetchSource::Url(url) => Some(url.clone()),
            FetchSource::LocalPath(path) => Some(path.to_string_lossy().into_owned()),
        };

        let metadata = Self::fetch_metadata(receipt);

        Self { origin, metadata }
    }

    pub fn from_archive_report(report: &ArchiveReport) -> Self {
        Self {
            origin: None,
            metadata: Self::archive_metadata(report),
        }
    }

    pub fn from_fetched_archive_extraction(receipt: &FetchReceipt, report: &ArchiveReport) -> Self {
        let mut metadata = Metadata::new();
        metadata.extend(Self::fetch_metadata(receipt));
        metadata.extend(Self::archive_metadata(report));

        Self {
            origin: Self::from_fetch_receipt(receipt).origin,
            metadata,
        }
    }

    fn fetch_metadata(receipt: &FetchReceipt) -> Metadata {
        let mut metadata = Metadata::new();
        if let Some(sha256_hex) = &receipt.sha256_hex {
            metadata.insert("fetch.sha256".to_string(), sha256_hex.clone());
        }
        metadata
    }

    fn archive_metadata(report: &ArchiveReport) -> Metadata {
        Metadata::from([
            ("archive.format".to_string(), format!("{:?}", report.format)),
            (
                "archive.entry_count".to_string(),
                report.entry_count.to_string(),
            ),
            (
                "archive.total_bytes".to_string(),
                report.total_bytes.to_string(),
            ),
        ])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoredKind {
    Artifact,
    Extract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreMetadataRecord {
    #[serde(default = "default_store_metadata_schema_version")]
    pub schema_version: u32,
    pub key: StoreKey,
    pub kind: StoredKind,
    pub provenance: Option<StoreProvenance>,
    pub updated_at_unix: u64,
}

impl StoreMetadataRecord {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != STORE_METADATA_SCHEMA_VERSION {
            return Err(StoreError::UnsupportedMetadataSchemaVersion {
                expected: STORE_METADATA_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }
        Ok(())
    }
}

fn default_store_metadata_schema_version() -> u32 {
    STORE_METADATA_SCHEMA_VERSION
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PruneReport {
    pub removed_metadata: usize,
    pub protected_metadata: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct MetadataPrunePlan {
    pub removable: Vec<StoreMetadataRecord>,
    pub protected: Vec<StoreMetadataRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredArtifact {
    pub key: StoreKey,
    pub path: PathBuf,
    pub provenance: Option<StoreProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedArtifact {
    pub key: StoreKey,
    pub path: PathBuf,
    pub provenance: Option<StoreProvenance>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct StoredEntry {
    key: StoreKey,
    path: PathBuf,
    provenance: Option<StoreProvenance>,
}

fn algorithm_name(algorithm: &pulith_resource::DigestAlgorithm) -> String {
    match algorithm {
        pulith_resource::DigestAlgorithm::Sha256 => "sha256".to_string(),
        pulith_resource::DigestAlgorithm::Blake3 => "blake3".to_string(),
        pulith_resource::DigestAlgorithm::Custom(value) => sanitize(value),
    }
}

fn sanitize(value: &str) -> String {
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

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulith_archive::{ArchiveFormat, ArchiveReport};
    use pulith_fetch::{FetchReceipt, FetchSource};
    use pulith_resource::{
        RequestedResource, ResolvedLocator, ResourceLocator, ResourceSpec, ValidUrl,
    };
    use pulith_serde_backend::CompactJsonTextCodec;

    #[test]
    fn store_provenance_from_fetch_receipt_translates_source_and_digest() {
        let receipt = FetchReceipt {
            source: FetchSource::Url("https://example.com/runtime.zip".to_string()),
            destination: PathBuf::from("/tmp/runtime.zip"),
            bytes_downloaded: 12,
            total_bytes: Some(12),
            sha256_hex: Some("abc123".to_string()),
        };

        let provenance = StoreProvenance::from_fetch_receipt(&receipt);
        assert_eq!(
            provenance.origin.as_deref(),
            Some("https://example.com/runtime.zip")
        );
        assert_eq!(
            provenance.metadata.get("fetch.sha256").map(String::as_str),
            Some("abc123")
        );
    }

    #[test]
    fn store_provenance_from_archive_report_populates_archive_metadata() {
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 2,
            total_bytes: 42,
            entries: vec![],
        };

        let provenance = StoreProvenance::from_archive_report(&report);
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
            Some("2")
        );
        assert_eq!(
            provenance
                .metadata
                .get("archive.total_bytes")
                .map(String::as_str),
            Some("42")
        );
    }

    #[test]
    fn store_provenance_from_fetched_archive_extraction_merges_fetch_and_archive() {
        let receipt = FetchReceipt {
            source: FetchSource::Url("https://example.com/runtime.zip".to_string()),
            destination: PathBuf::from("/tmp/runtime.zip"),
            bytes_downloaded: 12,
            total_bytes: Some(12),
            sha256_hex: Some("abc123".to_string()),
        };
        let report = ArchiveReport {
            format: ArchiveFormat::Zip,
            entry_count: 2,
            total_bytes: 42,
            entries: vec![],
        };

        let provenance = StoreProvenance::from_fetched_archive_extraction(&receipt, &report);
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

    #[test]
    fn store_initializes_and_writes_artifact() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let key = StoreKey::logical("node-lts").unwrap();
        let artifact = store.put_artifact_bytes(&key, b"hello").unwrap();
        assert!(artifact.path.exists());
        assert!(store.get_artifact(&key).is_some());
    }

    #[test]
    fn named_version_key_uses_resource_identity() {
        let key = StoreKey::NamedVersion {
            id: ResourceId::parse("nodejs.org/node").unwrap(),
            version: ResolvedVersion::new("20.12.1").unwrap(),
        };
        assert!(key.relative_name().contains("nodejs.org-node"));
    }

    #[test]
    fn trait_can_derive_key_from_resolved_resource() {
        struct ByVersion;
        impl KeyDerivation for ByVersion {
            fn derive(&self, resource: &ResolvedResource) -> Option<StoreKey> {
                Some(StoreKey::NamedVersion {
                    id: resource.spec().id.clone(),
                    version: resource.version().clone(),
                })
            }
        }

        let requested = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.tar.gz").unwrap()),
        ));
        let resolved = requested.resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(
                ValidUrl::parse("https://mirror.example.com/runtime.tar.gz").unwrap(),
            ),
            None,
        );

        assert!(ByVersion.derive(&resolved).is_some());
    }

    #[test]
    fn store_can_lookup_artifact_for_resource_via_key_derivation() {
        struct ByVersion;
        impl KeyDerivation for ByVersion {
            fn derive(&self, resource: &ResolvedResource) -> Option<StoreKey> {
                Some(StoreKey::NamedVersion {
                    id: resource.spec().id.clone(),
                    version: resource.version().clone(),
                })
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();
        let resolved = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.tar.gz").unwrap()),
        ))
        .resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(
                ValidUrl::parse("https://mirror.example.com/runtime.tar.gz").unwrap(),
            ),
            None,
        );
        let key = ByVersion.derive(&resolved).unwrap();

        store.put_artifact_bytes(&key, b"hello").unwrap();

        let artifact = store.get_artifact_for(&resolved, &ByVersion).unwrap();
        assert!(artifact.path.exists());
        assert_eq!(artifact.key, key);
    }

    #[test]
    fn store_can_lookup_extract_metadata_for_resource_via_key_derivation() {
        struct ByVersion;
        impl KeyDerivation for ByVersion {
            fn derive(&self, resource: &ResolvedResource) -> Option<StoreKey> {
                Some(StoreKey::NamedVersion {
                    id: resource.spec().id.clone(),
                    version: resource.version().clone(),
                })
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();
        let resolved = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.tar.gz").unwrap()),
        ))
        .resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(
                ValidUrl::parse("https://mirror.example.com/runtime.tar.gz").unwrap(),
            ),
            None,
        );
        let key = ByVersion.derive(&resolved).unwrap();
        let extract_root = temp.path().join("extract-root");
        std::fs::create_dir_all(&extract_root).unwrap();
        std::fs::write(extract_root.join("tool.exe"), b"hello").unwrap();

        store
            .register_extract_dir_with_provenance(
                &key,
                &extract_root,
                Some(StoreProvenance {
                    origin: Some("integration-test".to_string()),
                    metadata: Metadata::from([("archive.format".to_string(), "Zip".to_string())]),
                }),
            )
            .unwrap();

        let extract = store.get_extract_for(&resolved, &ByVersion).unwrap();
        assert_eq!(extract.key, key);

        let metadata = store
            .get_metadata_for(&resolved, &ByVersion)
            .unwrap()
            .unwrap();
        assert_eq!(metadata.kind, StoredKind::Extract);
        assert_eq!(
            metadata.provenance.unwrap().origin.as_deref(),
            Some("integration-test")
        );
    }

    #[test]
    fn store_persists_and_reads_provenance() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let source = temp.path().join("source.bin");
        std::fs::write(&source, b"hello").unwrap();
        let key = StoreKey::logical("runtime").unwrap();
        let artifact = store
            .import_artifact_with_provenance(
                &key,
                &source,
                Some(StoreProvenance {
                    origin: Some("integration-test".to_string()),
                    metadata: Metadata::new(),
                }),
            )
            .unwrap();

        assert_eq!(
            artifact.provenance.as_ref().unwrap().origin.as_deref(),
            Some("integration-test")
        );
        let looked_up = store.get_artifact(&key).unwrap();
        assert_eq!(
            looked_up.provenance.as_ref().unwrap().origin.as_deref(),
            Some("integration-test")
        );
    }

    #[test]
    fn register_artifact_absorbs_path_and_provenance_tuple() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let source = temp.path().join("source.bin");
        std::fs::write(&source, b"hello").unwrap();
        let key = StoreKey::logical("runtime-register").unwrap();
        let artifact = store
            .register_artifact(
                &key,
                (
                    source.as_path(),
                    StoreProvenance {
                        origin: Some("fetch".to_string()),
                        metadata: Metadata::from([("fetch.sha256".to_string(), "abc".to_string())]),
                    },
                ),
            )
            .unwrap();

        assert!(artifact.path.exists());
        assert_eq!(
            artifact.provenance.unwrap().origin.as_deref(),
            Some("fetch")
        );
    }

    #[test]
    fn register_extract_absorbs_path_and_provenance_tuple() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let extract_root = temp.path().join("extract-root");
        std::fs::create_dir_all(extract_root.join("bin")).unwrap();
        std::fs::write(extract_root.join("bin/tool"), b"hello").unwrap();
        let key = StoreKey::logical("runtime-extract-register").unwrap();
        let extract = store
            .register_extract(
                &key,
                (
                    extract_root.as_path(),
                    StoreProvenance {
                        origin: Some("archive".to_string()),
                        metadata: Metadata::from([(
                            "archive.format".to_string(),
                            "tar.gz".to_string(),
                        )]),
                    },
                ),
            )
            .unwrap();

        assert!(extract.path.join("bin/tool").exists());
        assert_eq!(
            extract.provenance.unwrap().origin.as_deref(),
            Some("archive")
        );
    }

    #[test]
    fn prune_missing_removes_orphaned_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let key = StoreKey::logical("orphan").unwrap();
        store.put_artifact_bytes(&key, b"hello").unwrap();
        std::fs::remove_file(store.artifact_path(&key)).unwrap();

        let report = store.prune_missing().unwrap();
        assert_eq!(report.removed_metadata, 1);
        assert!(store.list_metadata().unwrap().is_empty());
    }

    #[test]
    fn store_can_list_orphaned_metadata_before_pruning() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let artifact_key = StoreKey::logical("artifact-orphan").unwrap();
        store.put_artifact_bytes(&artifact_key, b"hello").unwrap();
        std::fs::remove_file(store.artifact_path(&artifact_key)).unwrap();

        let extract_key = StoreKey::logical("extract-orphan").unwrap();
        let extract_root = temp.path().join("extract-root");
        std::fs::create_dir_all(&extract_root).unwrap();
        std::fs::write(extract_root.join("tool.exe"), b"hello").unwrap();
        store
            .register_extract_dir(&extract_key, &extract_root)
            .unwrap();
        std::fs::remove_dir_all(store.extract_path(&extract_key)).unwrap();

        let orphans = store.list_orphaned_metadata().unwrap();
        assert_eq!(orphans.len(), 2);
        assert!(orphans.iter().any(|record| record.key == artifact_key));
        assert!(orphans.iter().any(|record| record.key == extract_key));
    }

    #[test]
    fn store_can_lookup_orphaned_metadata_for_resource_via_key_derivation() {
        struct ByVersion;
        impl KeyDerivation for ByVersion {
            fn derive(&self, resource: &ResolvedResource) -> Option<StoreKey> {
                Some(StoreKey::NamedVersion {
                    id: resource.spec().id.clone(),
                    version: resource.version().clone(),
                })
            }
        }

        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();
        let resolved = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.tar.gz").unwrap()),
        ))
        .resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(
                ValidUrl::parse("https://mirror.example.com/runtime.tar.gz").unwrap(),
            ),
            None,
        );
        let key = ByVersion.derive(&resolved).unwrap();

        store.put_artifact_bytes(&key, b"hello").unwrap();
        std::fs::remove_file(store.artifact_path(&key)).unwrap();

        let orphan = store
            .get_orphaned_metadata_for(&resolved, &ByVersion)
            .unwrap()
            .unwrap();
        assert_eq!(orphan.key, key);
        assert_eq!(orphan.kind, StoredKind::Artifact);
    }

    #[test]
    fn store_can_plan_protected_metadata_prune() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let protected_key = StoreKey::logical("protected-orphan").unwrap();
        store.put_artifact_bytes(&protected_key, b"hello").unwrap();
        std::fs::remove_file(store.artifact_path(&protected_key)).unwrap();

        let removable_key = StoreKey::logical("removable-orphan").unwrap();
        store.put_artifact_bytes(&removable_key, b"hello").unwrap();
        std::fs::remove_file(store.artifact_path(&removable_key)).unwrap();

        let plan = store
            .plan_metadata_prune(std::slice::from_ref(&protected_key))
            .unwrap();
        assert_eq!(plan.protected.len(), 1);
        assert_eq!(plan.protected[0].key, protected_key);
        assert_eq!(plan.removable.len(), 1);
        assert_eq!(plan.removable[0].key, removable_key);
    }

    #[test]
    fn store_prune_can_skip_protected_metadata() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let protected_key = StoreKey::logical("protected-orphan").unwrap();
        store.put_artifact_bytes(&protected_key, b"hello").unwrap();
        std::fs::remove_file(store.artifact_path(&protected_key)).unwrap();

        let report = store
            .prune_missing_with_protection(std::slice::from_ref(&protected_key))
            .unwrap();
        assert_eq!(report.removed_metadata, 0);
        assert_eq!(report.protected_metadata, 1);
        assert!(store.has_metadata(&protected_key));
    }

    #[test]
    fn store_rejects_unsupported_metadata_schema_version() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let key = StoreKey::logical("invalid-schema").unwrap();
        let path = store.metadata_path(&key);
        let invalid = StoreMetadataRecord {
            schema_version: STORE_METADATA_SCHEMA_VERSION + 1,
            key,
            kind: StoredKind::Artifact,
            provenance: None,
            updated_at_unix: 0,
        };
        let bytes = encode_pretty_vec(&JsonTextCodec, &invalid).unwrap();
        atomic_write(path, &bytes, Default::default()).unwrap();

        assert!(matches!(
            store.list_metadata(),
            Err(StoreError::UnsupportedMetadataSchemaVersion {
                expected,
                actual
            }) if expected == STORE_METADATA_SCHEMA_VERSION && actual == STORE_METADATA_SCHEMA_VERSION + 1
        ));
    }

    #[test]
    fn store_list_metadata_accepts_compact_json_payload() {
        let temp = tempfile::tempdir().unwrap();
        let store = StoreReady::initialize(StoreRoots::new(
            temp.path().join("artifacts"),
            temp.path().join("extracts"),
            temp.path().join("metadata"),
        ))
        .unwrap();

        let key = StoreKey::logical("compact-json").unwrap();
        let path = store.metadata_path(&key);
        let record = StoreMetadataRecord {
            schema_version: STORE_METADATA_SCHEMA_VERSION,
            key: key.clone(),
            kind: StoredKind::Artifact,
            provenance: None,
            updated_at_unix: 1,
        };
        let bytes = encode_pretty_vec(&CompactJsonTextCodec, &record).unwrap();
        atomic_write(path, &bytes, Default::default()).unwrap();

        let listed = store.list_metadata().unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].key, key);
    }
}
