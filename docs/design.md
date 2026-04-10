# Pulith Design Document

## Vision

Pulith is a mechanism-first Rust ecosystem for resource management primitives.

It exists to let tool authors build reliable resource managers from explicit, composable parts rather than from package-manager-specific script glue.

Pulith is not trying to be a monolithic package manager framework. It is trying to be a trustworthy substrate for managers that need to:

- describe resources semantically
- plan and fetch sources explicitly
- verify and materialize content safely
- install and activate resources predictably
- persist lifecycle state durably
- inspect, reconcile, repair, and retain state without hidden policy

## Scope

Pulith is in scope for:

- resource identity and version intent
- source planning and fetch execution
- verification, storage, extraction, install, and activation
- persistence of lifecycle facts and provenance
- semantic inspection, repair planning, and retention planning
- cross-platform behavior where install and activation semantics differ

Pulith is out of scope for:

- repository hosting
- authentication servers
- license management
- full dependency resolution
- lockfile-driven package graphs
- manager-specific policy such as ranking repositories, upgrade channels, or dependency solving rules

## Design Principles

1. atomicity first
2. semantic APIs over stringly glue
3. pure core, impure edge
4. composition over orchestration
5. crate roles stay narrow and predictable
6. ergonomic helpers remove repetition without hiding policy
7. cross-platform behavior is explicit where semantics differ
8. receipts, records, and reports should make behavior explainable afterward

## User Expectation

If a user builds on Pulith, the public API should read like a resource-management pipeline:

- describe a resource
- derive or plan sources
- fetch and verify content
- register an artifact or extracted tree
- install and optionally activate it
- persist lifecycle facts
- inspect drift, plan repair, and derive retention safely

The important constraint is not just correctness of each crate in isolation. The important constraint is that adjacent crates compose without forcing callers to rebuild path, key, provenance, and record glue by hand.

## Current Architecture

Pulith has four main layers plus support crates and examples.

### Primitive Layer

- `pulith-platform`: platform and environment helpers
- `pulith-version`: version parsing, comparison, matching, and preference
- `pulith-fs`: atomic filesystem and workspace primitives
- `pulith-verify`: verification primitives
- `pulith-archive`: extraction primitives
- `pulith-fetch`: transfer execution primitives
- `pulith-shim`: shim resolution primitives

### Semantic Model Layer

- `pulith-resource`: resource identity, locator, version intent, and selection helpers
- `pulith-store`: artifact/extract registration, metadata, provenance, and lookup
- `pulith-state`: lifecycle facts, inspection, repair planning, and retention-oriented state helpers

### Workflow Layer

- `pulith-install`: typed install, replace, activate, upgrade, and rollback workflow

### Source Layer

- `pulith-source`: source definitions, planning, candidate expansion, and planning strategy

### Support / Example Layer

- `pulith-shim-bin`: thin runtime boundary around shim resolution
- `pulith-backend-example`: thin adapter example
- `examples/runtime-manager/`: practical multi-crate integration example

## Dependency Shape

The dependency shape remains correct and should stay intact.

- primitive crates should remain independently useful
- semantic crates should remain policy-light
- workflow crates should compose lower layers instead of absorbing them
- examples should prove end-to-end ergonomics without redefining the architecture

High-level relationships:

- `pulith-fetch` depends on `pulith-fs` and `pulith-verify`
- `pulith-archive` depends on `pulith-fs`
- `pulith-store` depends on `pulith-fs` and `pulith-resource`
- `pulith-state` depends on `pulith-fs`, `pulith-resource`, and `pulith-store`
- `pulith-install` depends on `pulith-fs`, `pulith-resource`, `pulith-store`, and `pulith-state`
- `pulith-source` depends on `pulith-resource`

The next phase should tighten these handoffs, not merge the crates.

## Current State

### What Is Working

The workspace is no longer missing its basic layers.

What is already materially in place:

