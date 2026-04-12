//! Deterministic lock file model and diffing primitives.

use std::collections::BTreeMap;

use pulith_serde_backend::{CodecError, JsonTextCodec, TextCodec};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Metadata = BTreeMap<String, String>;

pub const LOCK_SCHEMA_VERSION: u32 = 1;

pub type Result<T> = std::result::Result<T, LockError>;

#[derive(Debug, Error)]
pub enum LockError {
    #[error("serialization backend error: {0}")]
    Codec(#[from] CodecError),
    #[error("unsupported lock schema version: expected {expected}, got {actual}")]
    UnsupportedSchemaVersion { expected: u32, actual: u32 },
    #[error("resource key must not be empty")]
    EmptyResourceKey,
    #[error("resource version must not be empty")]
    EmptyVersion,
    #[error("resource source must not be empty")]
    EmptySource,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockedResource {
    pub version: String,
    pub source: String,
    pub digest: Option<String>,
    pub metadata: Metadata,
}

impl LockedResource {
    pub fn new(version: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            version: version.into(),
            source: source.into(),
            digest: None,
            metadata: Metadata::new(),
        }
    }

    pub fn digest(mut self, digest: impl Into<String>) -> Self {
        self.digest = Some(digest.into());
        self
    }

    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockFile {
    pub schema_version: u32,
    pub resources: BTreeMap<String, LockedResource>,
    pub metadata: Metadata,
}

impl Default for LockFile {
    fn default() -> Self {
        Self {
            schema_version: LOCK_SCHEMA_VERSION,
            resources: BTreeMap::new(),
            metadata: Metadata::new(),
        }
    }
}

impl LockFile {
    pub fn upsert(&mut self, resource: impl Into<String>, locked: LockedResource) {
        self.resources.insert(resource.into(), locked);
    }

    pub fn to_json(&self) -> Result<String> {
        self.to_text_with(&JsonTextCodec)
    }

    pub fn from_json(data: &str) -> Result<Self> {
        Self::from_text_with(&JsonTextCodec, data)
    }

    pub fn to_text_with<C: TextCodec>(&self, codec: &C) -> Result<String> {
        Ok(codec.encode_pretty(self)?)
    }

    pub fn from_text_with<C: TextCodec>(codec: &C, data: &str) -> Result<Self> {
        Ok(codec.decode_str(data)?)
    }

    pub fn from_json_validated(data: &str) -> Result<Self> {
        let lock = Self::from_json(data)?;
        lock.validate()?;
        Ok(lock)
    }

    pub fn validate(&self) -> Result<()> {
        if self.schema_version != LOCK_SCHEMA_VERSION {
            return Err(LockError::UnsupportedSchemaVersion {
                expected: LOCK_SCHEMA_VERSION,
                actual: self.schema_version,
            });
        }

        for (resource, locked) in &self.resources {
            if resource.is_empty() {
                return Err(LockError::EmptyResourceKey);
            }
            if locked.version.is_empty() {
                return Err(LockError::EmptyVersion);
            }
            if locked.source.is_empty() {
                return Err(LockError::EmptySource);
            }
        }

        Ok(())
    }

    pub fn diff(&self, target: &Self) -> LockDiff {
        let mut added = Vec::with_capacity(target.resources.len());
        let mut removed = Vec::with_capacity(self.resources.len());
        let mut changed = Vec::new();

        for (resource, from_locked) in &self.resources {
            match target.resources.get(resource) {
                Some(to_locked) if to_locked != from_locked => changed.push(LockResourceChange {
                    resource: resource.clone(),
                    before: from_locked.clone(),
                    after: to_locked.clone(),
                }),
                Some(_) => {}
                None => removed.push((resource.clone(), from_locked.clone())),
            }
        }

        for (resource, to_locked) in &target.resources {
            if !self.resources.contains_key(resource) {
                added.push((resource.clone(), to_locked.clone()));
            }
        }

        added.shrink_to_fit();
        removed.shrink_to_fit();

        LockDiff {
            added,
            removed,
            changed,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockResourceChange {
    pub resource: String,
    pub before: LockedResource,
    pub after: LockedResource,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LockDiff {
    pub added: Vec<(String, LockedResource)>,
    pub removed: Vec<(String, LockedResource)>,
    pub changed: Vec<LockResourceChange>,
}

impl LockDiff {
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.changed.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulith_serde_backend::CompactJsonTextCodec;

    #[test]
    fn lock_json_is_deterministic_by_resource_key_order() {
        let mut lock = LockFile::default();
        lock.upsert(
            "zeta/tool",
            LockedResource::new("1.0.0", "https://example.com/zeta"),
        );
        lock.upsert(
            "alpha/tool",
            LockedResource::new("1.0.0", "https://example.com/alpha"),
        );

        let json = lock.to_json().unwrap();
        let alpha = json.find("alpha/tool").unwrap();
        let zeta = json.find("zeta/tool").unwrap();

        assert!(alpha < zeta);
    }

    #[test]
    fn lock_round_trip_preserves_content() {
        let mut lock = LockFile::default();
        lock.upsert(
            "example/runtime",
            LockedResource::new("20.12.1", "https://example.com/runtime.tar.zst")
                .digest("sha256:abc"),
        );

        let json = lock.to_json().unwrap();
        let parsed = LockFile::from_json(&json).unwrap();
        parsed.validate().unwrap();

        assert_eq!(parsed, lock);
    }

    #[test]
    fn lock_diff_reports_added_removed_and_changed_entries() {
        let mut base = LockFile::default();
        base.upsert(
            "example/a",
            LockedResource::new("1.0.0", "https://example.com/a"),
        );
        base.upsert(
            "example/b",
            LockedResource::new("1.0.0", "https://example.com/b"),
        );

        let mut next = LockFile::default();
        next.upsert(
            "example/b",
            LockedResource::new("2.0.0", "https://example.com/b"),
        );
        next.upsert(
            "example/c",
            LockedResource::new("1.0.0", "https://example.com/c"),
        );

        let diff = base.diff(&next);
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 1);
        assert_eq!(diff.changed.len(), 1);
        assert_eq!(diff.added[0].0, "example/c");
        assert_eq!(diff.removed[0].0, "example/a");
        assert_eq!(diff.changed[0].resource, "example/b");
        assert_eq!(diff.changed[0].before.version, "1.0.0");
        assert_eq!(diff.changed[0].after.version, "2.0.0");
    }

    #[test]
    fn lock_diff_is_empty_for_identical_files() {
        let mut lock = LockFile::default();
        lock.upsert(
            "example/runtime",
            LockedResource::new("1.0.0", "https://example.com/runtime"),
        );

        let diff = lock.diff(&lock);
        assert!(diff.is_empty());
    }

    #[test]
    fn lock_validate_rejects_wrong_schema() {
        let lock = LockFile {
            schema_version: 2,
            ..LockFile::default()
        };
        assert!(matches!(
            lock.validate(),
            Err(LockError::UnsupportedSchemaVersion {
                expected,
                actual
            }) if expected == LOCK_SCHEMA_VERSION && actual == 2
        ));
    }

    #[test]
    fn lock_validate_rejects_empty_fields() {
        let mut lock = LockFile::default();
        lock.resources.insert(
            String::new(),
            LockedResource::new("1.0.0", "https://example.com"),
        );

        assert!(matches!(lock.validate(), Err(LockError::EmptyResourceKey)));
    }

    #[test]
    fn lock_codec_roundtrip_preserves_semantic_parity() {
        let mut lock = LockFile::default();
        lock.upsert(
            "example/runtime",
            LockedResource::new("1.0.0", "https://example.com/runtime").digest("sha256:abc"),
        );

        let pretty = lock.to_json().unwrap();
        let compact = lock.to_text_with(&CompactJsonTextCodec).unwrap();

        let pretty_decoded = LockFile::from_json(&pretty).unwrap();
        let compact_decoded = LockFile::from_text_with(&CompactJsonTextCodec, &compact).unwrap();
        let cross_decoded = LockFile::from_json(&compact).unwrap();

        assert_eq!(pretty_decoded, compact_decoded);
        assert_eq!(pretty_decoded, cross_decoded);
    }
}
