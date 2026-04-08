//! Composable source abstractions and planning for Pulith.

use std::path::PathBuf;

use pulith_resource::{ResolvedResource, ResourceLocator, ValidUrl};
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
pub struct MirrorSource {
    pub mirrors: Vec<ValidUrl>,
    pub path: String,
}

impl MirrorSource {
    pub fn new(mirrors: Vec<ValidUrl>, path: impl Into<String>) -> Result<Self> {
        let path = path.into();
        if mirrors.is_empty() {
            return Err(SourceError::EmptyMirrorSet);
        }
        if path.is_empty() {
            return Err(SourceError::EmptyPath);
        }
        Ok(Self { mirrors, path })
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
pub enum SourceDefinition {
    HttpAsset(HttpAssetSource),
    Mirror(MirrorSource),
    Local(LocalSource),
    Git(GitSource),
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
        if entries.is_empty() {
            return Err(SourceError::EmptySourceSet);
        }
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
        let set = match locator {
            ResourceLocator::Url(url) => {
                SourceSet::new(vec![SourceDefinition::HttpAsset(HttpAssetSource {
                    url: url.clone(),
                    file_name: None,
                })])?
            }
            ResourceLocator::Alternatives(urls) => SourceSet::new(
                urls.iter()
                    .cloned()
                    .map(|url| {
                        SourceDefinition::HttpAsset(HttpAssetSource {
                            url,
                            file_name: None,
                        })
                    })
                    .collect(),
            )?,
            ResourceLocator::LocalPath(path) => {
                SourceSet::new(vec![SourceDefinition::Local(LocalSource {
                    path: path.clone(),
                })])?
            }
        };

        Ok(Self::new(set))
    }

    pub fn plan(self, strategy: SelectionStrategy) -> PlannedSources {
        let candidates = self
            .set
            .entries
            .iter()
            .flat_map(ResolvedSourceCandidate::from_definition)
            .collect();

        SourcePlan {
            set: self.set,
            state: Planned {
                strategy,
                candidates,
            },
        }
    }
}

impl<S> SourcePlan<S> {
    pub fn set(&self) -> &SourceSet {
        &self.set
    }
}

impl PlannedSources {
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
        match definition {
            SourceDefinition::HttpAsset(source) => vec![Self::Url(source.url.clone())],
            SourceDefinition::Mirror(source) => source
                .mirrors
                .iter()
                .map(|base| {
                    let joined = base
                        .as_url()
                        .join(&source.path)
                        .expect("validated mirror path");
                    Self::Url(ValidUrl::parse(joined.as_str()).expect("joined mirror URL"))
                })
                .collect(),
            SourceDefinition::Local(source) => vec![Self::LocalPath(source.path.clone())],
            SourceDefinition::Git(source) => vec![Self::Git {
                url: source.url.clone(),
                rev: source.rev.clone(),
                subpath: source.subpath.clone(),
            }],
        }
    }
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
    fn mirror_source_expands_to_urls() {
        let mirrors = vec![
            ValidUrl::parse("https://mirror-a.example.com/").unwrap(),
            ValidUrl::parse("https://mirror-b.example.com/").unwrap(),
        ];
        let set = SourceSet::new(vec![SourceDefinition::Mirror(
            MirrorSource::new(mirrors, "downloads/tool.tar.gz").unwrap(),
        )])
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

        let definition = SourceDefinition::HttpAsset(HttpAssetSource {
            url: ValidUrl::parse("https://example.com/tool.zip").unwrap(),
            file_name: Some("tool.zip".to_string()),
        });

        let expanded = PassthroughAdapter.expand(&resource, &definition).unwrap();
        assert_eq!(expanded.entries().len(), 1);
    }
}