- the crate split maps well to resource-management concerns
- semantic resource identity is present and increasingly reused across layers
- source planning, fetch/store handoff, install staging, activation, rollback, and persistence all exist
- provenance now survives more of the fetch -> store -> install path
- install replacement, rollback, and activation semantics are much stronger than earlier iterations
- `pulith-store` and `pulith-state` now expose more semantic helpers instead of forcing repeated caller-side reconstruction
- inspection, repair planning, ownership conflict detection, and retention planning now exist in initial form
- integration tests cover the main install-centered workflows

### Main Design Debt

Pulith's main problem is no longer missing crates. Its main problem is uneven maturity across behavior families and crate boundaries.

The most important debt is:

- some source/fetch/store/install/state handoffs still require caller-side choreography
- some lifecycle transitions still require record-level reconstruction by callers
- discovery, reconciliation, repair, ownership, retention, and explainability exist only in partial form
- user-facing guarantees are still weaker than the API surface suggests
- advanced fetch execution and retry/recovery semantics need a clearer contract story
- state snapshot growth is simple and acceptable for now, but long-term scaling remains an evidence question

## What Pulith Must Be Strong At

Pulith should converge on five first-class behavior families.

### 1. Install and Activation

This is the most mature behavior family today.

Pulith should support:

- install of file-like and directory-like resources
- explicit activation policy choices
- replace, upgrade, rollback, and interruption recovery
- clear cross-platform activation failure surfaces

### 2. Discovery and Inspection

This behavior family exists only partially today.

Pulith should support:

- detect already-installed resources
- inspect store, install, activation, and state facts without mutation
- compare persisted facts with filesystem and store reality
- produce explainable issue reports rather than opaque booleans

### 3. Reconciliation and Repair

This behavior family has initial helpers but is not yet mature enough.

Pulith should support:

- detect drift in install paths, activation targets, metadata, and store references
- plan repair explicitly before applying it
- reapply or restore state without inventing hidden manager policy
- preserve recoverability and explainability across repair flows

### 4. Ownership, Conflict, and Retention

This behavior family is still under-modeled.

Pulith should support:

- explicit ownership of install roots, activation targets, and stored material
- collision detection before destructive replacement or cleanup
- protected prune planning derived from semantic state
- retention planning that explains why something is protected or removable

### 5. Explainability and Auditability

This behavior family is still incomplete.

Pulith should support:

- explain why a source or candidate was selected
- explain why an install or activation target is current or stale
- keep provenance durable across workflow boundaries
- keep enough lifecycle evidence to explain cleanup, repair, or rollback decisions

## Required Guarantees

Pulith should be explicit about the guarantees it does and does not provide.

### Behavioral Guarantees

- repeatable baseline fetch -> store -> install -> activate flows, with advanced fetch retry/source strategies treated as maturing contracts
- explicit replace, upgrade, and rollback behavior within the install snapshot boundaries exposed by `pulith-install`
- explicit inspect and repair planning surfaces
- semantic continuity from resource identity to persisted lifecycle facts

### Safety Guarantees

- partial writes should not become live state where atomic replacement is possible
- recovery paths should restore both filesystem state and persisted lifecycle facts when a supported snapshot/backup path is used
- destructive cleanup should require explicit ownership or protection reasoning
- platform-specific activation limits should appear as explicit errors, not hidden fallback behavior

### Consistency Guarantees

- resource identity should remain the backbone of planning, storage, install, and state
- version intent should survive planning and install handoffs
- provenance should survive workflow boundaries instead of being dropped mid-pipeline
- adjacent crates should compose without ad hoc translation layers

### Non-Guarantees

Pulith should not imply support for:

- dependency solving
- lockfile-driven reproducibility for dependency graphs
- manager-specific policy such as channel ranking or repository trust models
- hidden automatic repair or cleanup decisions
- global rollback journals beyond per-resource snapshot/backup scope
- stronger advanced fetch retry/resume guarantees than the current `pulith-fetch` maturity level documents

## Where The Missing Work Is

The remaining work is concentrated in seven areas.

### 1. Semantic Handoffs

Remaining repeated orchestration should move into typed helpers.

Key examples:

