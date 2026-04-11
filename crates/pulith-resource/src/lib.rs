//! Composable resource description types for Pulith.

use std::collections::BTreeMap;
use std::path::PathBuf;

use pulith_version::{
    SelectionPolicy, VersionKind, VersionPreference, VersionRequirement, select_preferred,
};
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
    #[error("version alias `{0}` is not recognized")]
    UnknownVersionAlias(String),
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
        Ok(Self::Exact(parse_non_empty_value(
            value,
            VersionKind::parse,
        )?))
    }

    pub fn alias(value: impl Into<String>) -> Result<Self> {
        Ok(Self::Alias(non_empty_string(value)?))
    }

    pub fn requirement(value: impl Into<String>) -> Result<Self> {
        Ok(Self::Requirement(parse_non_empty_value(
            value,
            VersionRequirement::parse,
        )?))
    }

    pub fn matches_resolved_version(&self, version: &ResolvedVersion) -> Result<bool> {
        let resolved = match self {
            Self::Exact(_) | Self::Requirement(_) => Some(parse_resolved_version(version)?),
            Self::Alias(_) | Self::Unspecified => None,
        };

        match self {
            Self::Exact(expected) => Ok(Some(expected) == resolved.as_ref()),
            Self::Requirement(requirement) => {
                Ok(resolved.is_some_and(|resolved| requirement.matches(&resolved)))
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

    pub fn selection_policy(&self) -> Result<SelectionPolicy> {
        match self {
            Self::Exact(version) => Ok(selection_policy(
                VersionRequirement::Exact(version.clone()),
                VersionPreference::Pinned(version.clone()),
            )),
            Self::Alias(alias) => alias_selection_policy(alias),
            Self::Requirement(requirement) => Ok(selection_policy(
                requirement.clone(),
                VersionPreference::HighestStable,
            )),
            Self::Unspecified => Ok(SelectionPolicy::default()),
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
        ensure_non_empty_collection(&urls, ResourceError::EmptyAlternatives)?;
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
pub enum ActivationModel {
    /// No activation step is expected after install.
    None,
    /// Activation writes or links to a path target.
    PathTarget,
    /// Activation resolves commands through shims.
    ShimResolution,
    /// Activation registers service manager state.
    ServiceRegistration,
    /// Activation projects environment configuration.
    EnvironmentProjection,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MutationScope {
    /// Core workflow mutates install-root paths only.
    InstallRootOnly,
    /// Core install-root mutation plus explicit caller extension steps.
    InstallRootWithExtensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProvenanceRequirement {
    /// Source continuity is required.
    SourceOnly,
    /// Source and verification continuity are both required.
    SourceAndVerification,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct LifecycleRequirements {
    /// Replace-in-place operations are expected.
    pub replace: bool,
    /// Rollback behavior is expected.
    pub rollback: bool,
    /// Uninstall behavior is expected.
    pub uninstall: bool,
    /// Repair behavior is expected.
    pub repair: bool,
}

impl LifecycleRequirements {
    /// Sets whether replace behavior is required.
    pub fn replace(mut self, enabled: bool) -> Self {
        self.replace = enabled;
        self
    }

    /// Sets whether rollback behavior is required.
    pub fn rollback(mut self, enabled: bool) -> Self {
        self.rollback = enabled;
        self
    }

    /// Sets whether uninstall behavior is required.
    pub fn uninstall(mut self, enabled: bool) -> Self {
        self.uninstall = enabled;
        self
    }

    /// Sets whether repair behavior is required.
    pub fn repair(mut self, enabled: bool) -> Self {
        self.repair = enabled;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBehaviorContract {
    /// Materialization behavior axis.
    pub materialization: MaterializationSpec,
    /// Activation behavior axis.
    pub activation: ActivationModel,
    /// Mutation scope behavior axis.
    pub mutation_scope: MutationScope,
    /// Provenance continuity behavior axis.
    pub provenance: ProvenanceRequirement,
    /// Lifecycle behavior axis.
    pub lifecycle: LifecycleRequirements,
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
    pub activation: ActivationModel,
    pub mutation_scope: MutationScope,
    pub provenance: ProvenanceRequirement,
    pub lifecycle: LifecycleRequirements,
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
            activation: ActivationModel::None,
            mutation_scope: MutationScope::InstallRootOnly,
            provenance: ProvenanceRequirement::SourceOnly,
            lifecycle: LifecycleRequirements::default(),
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

    pub fn activation_model(mut self, activation: ActivationModel) -> Self {
        self.activation = activation;
        self
    }

    pub fn mutation_scope(mut self, mutation_scope: MutationScope) -> Self {
        self.mutation_scope = mutation_scope;
        self
    }

    pub fn provenance_requirement(mut self, provenance: ProvenanceRequirement) -> Self {
        self.provenance = provenance;
        self
    }

    pub fn lifecycle_requirements(mut self, lifecycle: LifecycleRequirements) -> Self {
        self.lifecycle = lifecycle;
        self
    }

    /// Returns the explicit behavior contract for this resource specification.
    pub fn behavior_contract(&self) -> ResourceBehaviorContract {
        ResourceBehaviorContract {
            materialization: self.materialization.clone(),
            activation: self.activation.clone(),
            mutation_scope: self.mutation_scope.clone(),
            provenance: self.provenance.clone(),
            lifecycle: self.lifecycle.clone(),
        }
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResolvedResourceContext {
    pub id: ResourceId,
    pub version: ResolvedVersion,
    pub locator: ResolvedLocator,
}

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

    pub fn version_selection_policy(&self) -> Result<SelectionPolicy> {
        self.spec.version.selection_policy()
    }

    pub fn select_preferred_resolved<'a>(
        &self,
        candidates: &'a [ResolvedResource],
    ) -> Result<Option<&'a ResolvedResource>> {
        let policy = self.version_selection_policy()?;
        let parsed_versions = candidates
            .iter()
            .filter(|candidate| candidate.spec().id == self.spec.id)
            .map(|candidate| {
                parse_resolved_version(candidate.version()).map(|version| (candidate, version))
            })
            .collect::<Result<Vec<_>>>()?;

        let versions = parsed_versions
            .iter()
            .map(|(_, version)| version.clone())
            .collect::<Vec<_>>();
        let Some(selected) = select_preferred(&versions, &policy) else {
            return Ok(None);
        };

        Ok(parsed_versions
            .into_iter()
            .find_map(|(candidate, version)| (&version == selected).then_some(candidate)))
    }

    /// Returns the explicit behavior contract for this resource.
    pub fn behavior_contract(&self) -> ResourceBehaviorContract {
        self.spec.behavior_contract()
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

    pub fn context(&self) -> ResolvedResourceContext {
        ResolvedResourceContext {
            id: self.spec.id.clone(),
            version: self.state.version.clone(),
            locator: self.state.locator.clone(),
        }
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
        if !self
            .spec
            .version
            .matches_resolved_version(&self.state.version)?
        {
            return Err(ResourceError::ResolvedVersionMismatch {
                selector: self.spec.version.as_label(),
                version: self.state.version.as_str().to_string(),
            });
        }

        Ok(())
    }
}

fn parse_resolved_version(version: &ResolvedVersion) -> Result<VersionKind> {
    VersionKind::parse(version.as_str())
        .map_err(|_| ResourceError::InvalidResolvedVersion(version.as_str().to_string()))
}

fn alias_selection_policy(alias: &str) -> Result<SelectionPolicy> {
    let preference = match alias.to_ascii_lowercase().as_str() {
        "latest" => VersionPreference::Latest,
        "lowest" => VersionPreference::Lowest,
        "stable" => VersionPreference::HighestStable,
        "lts" => VersionPreference::Lts,
        _ => return Err(ResourceError::UnknownVersionAlias(alias.to_string())),
    };

    Ok(selection_policy(VersionRequirement::Any, preference))
}

fn selection_policy(
    requirement: VersionRequirement,
    preference: VersionPreference,
) -> SelectionPolicy {
    SelectionPolicy {
        requirement,
        preference,
    }
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

fn non_empty_string(value: impl Into<String>) -> Result<String> {
    let value = value.into();
    ensure_non_empty(&value)?;
    Ok(value)
}

fn parse_non_empty_value<T, F, E>(value: impl Into<String>, parse: F) -> Result<T>
where
    F: FnOnce(&str) -> std::result::Result<T, E>,
{
    let value = non_empty_string(value)?;
    parse(&value).map_err(|_| ResourceError::EmptyValue)
}

fn ensure_non_empty_collection<T>(values: &[T], error: ResourceError) -> Result<()> {
    if values.is_empty() {
        Err(error)
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
    fn version_selector_exact_maps_to_pinned_policy() {
        let selector = VersionSelector::exact("1.2.3").unwrap();
        let policy = selector.selection_policy().unwrap();

        assert_eq!(
            policy,
            SelectionPolicy {
                requirement: VersionRequirement::Exact(VersionKind::parse("1.2.3").unwrap()),
                preference: VersionPreference::Pinned(VersionKind::parse("1.2.3").unwrap()),
            }
        );
    }

    #[test]
    fn version_selector_requirement_prefers_highest_stable() {
        let selector = VersionSelector::requirement("^1.2").unwrap();
        let policy = selector.selection_policy().unwrap();

        assert_eq!(
            policy.requirement,
            VersionRequirement::parse("^1.2").unwrap()
        );
        assert_eq!(policy.preference, VersionPreference::HighestStable);
    }

    #[test]
    fn version_selector_alias_maps_common_preferences() {
        assert_eq!(
            VersionSelector::alias("latest")
                .unwrap()
                .selection_policy()
                .unwrap()
                .preference,
            VersionPreference::Latest
        );
        assert_eq!(
            VersionSelector::alias("stable")
                .unwrap()
                .selection_policy()
                .unwrap()
                .preference,
            VersionPreference::HighestStable
        );
        assert_eq!(
            VersionSelector::alias("lts")
                .unwrap()
                .selection_policy()
                .unwrap()
                .preference,
            VersionPreference::Lts
        );
    }

    #[test]
    fn version_selector_rejects_unknown_alias_for_selection_policy() {
        assert!(matches!(
            VersionSelector::alias("canary").unwrap().selection_policy(),
            Err(ResourceError::UnknownVersionAlias(alias)) if alias == "canary"
        ));
    }

    #[test]
    fn resource_exposes_version_selection_policy() {
        let resource = RequestedResource::new(
            ResourceSpec::new(
                ResourceId::parse("nodejs.org/node").unwrap(),
                ResourceLocator::Url(ValidUrl::parse("https://example.com/node.zip").unwrap()),
            )
            .version(VersionSelector::alias("stable").unwrap()),
        );

        let policy = resource.version_selection_policy().unwrap();
        assert_eq!(policy.preference, VersionPreference::HighestStable);
    }

    #[test]
    fn resource_can_select_preferred_resolved_candidate() {
        let resource = RequestedResource::new(
            ResourceSpec::new(
                ResourceId::parse("nodejs.org/node").unwrap(),
                ResourceLocator::Url(ValidUrl::parse("https://example.com/node.zip").unwrap()),
            )
            .version(VersionSelector::alias("lts").unwrap()),
        );
        let candidates = vec![
            RequestedResource::new(ResourceSpec::new(
                ResourceId::parse("nodejs.org/node").unwrap(),
                ResourceLocator::Url(ValidUrl::parse("https://example.com/node-20.zip").unwrap()),
            ))
            .resolve(
                ResolvedVersion::new("20.11.0").unwrap(),
                ResolvedLocator::Url(ValidUrl::parse("https://example.com/node-20.zip").unwrap()),
                None,
            ),
            RequestedResource::new(ResourceSpec::new(
                ResourceId::parse("nodejs.org/node").unwrap(),
                ResourceLocator::Url(ValidUrl::parse("https://example.com/node-22.zip").unwrap()),
            ))
            .resolve(
                ResolvedVersion::new("22.4.0").unwrap(),
                ResolvedLocator::Url(ValidUrl::parse("https://example.com/node-22.zip").unwrap()),
                None,
            ),
        ];

        let selected = resource
            .select_preferred_resolved(&candidates)
            .unwrap()
            .unwrap();
        assert_eq!(selected.version().as_str(), "22.4.0");
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

    #[test]
    fn resource_behavior_contract_has_explicit_defaults() {
        let spec = ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
        );

        let contract = spec.behavior_contract();
        assert_eq!(contract.activation, ActivationModel::None);
        assert_eq!(contract.mutation_scope, MutationScope::InstallRootOnly);
        assert_eq!(contract.provenance, ProvenanceRequirement::SourceOnly);
        assert_eq!(contract.lifecycle, LifecycleRequirements::default());
    }

    #[test]
    fn resource_behavior_contract_can_be_specialized_by_axis() {
        let lifecycle = LifecycleRequirements::default()
            .replace(true)
            .rollback(true)
            .repair(true)
            .uninstall(true);
        let spec = ResourceSpec::new(
            ResourceId::parse("example/service").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/service.tar.zst").unwrap()),
        )
        .materialization(MaterializationSpec {
            form: ArtifactForm::Archive,
            unpack: UnpackPolicy::Extract {
                strip_components: 1,
            },
        })
        .activation_model(ActivationModel::ServiceRegistration)
        .mutation_scope(MutationScope::InstallRootWithExtensions)
        .provenance_requirement(ProvenanceRequirement::SourceAndVerification)
        .lifecycle_requirements(lifecycle.clone());

        let contract = spec.behavior_contract();
        assert_eq!(contract.materialization.form, ArtifactForm::Archive);
        assert_eq!(contract.activation, ActivationModel::ServiceRegistration);
        assert_eq!(
            contract.mutation_scope,
            MutationScope::InstallRootWithExtensions
        );
        assert_eq!(
            contract.provenance,
            ProvenanceRequirement::SourceAndVerification
        );
        assert_eq!(contract.lifecycle, lifecycle);
    }

    #[test]
    fn requested_and_resolved_resource_share_behavior_contract() {
        let requested = RequestedResource::new(
            ResourceSpec::new(
                ResourceId::parse("example/tool").unwrap(),
                ResourceLocator::Url(ValidUrl::parse("https://example.com/tool.zip").unwrap()),
            )
            .activation_model(ActivationModel::ShimResolution)
            .mutation_scope(MutationScope::InstallRootWithExtensions),
        );
        let expected = requested.behavior_contract();

        let resolved = requested.resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(ValidUrl::parse("https://example.com/tool.zip").unwrap()),
            None,
        );

        assert_eq!(resolved.behavior_contract(), expected);
    }
}
