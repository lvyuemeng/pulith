# Pulith Roadmap

## Goal

Make Pulith a trustworthy, mechanism-first substrate for real resource managers by improving semantic composition, operational coverage, and contract clarity without collapsing crate boundaries or embedding manager policy.

## Current Assessment

Pulith is past the phase where it lacked major layers.

The current gap is not "missing another crate." The current gap is that some of the important manager-facing behavior families are only partially modeled or only partially integrated.

What is already in good shape:

- install-centered flows are materially stronger than before
- fetch/store/install/state handoffs are improving
- rollback and activation contracts are much clearer
- provenance continuity is stronger across common paths
- store/state semantic helpers exist in initial form
- inspection, repair planning, conflict detection, and retention planning now exist, but are not yet mature enough to be considered finished

## Roadmap Themes

All near-term work should align with five themes.

### 1. Semantic Composition

- remove repeated caller-side key/path/record reconstruction
- standardize typed handoffs across source, fetch, store, install, and state
- keep helpers explicit about effects and policy

### 2. Operational Coverage Beyond Install

- promote inspect/detect/reconcile/repair to first-class behavior families
- model ownership/conflict/retention before cleanup semantics grow wider
- keep these behaviors outside a monolithic install-centric abstraction

### 3. Contract Clarity

- document what is safe to retry
- document what rollback and recovery restore
- document where Windows and Unix semantics intentionally differ
- document what Pulith does not guarantee

### 4. Evidence and Testing

- grow end-to-end examples and integration tests
- add reconciliation and ownership contract coverage
- continue cross-platform activation coverage
- keep benchmark-driven decisions evidence-based

### 5. Thin Adapter Story

- make real manager composition easier through typed helpers and examples
- avoid turning adapters into policy-heavy convenience layers

## Concrete Plan

### Milestone 1 - Finish Semantic Handoffs

Objective:

- reduce the remaining manual glue between adjacent crates

Work items:

- finish `ResolvedResource`-centric lookup helpers in `pulith-store` and `pulith-state`
- standardize fetch/archive/store/install handoff helpers around stable receipt-oriented types
- remove repeated path/file-name/provenance reconstruction from examples and internal integration tests
- review crate APIs for helper naming consistency: `plan`, `derive`, `lookup`, `register`, `record`, `activate`, `inspect`, `repair`

Done when:

- common end-to-end flows can be expressed with substantially less path/key glue
- examples no longer reconstruct store/state transitions manually for normal flows

### Milestone 2 - Make Discovery A First-Class Public Surface (completed: initial stable surface)

Objective:

- turn inspection from an emerging helper set into a stable public behavior family

Work items:

- define durable inspection report types for resource, install, activation, and store state
- separate inspect-only APIs from repair-capable APIs clearly in naming and types
- ensure issue reports are stable enough for operator-facing tools to render directly
- add end-to-end tests for inspect-only flows that verify no mutation occurs

Done when:

- a caller can inspect current state and drift without reconstructing low-level comparisons or worrying about accidental mutation

### Milestone 3 - Mature Repair, Ownership, and Retention (in progress)

Objective:

- make corrective and cleanup behavior explicit, inspectable, and safe to compose

Work items:

- define inspectable repair plan types with explicit apply steps
- extend ownership/conflict reporting for install roots, activation targets, and stored material
- extend protected prune planning with state-backed explanations
- ensure destructive cleanup paths require explicit ownership or protection reasoning
- add reconciliation and ownership integration tests covering ambiguous or conflicting cases

Done when:

- cleanup and repair flows are explainable and policy-light rather than open-coded per manager

### Milestone 4 - Tighten Planner and Adapter Integration

Objective:

- make Pulith feel direct to use for real managers without adding a framework layer

Work items:

- thread shared version-selection policy into more planner and adapter paths
- add thin source -> fetch helpers for common planned-source cases
- improve `examples/runtime-manager/` so it exercises inspect, repair, and retention paths in addition to install flows
- add one more example path that demonstrates a manager-like reconcile/apply cycle

Done when:

- example and adapter code reflects intended public usage rather than internal crate knowledge

### Milestone 5 - Harden Contracts and Publish Guarantees

Objective:

- make Pulith's guarantees explicit and test-backed

Work items:

- document retry, rollback, recovery, ownership, and platform-difference guarantees in crate docs and top-level docs
- add remaining Windows-specific activation contract tests
- add more persistence and recovery tests around interrupted or partially stale state
- rerun performance benchmarks on steadier environments before changing thresholds or storage internals
- document explicit non-goals and non-guarantees to avoid overclaiming

Done when:

- docs, tests, and behavior tell the same story about what Pulith guarantees and what it intentionally leaves to callers

## Priority Order

The recommended execution order is:

1. semantic handoffs
2. discovery surface
3. repair, ownership, and retention
4. planner/adapter integration
5. contract hardening and evidence

This order keeps the work aligned with the actual missing features: better composition first, then stronger operational behavior, then stronger guarantees.

## Immediate Next Block

The next implementation block should be small and concrete.

### Progress Checklist

