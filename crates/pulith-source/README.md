# pulith-source

Composable source definitions and planning.

## Main APIs

- `SourceSpec`
- `PlannedSources`
- `SelectionStrategy`
- `SourceDefinition`

## Basic Usage

```rust
use pulith_resource::{ResourceLocator, ValidUrl};
use pulith_source::{PlannedSources, SelectionStrategy};

let planned = PlannedSources::from_locator(
    &ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip")?),
    SelectionStrategy::OrderedFallback,
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

See `docs/design/source.md`.
