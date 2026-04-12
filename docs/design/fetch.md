# pulith-fetch

HTTP fetching primitives for Pulith.

## Purpose

`pulith-fetch` is the network-facing layer of the Pulith ecosystem. It is responsible for moving remote bytes into local storage safely while staying composable with `pulith-fs` and `pulith-verify`.

The crate is still mechanism-first. It should expose reusable fetching components, not a policy-heavy downloader framework.

## Current Module Structure

```text
pulith-fetch
  cache/      file and HTTP cache helpers
  codec/      checksum, decompression, signature helpers
  config/     fetch and source option types
  fetch/      fetcher strategies
  net/        HTTP client abstraction and protocol helpers
  perf/       performance helpers
  progress/   progress reporting
  rate/       backoff, throttling, bandwidth limiting
  segment/    segmented download helpers
```

## Core Public Surface

```rust
use pulith_fetch::{
    Fetcher, FetchOptions, FetchPhase, Progress,
    HttpClient, ReqwestClient,
    ConditionalFetcher, ResumableFetcher, MultiSourceFetcher, SegmentedFetcher, BatchFetcher,
    DownloadSource, MultiSourceOptions, SourceSelectionStrategy,
    TokenBucket, ThrottledStream, retry_delay,
};
```

## Stable Intent

The most dependable path today is the base single-source fetch flow:

```rust
let client = ReqwestClient::new()?;
let fetcher = Fetcher::new(client, temp_root);

let receipt = fetcher
    .fetch_with_receipt(url, destination, FetchOptions::default())
    .await?;
```

This flow is the baseline Pulith fetch contract:

- streaming download from an abstract `HttpClient`
- optional verification hooks
- atomic placement through `pulith-fs::Workspace`
- progress callbacks through explicit caller-provided hooks
- typed `FetchReceipt` handoff for higher workflow layers

Runtime coupling boundary:

- retry waiting behavior can be injected through `FetchOptions::retry_delay_provider(...)`
- default delay behavior remains available when no provider is supplied
- public fetch contracts avoid runtime-handle coupling
- batch concurrency path avoids runtime-specific task spawn APIs in core execution flow

Advanced retry/resume behavior outside this baseline should be treated as maturing and must not be interpreted as a stronger reliability contract than current tests document.

## Advanced Fetchers

The crate also exposes higher-level fetchers:

- `ConditionalFetcher`
- `ResumableFetcher`
- `MultiSourceFetcher`
- `SegmentedFetcher`
- `BatchFetcher`

These are part of the intended design, but they should currently be treated as maturing APIs rather than fully proven policy engines.

`MultiSourceFetcher` now also accepts planned candidates from `pulith-source`, so source planning can stay in `pulith-source` while transfer execution stays in `pulith-fetch`.

## Design Boundaries

`pulith-fetch` should own:

- HTTP transfer orchestration
- retry and backoff primitives
- source selection mechanics
- resumable transfer mechanics
- conditional request mechanics
- progress and throttling helpers

`pulith-fetch` should not own:

- package semantics
- installation layout policy
- version resolution policy
- activation logic
- long-term registry/state schema

`pulith-fetch` should consume planned source candidates, not become the planning layer itself.

Those responsibilities belong in higher-level crates built later.

## Current Maturity Assessment

Working baseline:

- `Result<T>` and public API exports are consistent with the rest of Pulith
- module structure is aligned with the current codebase
- strict clippy on `--all-targets --all-features` is now realistic
- base fetch flow, progress plumbing, throttling helpers, and checksum-related utilities are usable
- source planning can now feed directly into multi-source fetch execution
- fetch execution can now hand a typed receipt to higher layers instead of only returning a path

Still maturing:

- retry policy should become a clearer execution model instead of a loose option bag
- multi-source selection needs stronger guarantees around planning, consistency, and concurrent destination safety
- resumable and conditional fetching need more explicit persistence and recovery semantics
- transport and source policy boundaries should become clearer over time

## Phase 1 Outcome

Phase 1 for `pulith-fetch` focuses on stabilization rather than feature expansion.

Completed in the current phase:

- codebase and docs now reflect the current module layout
- library and extra targets build cleanly under strict linting
- public API exports are coherent and consistent
- the crate is documented as a maturing primitive layer, not a finished downloader framework

Deferred to later phases:

- stronger retry model
- stronger multi-source planning and verification
- more complete resumable / conditional semantics
- higher-level resource and install composition crates

## Relationship to Other Crates

- depends on `pulith-fs` for atomic placement and staging
- depends on `pulith-verify` for verification primitives
- feeds future crates such as `pulith-store`, `pulith-resource`, and `pulith-install`

## Design Direction

The next design step is not to add more unrelated fetch features. It is to make the existing advanced paths explicit, trustworthy, and easy to compose into a larger resource-management pipeline.
