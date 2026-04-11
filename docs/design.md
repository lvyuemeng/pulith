# Pulith Design

## Vision

Pulith is a mechanism-first Rust crate ecosystem for building resource managers.
It provides reusable primitives and semantic workflow contracts; it does not embed manager policy.

Design priorities:

- semantic APIs over raw path/string glue
- explicit effects and typed boundaries
- composable crates over monolithic framework behavior
- deterministic, test-backed contracts
- cross-platform behavior as a first-class constraint

## Scope

In scope:

- resource identity/version/trust semantics
- source planning, fetch, verify, extract, store, install, activate
- lifecycle persistence, inspection, repair planning, retention planning

Out of scope:

- dependency graph solving and lock orchestration (for now)
- repository hosting/auth/authz systems
- hidden ranking/trust/channel/cleanup policy

## Canonical Pipeline

`resource -> source plan -> fetch -> verify -> extract/register -> install -> activate -> state`

Required property: adjacent crates compose through typed method/trait absorption with minimal manual glue.

## Resource Taxonomy Contract

Pulith must support a broad resource spectrum without collapsing semantics into one artifact model:

- binaries (single, bundled, sidecar, platform-specific)
- runtimes/toolchains/SDKs
- system/language packages
- plugins (dynamic/script/protocol/asset)
- configuration/secret/env resources
- container/rootfs/OCI resources
- service/daemon resources

Design implication: API boundaries must carry identity, provenance, activation, and lifecycle facts without assuming one install shape.

## Crate Roles

- Primitive: `pulith-platform`, `pulith-version`, `pulith-fs`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-shim`
- Semantic: `pulith-resource`, `pulith-source`, `pulith-store`, `pulith-state`
- Workflow: `pulith-install`
- Adapter/examples: `pulith-backend-example`, `pulith-shim-bin`, `examples/runtime-manager`

## API Unification Strategy

Pulith standardizes on **method + trait pipeline composition**.

Rules:

- prefer trait absorption (`Into*Registration`, `Into*Input`) over free conversion helpers
- prefer crate-owned methods for semantic composition (for example provenance, report shaping)
- keep one canonical boundary path per crate role; remove compatibility aliases after migration
- keep policy out of helpers; helpers convert facts only

Applied boundary model:

- `pulith-store`
  - owns provenance composition semantics
  - registration APIs absorb fetch/archive evidence via trait inputs
- `pulith-install`
  - input API is materialized and transport-agnostic (`StagedFile`, `StoredArtifact`, `ExtractedArtifact`, `ExtractedTree`)
  - fetch/archive receipts do not cross install boundary
- `pulith-state`
  - single inspection model (`ResourceInspectionReport` + `ResourceInspectionFinding`)
  - no dual legacy/new shape drift

## Lifecycle Receipt Model

Lifecycle outputs use a unified envelope:

- context: resource, phase, install root, activation target, replacement flag, timestamp
- payload: phase-specific details

Install lifecycle envelope types:

- `LifecycleOperationPhase`
- `LifecycleOperationDetails`
- `LifecycleOperationReceipt`

This keeps receipts composable and audit-friendly while preserving phase-specific detail records.

## Installation Variant Contract (Block Q)

First-class variants:

- direct local artifact install
- mirrored/air-gapped fetch+store+install
- pre-staged store install
- scoped user/system install
- replace/rollback install
- uninstall/reinstall repair

Variant requirements:

- explicit capabilities (offline, writable roots, activation support, rollback expectation)
- preview/read-only planning where feasible
- provenance + receipt continuity across transitions
- caller-declared fallback choreography (no hidden downgrade)
- non-filesystem side effects (registry/service/env) are modeled as caller extension steps around install pipeline, not rigid core enums

## Cross-Crate Invariants

- provenance continuity: installed bytes remain explainable
- explicit mutation scope: each crate mutates only its own contract boundary
- deterministic retries for stage/inspect/plan operations
- explicit fallback/downgrade reasons (typed, visible)

Extension invariant:

- `pulith-install` remains filesystem/install-root focused; external side-effect orchestration composes as caller-owned pipeline stages

## Security and Integrity Baseline

- checksum verification before extraction/use
- archive safety (path traversal and escape protection)
- typed failure surfaces for verification and activation limits
- no silent fallback across trust/integrity boundaries

## Observability Baseline

- structured instrumentation in mutating/hot-path workflows
- machine-readable plan/report outputs
- contextual error chains across crate boundaries

## Quality Gates

- API gate: boundary changes require updated example + integration test
- composition gate: runtime example must show reduced manual glue
- reliability gate: each mutation-path feature includes negative-path coverage
- performance gate: touched hot paths must run benchmark or strict validation
- policy gate: no hidden strategy logic in primitives/semantics/workflow crates

## Current Open Design Decisions

- lock model introduction (`pulith-lock`) and scope (single-resource vs graph lock)
- fetch transport expansion (HTTP baseline, then S3/OCI/git/ssh)
- archive format expansion (`tar.xz`, `tar.zst`) with safety fixtures
- state backend evolution (JSON/SQLite abstraction)
- shim resolution hot-path and project-context activation semantics

## References

- `docs/roadmap.md`
- `docs/AGENT.md`
- `docs/design/install.md`
- `docs/design/store.md`
- `docs/design/state.md`
