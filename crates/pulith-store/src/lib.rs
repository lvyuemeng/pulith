//! Composable local artifact storage for Pulith.

use std::path::{Path, PathBuf};

use pulith_fs::{Workspace, atomic_write, copy_dir_all};
use pulith_resource::{ResolvedResource, ResolvedVersion, ResourceId, ValidDigest};
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

    pub fn put_artifact_bytes(&self, key: &StoreKey, bytes: &[u8]) -> Result<StoredArtifact> {
        let path = self.artifact_path(key);
        atomic_write(&path, bytes, Default::default())?;
        Ok(StoredArtifact {
            key: key.clone(),
            path,
        })
    }

    pub fn import_artifact(
        &self,
        key: &StoreKey,
        source: impl AsRef<Path>,
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
        workspace.copy_file(source, PathBuf::from(key.relative_name()).join(file_name))?;
        workspace.commit()?;

        Ok(StoredArtifact {
            key: key.clone(),
            path: self.artifact_path(key).join(file_name),
        })
    }

    pub fn register_extract_dir(
        &self,
        key: &StoreKey,
        source_dir: impl AsRef<Path>,
    ) -> Result<ExtractedArtifact> {
        let source_dir = source_dir.as_ref();
        let target = self.extract_path(key);

        if target.exists() {
            std::fs::remove_dir_all(&target)?;
        }

        copy_dir_all(source_dir, &target)?;
        Ok(ExtractedArtifact {
            key: key.clone(),
            path: target,
        })
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
pub struct StoredArtifact {
    pub key: StoreKey,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtractedArtifact {
    pub key: StoreKey,
    pub path: PathBuf,
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
}
