# Pulith Roadmap

## Goal

Tighten Pulith into a trustworthy, mechanism-first resource-management ecosystem.

The crate split is already in place. The remaining work is to make the composed system easier to build on, safer to operate, and clearer about its guarantees.

## Current Position

- the crate boundaries are broadly correct and should stay intact
- the main gap is no longer missing primitives; it is semantic ergonomics, operational coverage, and consistency across crates
- install-oriented behavior is comparatively mature; non-install operational behavior is still under-modeled
- the next work should favor reusable semantic helpers, stronger safety guarantees, integrated tests, and measured performance over more raw surface area

## Stable Ground

- resource -> source -> fetch -> store -> install flows now have much less path/key glue
- provenance survives more workflow boundaries, including fetch -> store and fetched-archive -> store
- install/upgrade/rollback/activation behavior is materially stronger than before
- `pulith-store` and `pulith-state` now expose more semantic helpers instead of forcing repeated caller-side reconstruction
- `pulith-store` now supports orphaned-metadata inspection before pruning, making cleanup paths easier to preview and safer to explain
- `pulith-state` now supports capture/restore of per-resource state snapshots, which makes recovery flows easier to compose and less reliant on workflow-local restoration logic
- `pulith-state` now also supports per-resource inspection against filesystem/store reality, giving managers an initial detect/explain path without forcing that behavior into `pulith-install`
- `pulith-state` now also supports explicit per-resource repair planning/application for stale state facts, giving managers an initial reconcile path without hiding policy in install flows
- `pulith-state` now also supports activation ownership conflict detection, giving managers an initial conflict signal before broader cleanup semantics are added
- `pulith-state` now exposes store-key references and `pulith-store` now supports protected prune planning, giving managers an initial retention/prune safety story instead of only blind orphan cleanup
- `pulith-state` now also exposes lifecycle-based store retention helpers, which makes protected prune behavior composable from semantic state instead of requiring ad hoc key lists
- `pulith-state` now also composes store orphan inspection with lifecycle-based retention into explicit metadata retention plans, reducing cleanup planning glue for manager authors
- `pulith-resource` and `pulith-backend-example` now thread version-selection policy into real candidate selection helpers, reducing adapter-side preference glue
- workspace integration coverage now includes repeated copy-based activation over the same file target, strengthening cross-platform activator contract coverage for non-link activation paths
- workspace integration coverage now also includes archive fetch/extract/store/install/activate/rollback recovery, strengthening archive-inclusive recovery guarantees instead of only install-from-directory recovery
- workspace integration coverage now also includes repeated symlink-based file activation over the same target, balancing link-based and copy-based file activation contract coverage
- workspace integration tests already cover the main install-centered flows

## What Real Managers Need

Pulith should be strong enough to support:

- system package managers
- config managers
- plugin managers
- runtime/tool installers

Those users need four things:

- behavior: predictable fetch/store/install/activate/rollback plus inspect/reconcile/repair paths
- safety: atomic replacement, explicit verification/trust, recoverable interruption, clear platform-specific failures
- consistency: aligned identity, provenance, version intent, and lifecycle facts across crates
- ergonomics: common flows should not require repeated path/key/record reconstruction

## Current Priorities

### 1. Semantic Ergonomics

- keep reducing repeated query and transition glue
- standardize helpers where callers still reconstruct records, keys, or lifecycle updates manually
- keep helpers explicit about effects and policy

### 2. Safety and Recovery

- strengthen retry, rollback, interruption recovery, and activation-failure guarantees
- keep provenance and lifecycle facts durable enough to explain and recover state
- make platform-specific constraints visible rather than implicit

### 3. Operational Behavior Beyond Install

- add inspect, detect, reconcile, repair, and explain paths as first-class behaviors
- add ownership/conflict/retention semantics before cleanup/prune behavior grows wider
- avoid absorbing all operational behavior into `pulith-install`

### 4. Planner and Adapter Integration

- thread `VersionSelector` -> `SelectionPolicy` into more real planner/backend paths
- keep adapters thin and reusable

### 5. Contract and Evidence

