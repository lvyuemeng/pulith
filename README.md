# pulith

Pulith is a Rust crate ecosystem for building resource managers with typed, composable workflow contracts.

It is designed for tools that need to:

- describe resources semantically
- plan source candidates and mirrors
- fetch and verify bytes explicitly
- extract archives safely
- register artifacts with provenance
- install and activate content predictably
- persist lifecycle state, inspect drift, and plan repair/cleanup

## Published Crates

Current crates.io wave now includes:

- `pulith-fs`
- `pulith-version`
- `pulith-verify`
- `pulith-shim`
- `pulith-resource`
- `pulith-platform`
- `pulith-archive`
- `pulith-serde-backend`
- `pulith-source`
- `pulith-fetch`
- `pulith-lock`
- `pulith-store`
- `pulith-state`
- `pulith-install`

Publish status and dependency-order details live in `docs/publish/overview.md`.

## Core Pipeline

Pulith standardizes on this explicit pipeline:

`resource -> source plan -> fetch -> verify -> extract/register -> install -> activate -> state`

Each crate owns one boundary in that flow. The goal is to reduce glue without hiding policy.

## Quick Start

### 1. Describe a resource

```rust
use pulith_resource::{RequestedResource, ResourceId, ResourceLocator, ResourceSpec, ValidUrl, VersionSelector};

let requested = RequestedResource::new(
    ResourceSpec::new(
        ResourceId::parse("example/runtime")?,
        ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip")?),
    )
    .version(VersionSelector::alias("stable")?),
);
# Ok::<(), pulith_resource::ResourceError>(())
```

### 2. Plan sources

```rust
use pulith_source::{PlannedSources, SelectionStrategy};

# use pulith_resource::{ResourceLocator, ValidUrl};
let planned = PlannedSources::from_locator(
    &ResourceLocator::Url(ValidUrl::parse("https://example.com/runtime.zip")?),
    SelectionStrategy::OrderedFallback,
)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

### 3. Fetch bytes

```rust
use pulith_fetch::{Fetcher, MultiSourceFetcher, ReqwestClient};
use std::sync::Arc;

let fetcher = Fetcher::new(ReqwestClient::new()?, "workspace")?;
let multi = MultiSourceFetcher::new(Arc::new(fetcher));
# let _ = multi;
# Ok::<(), pulith_fetch::Error>(())
```

### 4. Register in store and install

```rust
use pulith_install::{InstallReady, InstallSpec, PlannedInstall};

# let ready: InstallReady = todo!();
# let spec: InstallSpec = todo!();
let receipt = PlannedInstall::new(ready, spec)
    .stage()?
    .commit()?
    .finish();
# let _ = receipt;
# Ok::<(), pulith_install::InstallError>(())
```

## Crate Guide

Primitive crates:

- `pulith-version` - version parsing and selection
- `pulith-platform` - OS/shell/environment helpers
- `pulith-fs` - atomic filesystem and workspace primitives
- `pulith-verify` - streaming verification and hash validation
- `pulith-archive` - safe archive extraction
- `pulith-fetch` - transfer execution and receipts
- `pulith-shim` - shim resolution primitives
- `pulith-serde-backend` - serialization backend contract and JSON baseline adapter

Semantic/workflow crates:

- `pulith-resource` - resource identity, locator, version, trust, behavior contract
- `pulith-source` - normalized remote/local source planning
- `pulith-lock` - deterministic lock model and diff
- `pulith-store` - artifact/extract storage and provenance persistence
- `pulith-state` - lifecycle persistence, inspection, repair, retention planning
- `pulith-install` - typed install, activation, backup/restore, rollback workflow

Examples:

- `examples/runtime-manager/` - end-to-end integration example
- `examples/pulith-backend-example/` - thin backend adapter example

## How To Use Pulith Well

- keep policy in your top-level manager, not in core crates
- prefer semantic types (`ResourceId`, `StoreKey`, `InstallInput`) over raw string/path glue
- use typed plan/report outputs before mutation where available
- carry provenance and receipts across transitions instead of reconstructing facts later
- use `FromStr`/`Display` types for normal string boundaries

## Docs

- design overview: `docs/design.md`
- roadmap: `docs/roadmap.md`
- publish status: `docs/publish/overview.md`
- crate-level design notes: `docs/design/*.md`

## Local Development

```bash
just fmt
just clippy
just test
just doc
just ci
```

Project-specific engineering constraints live in `docs/AGENT.md`.

## License

Apache-2.0. See `LICENSE`.
