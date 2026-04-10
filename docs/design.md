# Pulith Design

## Vision

Pulith is a mechanism-first Rust ecosystem for resource-management primitives.
It enables manager authors to compose explicit building blocks without hidden framework policy.

## Scope

In scope:

- resource identity and version intent
- source planning, fetch, verify, extract, install, activate
- lifecycle persistence, inspect, repair planning, ownership/retention planning
- explicit cross-platform behavior for install/activation differences

Out of scope:

- dependency solving and lockfile graph solving
- repository hosting/auth/authz services
- manager policy (ranking, trust, channels, hidden auto-repair/cleanup)

## Architecture

Crate roles:

- Primitive: `pulith-platform`, `pulith-version`, `pulith-fs`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-shim`
- Semantic: `pulith-resource`, `pulith-source`, `pulith-store`, `pulith-state`
- Workflow: `pulith-install`
- Internal adapter/examples: `pulith-backend-example`, `pulith-shim-bin`, `examples/runtime-manager/`

Design boundary:

- keep crate roles narrow and composable
- use typed receipts/reports over ad hoc reconstruction
- add helper APIs that remove glue without embedding manager policy

## Composition Contract

Expected pipeline:

1. describe resource semantics
2. plan/derive sources
3. fetch and verify material
4. register/store or extract
5. install and optionally activate
6. persist lifecycle facts
7. inspect drift and apply explicit repair plans

Required property: adjacent crates compose without manual key/path/provenance glue.

Filesystem boundary:

- core crates use `pulith-fs` when atomic/transactional/cross-platform fs guarantees matter
- top-level examples may use `std::fs` for orchestration glue

## Guarantees and Non-Guarantees

Guaranteed (test-backed):

- repeatable baseline source->fetch->store->install->activate flows
- explicit replace/upgrade/rollback behavior within `pulith-install` snapshot boundaries
- explicit inspect and repair-plan surfaces
- lifecycle continuity from resource identity into persisted state facts
- platform-specific activation limitations surfaced as typed errors

Not guaranteed:

- dependency solving or lockfile-grade graph reproducibility
- global rollback journals beyond per-resource backup/snapshot scope
- stronger fetch retry/resume semantics than current `pulith-fetch` contract/tests
- automatic policy decisions for ranking/trust/cleanup
- archive decompression resource-limit protection is not guaranteed unless callers opt in via extraction limit options

## API Stability Decisions

Dispatch strategy:

- generic-first for in-process composition extension points (`Activator`, `TargetResolver`, source adapters)
- dyn-capable boundaries for runtime I/O substitution (HTTP/stream plumbing)
- no generic<->dyn flip on stabilized public APIs without explicit semver review

Error boundary strategy:

- one public error enum per crate
- wrap direct dependency errors via source-bearing variants (`#[from]`/`#[source]`)
- keep crate-specific contract errors explicit at each boundary

## Publish Intent

Public-target crates:

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-install`, `pulith-platform`, `pulith-shim`

Internal/non-publish crates:

- `pulith-backend-example` (reference adapter crate)
- `pulith-shim-bin` (internal binary wrapper)
- `runtime-manager-example` (integration example)

## Version Management Strategy

Pulith uses an independent crate versioning model with release-train coordination.

- public crates keep semver-independent versions so low-level crates can ship fixes without forcing lockstep bumps
- releases are executed in dependency order (bottom-up publish train) so downstream crates can reference published versions cleanly
- path dependencies in workspace manifests include explicit version requirements to keep publish manifests valid
- internal/non-publish crates (`publish = false`) are excluded from publish-train version pressure

Practical implication:

- do not require every public crate to pass crates.io dry-run simultaneously before first publish
- instead, require staged dry-run/publish evidence by dependency layer

## Current Readiness Snapshot

Strong:

- install replacement/rollback/activation contracts
- inspect/repair/ownership/retention surfaces
- end-to-end integration coverage in `workspace_pipeline` tests
- archive traversal/symlink escape protections are test-backed

Remaining before first publish wave:

- crates.io-direct dry-run path (independent of local mirror replacement)
- final publish-readiness matrix and verification log per target crate
- continued corpus/property expansion for version edge cases
- tune and document default resource-limit recommendations for zip-bomb resistance (API support is now test-backed)

## References

- `docs/roadmap.md`
- `docs/AGENT.md`
- `docs/design/archive.md`
- `docs/design/fetch.md`
- `docs/design/install.md`
- `docs/design/state.md`
- `docs/design/source.md`