- add cross-platform contract coverage where semantics still differ
- keep benchmarking advanced fetch, copy/hardlink thresholds, and state growth before redesigning internals

## Phase Plan

### Phase A - State and Store Ergonomics

- continue semantic lifecycle/state helpers
- continue semantic store lookup/provenance/pruning helpers
- remove remaining repeated record/key reconstruction where clearly shared

Current Phase A progress:

- semantic state upsert/patch helpers are now in place
- semantic store lookup helpers are now in place
- orphaned metadata can now be inspected before prune, reducing hidden cleanup behavior

### Phase B - Recovery and Consistency

- make persistence/recovery helpers easier to compose
- tighten provenance continuity and lifecycle consistency across fetch/store/install/state
- clarify what is durable, recoverable, and safe to retry

Current Phase B progress:

- per-resource state capture/restore helpers now exist in `pulith-state`
- install rollback now reuses semantic state restoration instead of open-coding record and activation recovery
- recovery composition is becoming more explicit, though broader retry/reconciliation semantics still remain

### Phase C - Operational Behaviors

- design inspect/detect/reconcile/repair helpers
- establish ownership/conflict/retention semantics
- make explainability explicit through receipts, records, and state facts

Current Phase C progress:

- initial inspect/detect semantics now exist through per-resource state inspection
- missing install paths, activation targets, store entries, and store metadata can now be reported as explicit issues
- explicit repair planning/application now exists for stale install paths, store references, and activation records
- activation ownership conflicts can now be detected and reported explicitly
- initial retention/prune safety now exists through store-key reference inspection and protected prune planning
- lifecycle-based retention selection now exists for state-driven protected prune behavior
- explicit state-driven metadata retention planning now exists on top of those lower-level helpers
- broader retention policy semantics still remain for follow-up blocks

### Phase D - Planner and Adapter Integration

- extend version-selection semantics into real adapters/planners
- add thin helpers only where they remove real repeated orchestration

Current Phase D progress:

- resources can now select preferred resolved candidates through shared version-selection policy
- the backend example now exposes that same selection helper for real adapter paths
- broader planner integration beyond the example layer still remains for follow-up blocks

### Phase E - Contract Hardening and Performance

- add the remaining Windows/cross-platform contract tests
- keep archive-inclusive and recovery-inclusive integration coverage growing
- rerun benchmarks on steadier environments before changing behavior thresholds or storage architecture

Current Phase E progress:

- repeated copy-based activation is now covered in workspace integration tests, not only symlink/junction-backed activation paths
- cross-platform activator semantics are becoming more explicit through both link-based and copy-based coverage
- archive-inclusive replacement and rollback recovery is now covered end-to-end, not only directory-based replacement flows
- repeated symlink-based file activation is now also covered, not just directory symlink and copy-file replacement paths
- broader Windows-specific contract gaps still remain for follow-up blocks

## Near-Term Plan

1. keep improving `pulith-store` provenance/pruning ergonomics where cleanup still needs manual inspection logic
2. extend persistence/recovery helpers and tests where retry/reconcile behavior still needs workflow-local glue
3. extend Phase C from lifecycle-based retention planning into broader retention policy semantics without collapsing it into install
4. thread version-selection policy into additional planner/backend paths beyond the current resource/example helpers
5. add more Windows-specific contract coverage where activator behavior still differs beyond the current copy/link integration coverage

## Risks

- widening APIs faster than integration quality improves
- hiding policy or effects inside convenience helpers
- letting install-centric abstractions absorb unrelated operational behaviors
- claiming safety guarantees before tests and persistence behavior truly support them
- tuning performance heuristics without stable benchmark evidence

## Success Criteria

Pulith succeeds when callers can:

- describe a resource semantically
- derive and plan sources directly
- fetch, verify, store, extract, install, and activate with low glue overhead
- persist lifecycle state atomically enough to recover from interruption
- explain what happened through durable provenance and lifecycle facts
- detect, reconcile, repair, and safely clean up drift without hidden policy

And do so without adopting a monolithic framework or rigid package model.
