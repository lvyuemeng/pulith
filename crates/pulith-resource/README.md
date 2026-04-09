# pulith-resource

Semantic resource description types.

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

See `docs/design/resource.md`.
