# pulith-source

Composable source planning with normalized remote-source vocabulary.

## What This Crate Owns

`pulith-source` describes where a resource can come from and turns that into a planned list of candidates.

It owns:

- source-set declaration
- remote/local source normalization
- planning strategy selection
- expansion of mirrors into executable candidates

It does not own:

- transfer execution
- store/install policy
- ranking heuristics beyond explicit selection strategy

## Main Types

- `SourceSpec`
- `PlannedSources`
- `SourceSet`
- `SourceDefinition`
- `RemoteSource`
- `ResolvedSourceCandidate`
- `SelectionStrategy`
- `SourcePath`

## Normalized Source Model

Remote source families are grouped under `RemoteSource`:

- `RemoteSource::HttpAsset`
- `RemoteSource::Mirror`
- `RemoteSource::Git`

This avoids overlapping top-level type trees for URL, mirror, and git definitions.

## Basic Usage

```rust
use pulith_resource::{ResourceLocator, ValidUrl};
use pulith_source::{PlannedSources, SelectionStrategy};

let planned = PlannedSources::from_locator(
    &ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip")?),
    SelectionStrategy::OrderedFallback,
)?;

assert_eq!(planned.candidates().len(), 1);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Mirror Example

```rust
use pulith_resource::ValidUrl;
use pulith_source::{MirrorSource, RemoteSource, SelectionStrategy, SourceDefinition, SourceSet, SourceSpec};

let mirrors = vec![
    ValidUrl::parse("https://mirror-a.example.com/")?,
    ValidUrl::parse("https://mirror-b.example.com/")?,
];

let set = SourceSet::new(vec![SourceDefinition::Remote(RemoteSource::Mirror(
    MirrorSource::new(mirrors, "downloads/tool.tar.gz")?,
))])?;

let planned = SourceSpec::new(set).plan(SelectionStrategy::Race);
assert_eq!(planned.candidates().len(), 2);
# Ok::<(), Box<dyn std::error::Error>>(())
```

## String Boundaries

`SourcePath` supports `FromStr` and `Display`, so path-like source fragments can be parsed and rendered consistently.

## See Also

- `docs/design/source.md`
- `crates/pulith-fetch/README.md`
