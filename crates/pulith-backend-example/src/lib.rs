//! Thin backend example crate for Pulith.
//!
//! This crate demonstrates how a backend can stay adapter-first:
//! it shapes resource, source, and install inputs without absorbing
//! fetch/store/state policy into a monolithic framework.

use std::path::PathBuf;

use pulith_install::{
    ActivationTarget, InstallInput, InstallSpec, ShimCommand, ShimCopyActivator, ShimLinkActivator,
};
use pulith_resource::{
    Metadata, RequestedResource, ResolvedResource, ResourceId, ResourceLocator, ResourceSpec,
    Result as ResourceResult, VersionSelector,
};
use pulith_source::{PlannedSources, Result as SourceResult, SelectionStrategy, SourceSpec};
use pulith_version::SelectionPolicy;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, BackendError>;

#[derive(Debug, Error)]
pub enum BackendError {
    #[error(transparent)]
    Resource(#[from] pulith_resource::ResourceError),
    #[error(transparent)]
    Source(#[from] pulith_source::SourceError),
    #[error(transparent)]
    Install(#[from] pulith_install::InstallError),
}

#[derive(Debug, Clone)]
pub struct ManagedBinarySpec {
    pub id: ResourceId,
    pub locator: ResourceLocator,
    pub version: VersionSelector,
    pub install_root: PathBuf,
    pub executable_path: PathBuf,
    pub activation_path: Option<PathBuf>,
    pub metadata: Metadata,
}

impl ManagedBinarySpec {
    pub fn new(
        id: ResourceId,
        locator: ResourceLocator,
        version: VersionSelector,
        install_root: PathBuf,
        executable_path: PathBuf,
    ) -> Self {
        Self {
            id,
            locator,
            version,
            install_root,
            executable_path,
            activation_path: None,
            metadata: Metadata::new(),
        }
    }

    pub fn activation_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.activation_path = Some(path.into());
        self
    }

    pub fn metadata(mut self, metadata: Metadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn resource_spec(&self) -> ResourceSpec {
        ResourceSpec::new(self.id.clone(), self.locator.clone()).version(self.version.clone())
    }

    pub fn requested_resource(&self) -> RequestedResource {
        RequestedResource::new(self.resource_spec())
    }

    pub fn version_selection_policy(&self) -> ResourceResult<SelectionPolicy> {
        self.requested_resource().version_selection_policy()
    }

    pub fn select_preferred_resolved<'a>(
        &self,
        candidates: &'a [ResolvedResource],
    ) -> ResourceResult<Option<&'a ResolvedResource>> {
        self.requested_resource()
            .select_preferred_resolved(candidates)
    }

    pub fn source_spec(&self) -> SourceResult<SourceSpec> {
        SourceSpec::from_locator(&self.locator)
    }

    pub fn planned_sources(&self, strategy: SelectionStrategy) -> SourceResult<PlannedSources> {
        PlannedSources::from_locator(&self.locator, strategy)
    }

    pub fn install_spec(&self, resource: ResolvedResource, input: InstallInput) -> InstallSpec {
        let mut spec = InstallSpec::new(resource, input, self.install_root.clone());
        spec.metadata = self.metadata.clone();
        if let Some(path) = &self.activation_path {
            spec = spec.activation(ActivationTarget { path: path.clone() });
        }
        spec
    }

    pub fn shim_command(&self, command: impl Into<String>) -> Result<ShimCommand> {
        Ok(ShimCommand::new(command, self.executable_path.clone())?)
    }

    pub fn shim_activator(&self, command: impl Into<String>) -> Result<ShimLinkActivator> {
        Ok(ShimLinkActivator::new(self.shim_command(command)?))
    }

    pub fn shim_copy_activator(&self, command: impl Into<String>) -> Result<ShimCopyActivator> {
        Ok(ShimCopyActivator::new(self.shim_command(command)?))
    }
}

pub fn managed_binary(
    id: &str,
    locator: ResourceLocator,
    version: VersionSelector,
    install_root: impl Into<PathBuf>,
    executable_path: impl Into<PathBuf>,
) -> ResourceResult<ManagedBinarySpec> {
    Ok(ManagedBinarySpec::new(
        ResourceId::parse(id)?,
        locator,
        version,
        install_root.into(),
        executable_path.into(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulith_install::InstallInput;
    use pulith_resource::{ResolvedLocator, ResolvedVersion, ValidUrl};

    #[test]
    fn managed_binary_produces_resource_and_source_specs() {
        let spec = managed_binary(
            "example/runtime",
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
            VersionSelector::exact("1.0.0").unwrap(),
            "/installs/runtime",
            "bin/runtime",
        )
        .unwrap();

        let requested = spec.requested_resource();
        let planned = spec
            .source_spec()
            .unwrap()
            .plan(pulith_source::SelectionStrategy::OrderedFallback);

        assert_eq!(requested.spec().id.as_string(), "example/runtime");
        assert_eq!(planned.candidates().len(), 1);
    }

    #[test]
    fn managed_binary_produces_planned_sources_directly() {
        let spec = managed_binary(
            "example/runtime",
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
            VersionSelector::exact("1.0.0").unwrap(),
            "/installs/runtime",
            "bin/runtime",
        )
        .unwrap();

        let planned = spec
            .planned_sources(SelectionStrategy::OrderedFallback)
            .unwrap();

        assert_eq!(planned.candidates().len(), 1);
        assert_eq!(planned.strategy(), &SelectionStrategy::OrderedFallback);
    }

    #[test]
    fn managed_binary_exposes_version_selection_policy() {
        let spec = managed_binary(
            "example/runtime",
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
            VersionSelector::alias("lts").unwrap(),
            "/installs/runtime",
            "bin/runtime",
        )
        .unwrap();

        let policy = spec.version_selection_policy().unwrap();
        assert_eq!(policy.preference, pulith_version::VersionPreference::Lts);
    }

    #[test]
    fn managed_binary_can_select_preferred_resolved_candidate() {
        let spec = managed_binary(
            "example/runtime",
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
            VersionSelector::alias("stable").unwrap(),
            "/installs/runtime",
            "bin/runtime",
        )
        .unwrap();
        let candidates = vec![
            spec.requested_resource().clone().resolve(
                ResolvedVersion::new("1.2.0-alpha.1").unwrap(),
                ResolvedLocator::Url(
                    ValidUrl::parse("https://example.com/runtime-alpha.zip").unwrap(),
                ),
                None,
            ),
            spec.requested_resource().clone().resolve(
                ResolvedVersion::new("1.1.0").unwrap(),
                ResolvedLocator::Url(
                    ValidUrl::parse("https://example.com/runtime-1.1.0.zip").unwrap(),
                ),
                None,
            ),
        ];

        let selected = spec
            .select_preferred_resolved(&candidates)
            .unwrap()
            .unwrap();
        assert_eq!(selected.version().as_str(), "1.1.0");
    }

    #[test]
    fn managed_binary_builds_install_spec_and_shim_activator() {
        let spec = managed_binary(
            "example/runtime",
            ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
            VersionSelector::exact("1.0.0").unwrap(),
            "/installs/runtime",
            "bin/runtime",
        )
        .unwrap()
        .activation_path("/active/runtime");

        let resolved = spec.requested_resource().resolve(
            ResolvedVersion::new("1.0.0").unwrap(),
            ResolvedLocator::Url(ValidUrl::parse("https://example.com/runtime.zip").unwrap()),
            None,
        );
        let install = spec.install_spec(
            resolved,
            InstallInput::from_fetch_receipt(pulith_fetch::FetchReceipt {
                source: pulith_fetch::FetchSource::Url(
                    "https://example.com/runtime.zip".to_string(),
                ),
                destination: PathBuf::from("/downloads/runtime.zip"),
                bytes_downloaded: 10,
                total_bytes: Some(10),
                sha256_hex: None,
            }),
        );

        assert_eq!(install.install_root, PathBuf::from("/installs/runtime"));
        assert!(install.activation.is_some());
        assert_eq!(
            spec.shim_command("runtime").unwrap().relative_target,
            PathBuf::from("bin/runtime")
        );
        let _link_activator = spec.shim_activator("runtime").unwrap();
        let _copy_activator = spec.shim_copy_activator("runtime").unwrap();
    }
}
