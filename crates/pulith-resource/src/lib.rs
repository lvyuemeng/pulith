//! Composable resource description types for Pulith.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pulith_version::{VersionKind, VersionRequirement};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use url::Url;

pub type Labels = BTreeMap<String, String>;
pub type Metadata = BTreeMap<String, String>;

pub type Result<T> = std::result::Result<T, ResourceError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ResourceError {
    #[error("resource authority must not be empty")]
    EmptyAuthority,
    #[error("resource name must not be empty")]
    EmptyName,
    #[error("invalid resource segment `{0}`")]
    InvalidSegment(String),
    #[error("invalid URL: {0}")]
    InvalidUrl(String),
    #[error("digest hex is invalid: {0}")]
    InvalidDigestHex(String),
    #[error("digest length for {algorithm:?} must be {expected} bytes, got {actual}")]
    InvalidDigestLength {
        algorithm: DigestAlgorithm,
        expected: usize,
        actual: usize,
    },
    #[error("value must not be empty")]
    EmptyValue,
    #[error("alternatives must not be empty")]
    EmptyAlternatives,
    #[error("trust anchor host must not be empty")]
    EmptyTrustHost,
    #[error("trust metadata key must not be empty")]
    EmptyTrustMetadataKey,
    #[error("resolved version is not parseable for selector matching: {0}")]
    InvalidResolvedVersion(String),
    #[error("resolved version `{version}` does not satisfy selector `{selector}`")]
    ResolvedVersionMismatch { selector: String, version: String },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceId {
    pub authority: Option<String>,
    pub name: String,
}

impl ResourceId {
    pub fn new(authority: Option<impl Into<String>>, name: impl Into<String>) -> Result<Self> {
        let authority = authority.map(Into::into);
        let name = name.into();

        if let Some(authority) = &authority {
            if authority.is_empty() {
                return Err(ResourceError::EmptyAuthority);
            }
            validate_segments(authority)?;
        }

        if name.is_empty() {
            return Err(ResourceError::EmptyName);
        }
        validate_segments(&name)?;

        Ok(Self { authority, name })
    }

    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let value = value.as_ref();
        if let Some((authority, name)) = value.rsplit_once('/') {
            Self::new(Some(authority.to_string()), name.to_string())
        } else {
            Self::new(None::<String>, value.to_string())
        }
    }

    pub fn as_string(&self) -> String {
        match &self.authority {
            Some(authority) => format!("{authority}/{}", self.name),
            None => self.name.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ValidUrl(Url);

impl ValidUrl {
    pub fn parse(value: impl AsRef<str>) -> Result<Self> {
        let parsed =
            Url::parse(value.as_ref()).map_err(|err| ResourceError::InvalidUrl(err.to_string()))?;
        Ok(Self(parsed))
    }

    pub fn as_url(&self) -> &Url {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionSelector {
    Exact(VersionKind),
    Alias(String),
    Requirement(VersionRequirement),
    Unspecified,
}

impl VersionSelector {
    pub fn exact(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure_non_empty(&value)?;
        Ok(Self::Exact(
            VersionKind::parse(&value).map_err(|_| ResourceError::EmptyValue)?,
        ))
    }

    pub fn alias(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure_non_empty(&value)?;
        Ok(Self::Alias(value))
    }

    pub fn requirement(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure_non_empty(&value)?;
        Ok(Self::Requirement(
            VersionRequirement::parse(&value).map_err(|_| ResourceError::EmptyValue)?,
        ))
    }

    pub fn matches_resolved_version(&self, version: &ResolvedVersion) -> Result<bool> {
        match self {
            Self::Exact(expected) => Ok(expected == &parse_resolved_version(version)?),
            Self::Requirement(requirement) => {
                Ok(requirement.matches(&parse_resolved_version(version)?))
            }
            Self::Alias(_) | Self::Unspecified => Ok(true),
        }
    }

    pub fn as_label(&self) -> String {
        match self {
            Self::Exact(version) => version.to_string(),
            Self::Alias(alias) => alias.clone(),
            Self::Requirement(requirement) => format!("{requirement:?}"),
            Self::Unspecified => "*".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedVersion(String);

impl ResolvedVersion {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure_non_empty(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResourceLocator {
    Url(ValidUrl),
    Alternatives(Vec<ValidUrl>),
    LocalPath(PathBuf),
}

impl ResourceLocator {
    pub fn alternatives(urls: Vec<ValidUrl>) -> Result<Self> {
        if urls.is_empty() {
            return Err(ResourceError::EmptyAlternatives);
        }
        Ok(Self::Alternatives(urls))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolvedLocator {
    Url(ValidUrl),
    LocalPath(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DigestAlgorithm {
    Sha256,
    Blake3,
    Custom(String),
}

impl DigestAlgorithm {
    fn expected_length(&self) -> Option<usize> {
        match self {
            Self::Sha256 | Self::Blake3 => Some(32),
            Self::Custom(_) => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidDigest {
    pub algorithm: DigestAlgorithm,
    pub bytes: Vec<u8>,
}

impl ValidDigest {
    pub fn from_bytes(algorithm: DigestAlgorithm, bytes: Vec<u8>) -> Result<Self> {
        if let Some(expected) = algorithm.expected_length()
            && bytes.len() != expected
        {
            return Err(ResourceError::InvalidDigestLength {
                algorithm,
                expected,
                actual: bytes.len(),
            });
        }

        Ok(Self { algorithm, bytes })
    }

    pub fn from_hex(algorithm: DigestAlgorithm, value: impl AsRef<str>) -> Result<Self> {
        let bytes = hex::decode(value.as_ref())
            .map_err(|err| ResourceError::InvalidDigestHex(err.to_string()))?;
        Self::from_bytes(algorithm, bytes)
    }

    pub fn hex(&self) -> String {
        hex::encode(&self.bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationRequirement {
    None,
    Digest(ValidDigest),
    AnyOf(Vec<ValidDigest>),
    AllOf(Vec<ValidDigest>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustMode {
    Open,
    RequireVerification,
    RequireAnchorMatch,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrustAnchor {
    Digest(ValidDigest),
    Host(String),
    Metadata { key: String, value: String },
}

impl TrustAnchor {
    pub fn host(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure_non_empty(&value).map_err(|_| ResourceError::EmptyTrustHost)?;
        Ok(Self::Host(value))
    }

    pub fn metadata(key: impl Into<String>, value: impl Into<String>) -> Result<Self> {
        let key = key.into();
        let value = value.into();
        ensure_non_empty(&key).map_err(|_| ResourceError::EmptyTrustMetadataKey)?;
        ensure_non_empty(&value)?;
        Ok(Self::Metadata { key, value })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrustPolicy {
    pub mode: TrustMode,
    pub anchors: Vec<TrustAnchor>,
}

impl Default for TrustPolicy {
    fn default() -> Self {
        Self {
            mode: TrustMode::Open,
            anchors: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrustDecision {
    Trusted,
    Untrusted(&'static str),
}

impl TrustPolicy {
    pub fn evaluate(
        &self,
        locator: Option<&ResolvedLocator>,
        artifact: Option<&ArtifactDescriptor>,
        metadata: &Metadata,
        verification: &VerificationRequirement,
    ) -> TrustDecision {
        match self.mode {
            TrustMode::Open => TrustDecision::Trusted,
            TrustMode::RequireVerification => match verification {
                VerificationRequirement::None => TrustDecision::Untrusted("verification required"),
                _ => TrustDecision::Trusted,
            },
            TrustMode::RequireAnchorMatch => {
                if self
                    .anchors
                    .iter()
                    .any(|anchor| anchor_matches(anchor, locator, artifact, metadata))
                {
                    TrustDecision::Trusted
                } else {
                    TrustDecision::Untrusted("no trust anchor matched")
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArtifactForm {
    File,
    Archive,
    DirectorySnapshot,
    Opaque,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnpackPolicy {
    None,
    Extract { strip_components: usize },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaterializationSpec {
    pub form: ArtifactForm,
    pub unpack: UnpackPolicy,
}

impl Default for MaterializationSpec {
    fn default() -> Self {
        Self {
            form: ArtifactForm::Opaque,
            unpack: UnpackPolicy::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArtifactDescriptor {
    pub digest: Option<ValidDigest>,
    pub file_name: Option<String>,
    pub metadata: Metadata,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceSpec {
    pub id: ResourceId,
    pub locator: ResourceLocator,
    pub version: VersionSelector,
    pub verification: VerificationRequirement,
    pub trust: TrustPolicy,
    pub materialization: MaterializationSpec,
    pub labels: Labels,
    pub metadata: Metadata,
}

impl ResourceSpec {
    pub fn new(id: ResourceId, locator: ResourceLocator) -> Self {
        Self {
            id,
            locator,
            version: VersionSelector::Unspecified,
            verification: VerificationRequirement::None,
            trust: TrustPolicy::default(),
            materialization: MaterializationSpec::default(),
            labels: Labels::new(),
            metadata: Metadata::new(),
        }
    }

    pub fn version(mut self, version: VersionSelector) -> Self {
        self.version = version;
        self
    }

    pub fn verification(mut self, verification: VerificationRequirement) -> Self {
        self.verification = verification;
        self
    }

    pub fn trust(mut self, trust: TrustPolicy) -> Self {
        self.trust = trust;
        self
    }

    pub fn materialization(mut self, materialization: MaterializationSpec) -> Self {
        self.materialization = materialization;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Requested;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resolved {
    pub version: ResolvedVersion,
    pub locator: ResolvedLocator,
    pub artifact: Option<ArtifactDescriptor>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource<S> {
    spec: ResourceSpec,
    state: S,
}

pub type RequestedResource = Resource<Requested>;
pub type ResolvedResource = Resource<Resolved>;

impl RequestedResource {
    pub fn new(spec: ResourceSpec) -> Self {
        Self {
            spec,
            state: Requested,
        }
    }

    pub fn resolve(
        self,
        version: ResolvedVersion,
        locator: ResolvedLocator,
        artifact: Option<ArtifactDescriptor>,
    ) -> ResolvedResource {
        ResolvedResource {
            spec: self.spec,
            state: Resolved {
                version,
                locator,
                artifact,
            },
        }
    }
}

impl<S> Resource<S> {
    pub fn spec(&self) -> &ResourceSpec {
        &self.spec
    }

    pub fn into_spec(self) -> ResourceSpec {
        self.spec
    }
}

impl ResolvedResource {
    pub fn resolved(&self) -> &Resolved {
        &self.state
    }

    pub fn version(&self) -> &ResolvedVersion {
        &self.state.version
    }

    pub fn locator(&self) -> &ResolvedLocator {
        &self.state.locator
    }

    pub fn trust_decision(&self) -> TrustDecision {
        self.spec.trust.evaluate(
            Some(&self.state.locator),
            self.state.artifact.as_ref(),
            &self.spec.metadata,
            &self.spec.verification,
        )
    }

    pub fn validate_version_selection(&self) -> Result<()> {
        if self
            .spec
            .version
            .matches_resolved_version(&self.state.version)?
        {
            Ok(())
        } else {
            Err(ResourceError::ResolvedVersionMismatch {
                selector: self.spec.version.as_label(),
                version: self.state.version.as_str().to_string(),
            })
        }
    }
}

fn parse_resolved_version(version: &ResolvedVersion) -> Result<VersionKind> {
    VersionKind::parse(version.as_str())
        .map_err(|_| ResourceError::InvalidResolvedVersion(version.as_str().to_string()))
}

fn anchor_matches(
    anchor: &TrustAnchor,
    locator: Option<&ResolvedLocator>,
    artifact: Option<&ArtifactDescriptor>,
    metadata: &Metadata,
) -> bool {
    match anchor {
        TrustAnchor::Digest(expected) => artifact
            .and_then(|artifact| artifact.digest.as_ref())
            .is_some_and(|digest| digest == expected),
        TrustAnchor::Host(host) => locator
            .and_then(|locator| match locator {
                ResolvedLocator::Url(url) => url.as_url().host_str(),
                ResolvedLocator::LocalPath(_) => None,
            })
            .is_some_and(|value| value == host),
        TrustAnchor::Metadata { key, value } => {
            metadata.get(key).is_some_and(|found| found == value)
        }
    }
}

fn ensure_non_empty(value: &str) -> Result<()> {
    if value.is_empty() {
        Err(ResourceError::EmptyValue)
    } else {
        Ok(())
    }
}

fn validate_segments(value: &str) -> Result<()> {
    for segment in value.split('/') {
        if segment.is_empty()
            || !segment
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-'))
        {
            return Err(ResourceError::InvalidSegment(segment.to_string()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resource_id_parses_authority_and_name() {
        let id = ResourceId::parse("github.com/neovim/nvim").unwrap();
        assert_eq!(id.authority.as_deref(), Some("github.com/neovim"));
        assert_eq!(id.name, "nvim");
    }

    #[test]
    fn url_and_digest_validation_work() {
        let url = ValidUrl::parse("https://example.com/tool.tar.gz").unwrap();
        assert_eq!(url.as_url().scheme(), "https");

        let digest = ValidDigest::from_hex(
            DigestAlgorithm::Sha256,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        )
        .unwrap();
        assert_eq!(digest.bytes.len(), 32);
    }

    #[test]
    fn requested_resource_can_resolve() {
        let spec = ResourceSpec::new(
            ResourceId::parse("nodejs.org/node").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/node.zip").unwrap()),
        )
        .version(VersionSelector::alias("lts").unwrap())
        .materialization(MaterializationSpec {
            form: ArtifactForm::Archive,
            unpack: UnpackPolicy::Extract {
                strip_components: 1,
            },
        });

        let requested = RequestedResource::new(spec);
        let resolved = requested.resolve(
            ResolvedVersion::new("20.12.1").unwrap(),
            ResolvedLocator::Url(ValidUrl::parse("https://mirror.example.com/node.zip").unwrap()),
            None,
        );

        assert_eq!(resolved.version().as_str(), "20.12.1");
        assert!(resolved.validate_version_selection().is_ok());
    }

    #[test]
    fn resolved_resource_rejects_requirement_mismatch() {
        let spec = ResourceSpec::new(
            ResourceId::parse("nodejs.org/node").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/node.zip").unwrap()),
        )
        .version(VersionSelector::requirement("^1.2").unwrap());

        let resolved = RequestedResource::new(spec).resolve(
            ResolvedVersion::new("2.0.0").unwrap(),
            ResolvedLocator::Url(ValidUrl::parse("https://mirror.example.com/node.zip").unwrap()),
            None,
        );

        assert!(matches!(
            resolved.validate_version_selection(),
            Err(ResourceError::ResolvedVersionMismatch { .. })
        ));
    }

    #[test]
    fn trust_policy_can_require_anchor_match() {
        let digest = ValidDigest::from_hex(
            DigestAlgorithm::Sha256,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824",
        )
        .unwrap();

        let spec = ResourceSpec::new(
            ResourceId::parse("nodejs.org/node").unwrap(),
            ResourceLocator::Url(
                ValidUrl::parse("https://downloads.example.com/node.zip").unwrap(),
            ),
        )
        .verification(VerificationRequirement::Digest(digest.clone()))
        .trust(TrustPolicy {
            mode: TrustMode::RequireAnchorMatch,
            anchors: vec![TrustAnchor::host("downloads.example.com").unwrap()],
        });

        let requested = RequestedResource::new(spec);
        let resolved = requested.resolve(
            ResolvedVersion::new("20.12.1").unwrap(),
            ResolvedLocator::Url(
                ValidUrl::parse("https://downloads.example.com/node.zip").unwrap(),
            ),
            Some(ArtifactDescriptor {
                digest: Some(digest),
                file_name: Some("node.zip".to_string()),
                metadata: Metadata::new(),
            }),
        );

        assert_eq!(resolved.trust_decision(), TrustDecision::Trusted);
    }
}