- [x] Block A - Discovery Surface Stabilization
- [x] Block B - Ownership / Retention Hardening
- [x] Block C - Recovery Contract Publication (completed)
- [x] Block D - Milestone 4 Planner/Adapter Integration Kickoff (completed)
- [x] Block E - Milestone 4 Broader Adapter Coverage (completed)
- [ ] Block F - Milestone 5 Hardening Kickoff (active)

### Block C execution checklist

- [x] retry/rollback guarantee docs
- [x] cross-platform recovery/activation tests
- [x] non-guarantees alignment

### Block D execution checklist

- [x] planner policy threading in adapter-facing paths
- [x] thin planned-source -> fetch helper pass
- [x] runtime-manager reconcile/apply example loop (inspect + repair + retention)
- [x] one policy-light manager cycle test/example assertion path
- [x] docs/examples alignment pass for intended public usage

### Block E execution checklist

- [x] additional non-archive adapter helper path
- [x] runtime-manager non-archive end-to-end command
- [x] focused adapter test for fetch -> install glue reduction
- [x] docs/usage alignment for the new path
- [x] validation pass for backend + example crates

Latest completed:

- Block E broader adapter coverage is complete with a non-archive adapter path, runtime-manager command coverage, and focused adapter tests
- Block D kickoff is complete with planner policy threading, planned-source fetch helpers, and a manager-like reconcile/apply example loop
- Block B is complete and ownership/retention planning is now composable
- canonical inspection report types are now the primary inspection surface
- inspect vs repair API split is explicit and stable
- inspect-only contract tests verify state inspection remains non-mutating
- `examples/runtime-manager` inspect output now renders canonical report types
- ownership conflict and reasoned retention plans are now composable as one inspect-only planning surface
- Block C contract docs and recovery/activation tests are now aligned with crate behavior

### Block A - Discovery Surface Stabilization

Recommended first block:

- define canonical inspection report structs in `pulith-state`
- split inspect-only helpers from repair helpers in public API shape
- add integration tests for resource drift inspection across install path, activation target, and store metadata
- update `examples/runtime-manager/` to print inspection results directly from those report types

Why first:

- this is the clearest still-missing manager-facing capability
- it forces cleaner boundaries between read-only inspection and corrective behavior
- it improves explainability without adding policy-heavy abstractions

### Block B - Ownership / Retention Hardening

Immediately after Block A:

- formalize ownership report types
- extend protected prune plans with explicit reasons
- add tests for ambiguous activation ownership and protected cleanup cases

### Block C - Recovery Contract Publication

Immediately after Block B:

- document retry and rollback guarantees
- add missing cross-platform recovery/activation tests
- align crate docs with actual tested guarantees

### Block D - Milestone 4 Planner/Adapter Integration Kickoff

Immediately after Block C:

- thread shared version-selection policy through remaining planner/adapter paths
- extend `examples/runtime-manager/` to exercise repair and retention plan/apply loops
- add one manager-like reconcile/apply cycle example that stays policy-light

Execution sequence for this kickoff:

1. thread shared version-selection policy through `pulith-source` planner entry points used by adapters/examples
2. add/normalize thin planned-source -> fetch helper(s) so examples avoid manual candidate conversion glue
3. extend `examples/runtime-manager/` with an explicit inspect -> plan -> apply/reconcile cycle that remains policy-light
4. add contract-focused tests or deterministic assertions for the new adapter/example flow
5. close Block D checklist and move to broader Milestone 4 integration once the example reflects intended public composition

### Block E - Milestone 4 Broader Adapter Coverage

Immediately after Block D kickoff:

- expand planner/adapter helpers into one additional non-archive manager path
- add one more end-to-end example that uses the same inspect/repair/retention composition style
- add focused tests to verify the added adapter path does not reintroduce manual key/path glue

Execution sequence for this block:

1. add a thin non-archive adapter helper in `pulith-backend-example` for fetch-receipt -> install-spec handoff
2. wire `examples/runtime-manager/` to expose that non-archive path as a first-class command
3. add focused tests proving the helper keeps path/key conversion policy-light and deterministic
4. align example docs/usage text with the new command and flow
5. close Block E and advance to Milestone 5 hardening kickoff once validation gates pass

### Block F - Milestone 5 Hardening Kickoff

Immediately after Block E:

- extend Windows-specific activation contract coverage for remaining edge paths
- add interrupted/stale-state persistence recovery tests beyond current snapshot restore checks
- stage benchmark reruns for steadier threshold/storage evidence before behavior changes

## Risks

- widening API surface faster than semantics become clear
- hiding policy inside convenience helpers
- letting `pulith-install` absorb discovery or reconciliation responsibilities that belong elsewhere
- shipping repair or cleanup behavior before ownership/conflict semantics are explicit enough
- implying guarantees that are not yet backed by tests and durable state behavior

## Success Criteria

Pulith succeeds when a caller can:

- describe a resource semantically
- derive sources and execute fetch/store/install flows with low glue overhead
- inspect current state and filesystem/store drift without mutation
- plan and apply repair explicitly
- explain ownership, cleanup, and retention decisions from durable facts
- rely on documented, tested guarantees for rollback, recovery, and activation behavior

And do so without adopting a monolithic package-manager framework or a rigid package model.
