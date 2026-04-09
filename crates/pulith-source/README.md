# pulith-source

Composable source definitions and planning.

## Role

`pulith-source` describes where resources may come from and turns that into executable candidate plans.

It should not execute fetches or manage storage/install policy.

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

## How To Use It

Use this crate when you have a semantic resource or locator and need:

- structured source definitions
- ordered fallback plans
- race-style source candidate plans

See `docs/design/source.md`.
