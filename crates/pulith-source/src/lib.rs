//! Composable source abstractions and planning for Pulith.

use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use pulith_resource::{RequestedResource, ResolvedResource, ResourceLocator, ValidUrl};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, SourceError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum SourceError {
    #[error("source set must not be empty")]
    EmptySourceSet,
    #[error("mirror set must not be empty")]
    EmptyMirrorSet,
    #[error("path must not be empty")]
    EmptyPath,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HttpAssetSource {
    pub url: ValidUrl,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourcePath(String);

impl SourcePath {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        ensure_non_empty_string(&value, SourceError::EmptyPath)?;
        Ok(Self(value))
    }
}

impl fmt::Display for SourcePath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for SourcePath {
    type Err = SourceError;

    fn from_str(s: &str) -> Result<Self> {
        Self::new(s)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MirrorSource {
    pub mirrors: Vec<ValidUrl>,
    pub path: SourcePath,
}

impl MirrorSource {
    pub fn new(mirrors: Vec<ValidUrl>, path: impl Into<String>) -> Result<Self> {
        ensure_non_empty_slice(&mirrors, SourceError::EmptyMirrorSet)?;
        Ok(Self {
            mirrors,
            path: SourcePath::new(path)?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalSource {
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitSource {
    pub url: ValidUrl,
    pub rev: Option<String>,
    pub subpath: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RemoteSource {
    HttpAsset(HttpAssetSource),
    Mirror(MirrorSource),
    Git(GitSource),
}

impl RemoteSource {
    pub fn resolved_candidates(&self) -> Vec<ResolvedSourceCandidate> {
        match self {
            Self::HttpAsset(source) => vec![ResolvedSourceCandidate::Url(source.url.clone())],
            Self::Mirror(source) => source
                .mirrors
                .iter()
                .map(|base| {
                    let joined = base
                        .as_url()
                        .join(&source.path.to_string())
                        .expect("validated mirror path");
                    ResolvedSourceCandidate::Url(
                        ValidUrl::parse(joined.as_str()).expect("joined mirror URL"),
                    )
                })
                .collect(),
            Self::Git(source) => vec![ResolvedSourceCandidate::Git {
                url: source.url.clone(),
                rev: source.rev.clone(),
                subpath: source.subpath.clone(),
            }],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceDefinition {
    Remote(RemoteSource),
    Local(LocalSource),
}

impl SourceDefinition {
    pub fn resolved_candidates(&self) -> Vec<ResolvedSourceCandidate> {
        match self {
            Self::Remote(remote) => remote.resolved_candidates(),
            Self::Local(source) => vec![ResolvedSourceCandidate::LocalPath(source.path.clone())],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectionStrategy {
    OrderedFallback,
    Race,
    Exhaustive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSet {
    entries: Vec<SourceDefinition>,
}

impl SourceSet {
    pub fn new(entries: Vec<SourceDefinition>) -> Result<Self> {
        ensure_non_empty_slice(&entries, SourceError::EmptySourceSet)?;
        Ok(Self { entries })
    }

    pub fn entries(&self) -> &[SourceDefinition] {
        &self.entries
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unplanned;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Planned {
    strategy: SelectionStrategy,
    candidates: Vec<ResolvedSourceCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePlan<S> {
    set: SourceSet,
    state: S,
}

pub type SourceSpec = SourcePlan<Unplanned>;
pub type PlannedSources = SourcePlan<Planned>;

impl SourceSpec {
    pub fn new(set: SourceSet) -> Self {
        Self {
            set,
            state: Unplanned,
        }
    }

    pub fn from_locator(locator: &ResourceLocator) -> Result<Self> {
        Ok(Self::new(source_set_from_locator(locator)?))
    }

    pub fn from_requested_resource(resource: &RequestedResource) -> Result<Self> {
        Self::from_locator(&resource.spec().locator)
    }

    pub fn from_resolved_resource(resource: &ResolvedResource) -> Result<Self> {
        Self::from_locator(&resource.spec().locator)
    }

    pub fn plan(self, strategy: SelectionStrategy) -> PlannedSources {
        planned_sources(self.set, strategy)
    }

    pub fn into_planned(self, strategy: SelectionStrategy) -> PlannedSources {
        self.plan(strategy)
    }
}

impl<S> SourcePlan<S> {
    pub fn set(&self) -> &SourceSet {
        &self.set
    }
}

impl PlannedSources {
    pub fn from_locator(locator: &ResourceLocator, strategy: SelectionStrategy) -> Result<Self> {
        Ok(planned_sources(source_set_from_locator(locator)?, strategy))
    }

    pub fn from_requested_resource(
        resource: &RequestedResource,
        strategy: SelectionStrategy,
    ) -> Result<Self> {
        Ok(SourceSpec::from_requested_resource(resource)?.plan(strategy))
    }

    pub fn from_resolved_resource(
        resource: &ResolvedResource,
        strategy: SelectionStrategy,
    ) -> Result<Self> {
        Ok(SourceSpec::from_resolved_resource(resource)?.plan(strategy))
    }

    pub fn strategy(&self) -> &SelectionStrategy {
        &self.state.strategy
    }

    pub fn candidates(&self) -> &[ResolvedSourceCandidate] {
        &self.state.candidates
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ResolvedSourceCandidate {
    Url(ValidUrl),
    LocalPath(PathBuf),
    Git {
        url: ValidUrl,
        rev: Option<String>,
        subpath: Option<PathBuf>,
    },
}

impl ResolvedSourceCandidate {
    fn from_definition(definition: &SourceDefinition) -> Vec<Self> {
        definition.resolved_candidates()
    }
}

fn source_set_from_locator(locator: &ResourceLocator) -> Result<SourceSet> {
    match locator {
        ResourceLocator::Url(url) => SourceSet::new(vec![http_asset(url.clone())]),
        ResourceLocator::Alternatives(urls) => {
            SourceSet::new(urls.iter().cloned().map(http_asset).collect())
        }
        ResourceLocator::LocalPath(path) => {
            SourceSet::new(vec![SourceDefinition::Local(LocalSource {
                path: path.clone(),
            })])
        }
    }
}

fn planned_sources(set: SourceSet, strategy: SelectionStrategy) -> PlannedSources {
    let candidates = set
        .entries
        .iter()
        .flat_map(ResolvedSourceCandidate::from_definition)
        .collect();

    SourcePlan {
        set,
        state: Planned {
            strategy,
            candidates,
        },
    }
}

fn http_asset(url: ValidUrl) -> SourceDefinition {
    SourceDefinition::Remote(RemoteSource::HttpAsset(HttpAssetSource {
        url,
        file_name: None,
    }))
}

fn ensure_non_empty_slice<T>(values: &[T], error: SourceError) -> Result<()> {
    if values.is_empty() {
        Err(error)
    } else {
        Ok(())
    }
}

fn ensure_non_empty_string(value: &str, error: SourceError) -> Result<()> {
    if value.is_empty() { Err(error) } else { Ok(()) }
}

pub trait SourceAdapter {
    fn expand(
        &self,
        resource: &ResolvedResource,
        definition: &SourceDefinition,
    ) -> Result<SourceSet>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct PassthroughAdapter;

impl SourceAdapter for PassthroughAdapter {
    fn expand(
        &self,
        _resource: &ResolvedResource,
        definition: &SourceDefinition,
    ) -> Result<SourceSet> {
        SourceSet::new(vec![definition.clone()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulith_resource::{
        RequestedResource, ResolvedLocator, ResolvedVersion, ResourceId, ResourceSpec,
    };

    #[test]
    fn source_spec_can_be_built_from_locator() {
        let locator = ResourceLocator::Alternatives(vec![
            ValidUrl::parse("https://a.example.com/file.zip").unwrap(),
            ValidUrl::parse("https://b.example.com/file.zip").unwrap(),
        ]);

        let spec = SourceSpec::from_locator(&locator).unwrap();
        let planned = spec.plan(SelectionStrategy::OrderedFallback);
        assert_eq!(planned.candidates().len(), 2);
    }

    #[test]
    fn source_spec_can_be_built_from_requested_resource() {
        let requested = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
        ));

        let planned = SourceSpec::from_requested_resource(&requested)
            .unwrap()
            .plan(SelectionStrategy::OrderedFallback);

        assert_eq!(planned.candidates().len(), 1);
    }

    #[test]
    fn planned_sources_can_be_built_from_requested_resource() {
        let requested = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
        ));

        let planned =
            PlannedSources::from_requested_resource(&requested, SelectionStrategy::OrderedFallback)
                .unwrap();

        assert_eq!(planned.candidates().len(), 1);
        assert_eq!(planned.strategy(), &SelectionStrategy::OrderedFallback);
    }

    #[test]
    fn source_spec_can_be_built_from_resolved_resource() {
        let resolved = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(PathBuf::from("/tmp/runtime.bin")),
        ))
        .resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::LocalPath(PathBuf::from("/tmp/runtime.bin")),
            None,
        );

        let planned = SourceSpec::from_resolved_resource(&resolved)
            .unwrap()
            .plan(SelectionStrategy::OrderedFallback);

        assert_eq!(planned.candidates().len(), 1);
    }

    #[test]
    fn planned_sources_can_be_built_from_resolved_resource() {
        let resolved = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/runtime").unwrap(),
            ResourceLocator::LocalPath(PathBuf::from("/tmp/runtime.bin")),
        ))
        .resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::LocalPath(PathBuf::from("/tmp/runtime.bin")),
            None,
        );

        let planned =
            PlannedSources::from_resolved_resource(&resolved, SelectionStrategy::OrderedFallback)
                .unwrap();

        assert_eq!(planned.candidates().len(), 1);
        assert_eq!(planned.strategy(), &SelectionStrategy::OrderedFallback);
    }

    #[test]
    fn mirror_source_expands_to_urls() {
        let mirrors = vec![
            ValidUrl::parse("https://mirror-a.example.com/").unwrap(),
            ValidUrl::parse("https://mirror-b.example.com/").unwrap(),
        ];
        let set = SourceSet::new(vec![SourceDefinition::Remote(RemoteSource::Mirror(
            MirrorSource::new(mirrors, "downloads/tool.tar.gz").unwrap(),
        ))])
        .unwrap();
        let planned = SourceSpec::new(set).plan(SelectionStrategy::Race);
        assert_eq!(planned.candidates().len(), 2);
    }

    #[test]
    fn adapter_expands_source_for_resource() {
        let resource = RequestedResource::new(ResourceSpec::new(
            ResourceId::parse("example/tool").unwrap(),
            ResourceLocator::Url(ValidUrl::parse("https://example.com/tool.zip").unwrap()),
        ))
        .resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(ValidUrl::parse("https://example.com/tool.zip").unwrap()),
            None,
        );

        let definition = SourceDefinition::Remote(RemoteSource::HttpAsset(HttpAssetSource {
            url: ValidUrl::parse("https://example.com/tool.zip").unwrap(),
            file_name: Some("tool.zip".to_string()),
        }));

        let expanded = PassthroughAdapter.expand(&resource, &definition).unwrap();
        assert_eq!(expanded.entries().len(), 1);
    }
}
