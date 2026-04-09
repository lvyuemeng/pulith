# pulith-resource

Semantic resource description types.

## Role

`pulith-resource` defines what a managed thing is.

It owns:

- resource identity
- locators
- version selectors
- trust/provenance-facing metadata types

## Main APIs

- `ResourceId`
- `ResourceSpec`
- `RequestedResource`
- `ResolvedResource`
- `VersionSelector`
- `ValidUrl`

## Basic Usage

```rust
use pulith_resource::{RequestedResource, ResourceId, ResourceLocator, ResourceSpec, VersionSelector, ValidUrl};

let requested = RequestedResource::new(
    ResourceSpec::new(
        ResourceId::parse("example/runtime")?,
        ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip")?),
    )
    .version(VersionSelector::alias("stable")?),
);
# Ok::<(), pulith_resource::ResourceError>(())
```

## How To Use It

Most composed flows begin here.

Use this crate to define the semantic identity and version intent that later crates will preserve across planning, storage, install, and state.

See `docs/design/resource.md`.
