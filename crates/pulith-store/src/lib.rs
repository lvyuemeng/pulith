//! Composable local artifact storage for Pulith.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use pulith_fs::{FallBack, HardlinkOrCopyOptions, Workspace, atomic_write, copy_dir_all};
use pulith_resource::{Metadata, ResolvedResource, ResolvedVersion, ResourceId, ValidDigest};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, StoreError>;

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
        let path = self.artifact_path(key);
        if !path.exists() {
            return None;
        }
        let provenance = self.load_provenance(key).ok().flatten();
        Some(StoredArtifact {
            key: key.clone(),
            path,
            provenance,
        })
    }

    pub fn get_extract(&self, key: &StoreKey) -> Option<ExtractedArtifact> {
        let path = self.extract_path(key);
        if !path.exists() {
            return None;
        }
        let provenance = self.load_provenance(key).ok().flatten();
        Some(ExtractedArtifact {
            key: key.clone(),
            path,
            provenance,
        })
    }

    pub fn list_metadata(&self) -> Result<Vec<StoreMetadataRecord>> {
        let mut records = Vec::new();
        for entry in std::fs::read_dir(&self.roots.metadata)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let content = std::fs::read_to_string(entry.path())?;
            let record: StoreMetadataRecord = serde_json::from_str(&content).map_err(|error| {
                StoreError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
            })?;
            records.push(record);
        }
        Ok(records)
    }

    pub fn prune_missing(&self) -> Result<PruneReport> {
        let mut report = PruneReport::default();
        for record in self.list_metadata()? {
            let key = &record.key;
            let keep = match record.kind {
                StoredKind::Artifact => self.artifact_path(key).exists(),
                StoredKind::Extract => self.extract_path(key).exists(),
            };
            if !keep {
                let metadata_path = self.metadata_path(key);
                if metadata_path.exists() {
                    std::fs::remove_file(&metadata_path)?;
                    report.removed_metadata += 1;
                }
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
        let workspace_root = tempfile::tempdir()?;
        let workspace = Workspace::new(
            workspace_root.path().join("artifact"),
            self.roots.artifacts.clone(),
        )?;
        workspace.link_or_copy_file(
            source,
            PathBuf::from(key.relative_name()).join(file_name),
            HardlinkOrCopyOptions::new().fallback(FallBack::Copy),
        )?;
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

    fn persist_provenance(
        &self,
        key: &StoreKey,
        kind: StoredKind,
        provenance: Option<&StoreProvenance>,
    ) -> Result<()> {
        let record = StoreMetadataRecord {
            key: key.clone(),
            kind,
            provenance: provenance.cloned(),
            updated_at_unix: now_unix(),
        };
        let bytes = serde_json::to_vec_pretty(&record).map_err(|error| {
            StoreError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        atomic_write(self.metadata_path(key), &bytes, Default::default())?;
        Ok(())
    }

    fn load_provenance(&self, key: &StoreKey) -> Result<Option<StoreProvenance>> {
        let path = self.metadata_path(key);
        if !path.exists() {
            return Ok(None);
        }
        let content = std::fs::read_to_string(path)?;
        let record: StoreMetadataRecord = serde_json::from_str(&content).map_err(|error| {
            StoreError::Io(std::io::Error::new(std::io::ErrorKind::InvalidData, error))
        })?;
        Ok(record.provenance)
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StoredKind {
    Artifact,
    Extract,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoreMetadataRecord {
    pub key: StoreKey,
    pub kind: StoredKind,
    pub provenance: Option<StoreProvenance>,
    pub updated_at_unix: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct PruneReport {
    pub removed_metadata: usize,
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
    use pulith_resource::{
        RequestedResource, ResolvedLocator, ResourceLocator, ResourceSpec, ValidUrl,
    };

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
}
