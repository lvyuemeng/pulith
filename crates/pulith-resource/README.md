# pulith-resource

Semantic resource identity, locator, version, trust, and behavior-contract types.

## What This Crate Owns

`pulith-resource` defines what a managed thing is before any fetch/store/install step happens.

It owns:

- `ResourceId`
- `ResourceLocator`
- `VersionSelector`
- trust and verification requirements
- typed behavior contract axes

## Main Types

- `ResourceId`
- `ValidUrl`
- `ResourceSpec`
- `RequestedResource`
- `ResolvedResource`
- `MaterializationSpec`
- `ActivationModel`
- `MutationScope`
- `ProvenanceRequirement`
- `LifecycleRequirements`

## Basic Usage

```rust
use pulith_resource::{
    ActivationModel, LifecycleRequirements, MutationScope, RequestedResource, ResourceId,
    ResourceLocator, ResourceSpec, ValidUrl, VersionSelector,
};

let requested = RequestedResource::new(
    ResourceSpec::new(
        ResourceId::parse("example/runtime")?,
        ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip")?),
    )
    .version(VersionSelector::alias("stable")?)
    .activation_model(ActivationModel::PathTarget)
    .mutation_scope(MutationScope::InstallRootOnly)
    .lifecycle_requirements(LifecycleRequirements::default().replace(true).rollback(true)),
);
# let _ = requested;
# Ok::<(), pulith_resource::ResourceError>(())
```

## Parsing and Formatting

First-class string boundary types support trait-based ergonomics:

- `ResourceId: FromStr + Display`
- `ValidUrl: FromStr + Display`

That means callers can parse or render them without ad hoc helper glue.

## See Also

- `docs/design/resource.md`
- `crates/pulith-source/README.md`