- planned source -> fetch input
- fetch receipt -> store registration
- store lookup -> install input
- resolved resource -> lifecycle record / query helpers

### 2. Discovery Maturity

Inspection needs to become a clear public behavior family rather than a small set of lower-level helpers.

Needed outputs:

- stable inspection/report types
- issue categories that are durable enough for managers to act on
- clear separation between inspect-only and repair-capable flows

### 3. Repair Contract Hardening

Repair must remain explicit and policy-light.

Needed outputs:

- repair plans that are inspectable before application
- clear statement of what is safe to retry
- clearer boundaries between repair of stale facts and repair of owned materialized state

### 4. Ownership and Retention Semantics

Retention helpers exist, but broader ownership semantics are still incomplete.

Needed outputs:

- ownership models for install roots and activation targets
- conflict reports that prevent ambiguous destructive replacement
- composable protected-prune plans that reference semantic state and provenance

### 5. Explainability

Receipts and records exist, but they do not yet form a complete explanation story.

Needed outputs:

- selection explanations for source/version choice
- activation and repair explanations tied to state facts
- clearer durable evidence for operator-facing tools

### 6. Contract Testing

Coverage is strongest for install-centered flows but still incomplete.

Needed outputs:

- more Windows-specific activation contract tests
- more reconciliation and ownership tests
- more cross-crate end-to-end examples that exercise the intended public API shape

### 7. Performance Evidence

Performance direction is reasonable, but several decisions still rely on provisional evidence.

Needed outputs:

- steadier benchmark runs for hardlink/copy thresholds
- larger-state benchmarks for `pulith-state`
- evidence-driven decisions before redesigning storage internals or fetch strategy behavior

## Concrete Design Direction

The next design phase should follow this shape.

### Phase 1: Finish Semantic Query and Handoff Helpers

Goal: remove the remaining repeated key/path/record glue between crates.

Deliverables:

- stable helper families for resource -> store lookup and resource -> state lookup
- stable receipt-to-registration helpers across fetch, archive, store, and install
- less manual reconstruction in examples and backend adapters

### Phase 2: Promote Discovery To A First-Class Public Surface

Goal: make inspect/detect/report behavior explicit and reusable.

Deliverables:

- stable inspection report types in `pulith-state`
- public helpers for store/install/activation/state drift inspection
- tests that prove inspect-only paths do not mutate state

### Phase 3: Mature Repair, Ownership, and Retention

Goal: make corrective behavior explicit without hiding policy.

Deliverables:

- inspectable repair plans
- ownership and conflict report types
- retention/protection plans that explain why content is protected or removable

### Phase 4: Tighten Planner and Adapter Integration

Goal: make real manager composition feel direct.

Deliverables:

- broader use of shared version-selection helpers
- thinner source -> fetch composition for common planner paths
- stronger example/back-end coverage that demonstrates intended usage rather than internal crate knowledge

### Phase 5: Harden Contracts and Guarantees

Goal: make Pulith trustworthy because its contracts are tested and documented, not because the implementation looks careful.

Deliverables:

- explicit docs for retry, rollback, recovery, and platform divergence
- cross-platform contract tests for remaining activation differences
- documented non-guarantees where Pulith intentionally leaves policy to callers

## Architectural Conclusion

Pulith does not need a new top-level architecture.

It needs tighter semantics at crate boundaries, stronger manager-facing behavior families outside the install path, and clearer guarantees about inspection, repair, ownership, retention, and explanation.

The current crate split is still the right one. The next work should refine composition, contracts, and evidence inside that split.

## References

- `README.md`
- `docs/AGENT.md`
- `docs/roadmap.md`
- `docs/design/platform.md`
- `docs/design/version.md`
- `docs/design/shim.md`
- `docs/design/fs.md`
- `docs/design/verify.md`
- `docs/design/archive.md`
- `docs/design/fetch.md`
- `docs/design/resource.md`
- `docs/design/store.md`
- `docs/design/state.md`
- `docs/design/install.md`
- `docs/design/source.md`
