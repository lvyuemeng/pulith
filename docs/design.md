# Pulith Design

## Vision

Pulith is a mechanism-first Rust crate ecosystem for building resource managers.
It provides composable primitives and semantic workflow contracts, without embedding manager policy.

Design priorities:

- semantic APIs over path/string glue
- explicit effects and typed boundaries
- composable crates over monolithic framework behavior
- deterministic, test-backed contracts
- cross-platform behavior as a first-class constraint

## Scope

In scope:

- resource identity/version/trust semantics
- source planning, fetch, verify, extract, store, install, activate
- lifecycle persistence, inspection, repair planning, retention planning

Out of scope (for core crates):

- dependency graph solving and lock orchestration
- repository hosting/auth/authz systems
- hidden ranking/trust/channel/cleanup policy

## Canonical Pipeline

`resource -> source plan -> fetch -> verify -> extract/register -> install -> activate -> state`

Required property: adjacent crates compose through typed method/trait absorption with minimal manual glue.

## Resource Taxonomy and Essential Behaviors

Pulith must support diverse resource classes (binaries, runtimes, packages, plugins, config/secret, images, services) without rigid type explosion.

Core rule: design around **essential behaviors**, not one enum per resource class.

Essential behavior axes:

- materialization shape: single file, extracted tree, layered/object set
- activation model: none, path target, shim resolution, service registration, environment projection
- mutation scope: install root only vs install root + external extension steps
- provenance requirement: source/verifier continuity and explainability
- integrity model: hash/signature/attestation requirements
- lifecycle requirement: replace/rollback/uninstall/repair expectations

Implication: a runtime, SDK, plugin, or service can share core pipeline primitives while varying by behavior configuration and extension steps.

## Crate Roles

- Primitive: `pulith-platform`, `pulith-version`, `pulith-fs`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-shim`, `pulith-serde-backend`
- Semantic: `pulith-resource`, `pulith-source`, `pulith-store`, `pulith-state`, `pulith-lock`
- Workflow: `pulith-install`
- Adapter/examples: `examples/pulith-backend-example`, `examples/runtime-manager`

## API Unification Strategy

Pulith standardizes on **method + trait pipeline composition**.

Rules:

- prefer trait absorption (`Into*Registration`, `Into*Input`) over free conversion helpers
- prefer crate-owned methods for semantic composition (provenance/report shaping)
- keep one canonical boundary path per crate role; remove compatibility aliases after migration
- keep policy out of helpers; helpers convert facts only

Applied boundary model:

- `pulith-store`
  - owns provenance composition semantics
  - registration APIs absorb fetch/archive evidence via trait inputs
- `pulith-install`
  - input API is materialized and transport-agnostic (`StagedFile`, `StoredArtifact`, `ExtractedArtifact`, `ExtractedTree`)
  - fetch/archive receipts do not cross install boundary
  - planning/uninstall capability surfaces should prefer typed dispositions/capability enums over bare booleans
  - internal workspace/staging machinery should not leak as external composition vocabulary
- `pulith-state`
  - single inspection model (`ResourceInspectionReport` + `ResourceInspectionFinding`)
  - no dual legacy/new shape drift
- `pulith-source`
  - remote-source families should converge on shared composition vocabulary instead of overlapping top-level type trees

## Lifecycle Receipt Model

Lifecycle outputs use a unified envelope:

- context: resource, phase, install root, activation target, replacement flag, timestamp
- payload: phase-specific details

Install lifecycle envelope types:

- `LifecycleOperationPhase`
- `LifecycleOperationDetails`
- `LifecycleOperationReceipt`

## Installation Variant Contract

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
- non-filesystem side effects (registry/service/env) modeled as caller extension stages around core install flow

## Crate Boundary Checklist (Normative)

Any new feature must satisfy:

- no policy in primitives (`pulith-fetch` takes `RetryPolicy`; does not choose strategy)
- no hidden side effects (mutating functions are explicit in API flow and receipts)
- error boundaries use source-wrapping; avoid re-enumerating deep leaf variants upstream
- async hot paths are instrumented (`#[tracing::instrument]`) with useful fields
- core filesystem mutation uses `pulith-fs` atomic/workspace primitives
- archive extraction uses path-contained safety model (no unsafe unpack semantics)
- progress/event surfaces are opt-in
- state mutation paths remain recoverable and explicit

## Efficiency and Adaptability Decisions

Pulith optimizes for explicit correctness first, then performance through bounded mechanisms:

- retry and backoff are explicit policy inputs
- copy/link thresholds are explicit and benchmark-driven
- extraction limits are explicit (entry/byte caps)
- fetch/extract/install boundaries keep deterministic, inspectable receipts

Initial stabilization decisions are recorded in `docs/design/stabilization.md`.

## Cross-Crate Invariants

- provenance continuity: installed bytes remain explainable
- explicit mutation scope: each crate mutates only its contract boundary
- deterministic retries/plans/inspections
- explicit typed fallback and limitation reasons

Current design pressure points to resolve next:

- `pulith-install` should continue reducing receipt/state duplication by preferring typed lifecycle/state payload reuse over bespoke backup/report structs
- `pulith-source` still has overlapping remote source families (direct URL, mirror, git) that need a more unified remote-source model
- option-heavy record/config shapes should continue moving toward typed state transitions, capability enums, and crate-owned helper methods
- parsing/formatting contracts should prefer `FromStr`/`Display` where string boundaries are part of normal composition

Extension invariant:

- `pulith-install` remains install-root focused; external side-effect orchestration composes as caller-owned pipeline stages

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
- reliability gate: mutation-path changes include negative-path coverage
- performance gate: hot-path changes run benchmark or strict validation
- policy gate: no hidden strategy logic in primitives/semantics/workflow crates

## References

- `docs/roadmap.md`
- `docs/AGENT.md`
- `docs/design/install.md`
- `docs/design/lock.md`
- `docs/design/serialization.md`
- `docs/design/stabilization.md`
- `docs/design/store.md`
- `docs/design/state.md`
