# Pulith Roadmap

## Goal

Tighten Pulith into a cohesive, efficient, mechanism-first resource-management ecosystem.

The core crate split is in place. The remaining work is integration maturity, trustworthy advanced paths, and performance evidence.

## Current Position

- the architecture is broadly correct
- the crate boundaries should stay intact
- the main gap is still cross-crate ergonomics and integration depth
- the next work should favor typed bridges, integrated tests, and measured performance over more surface area

## What Is Already In Place

- `pulith-source` plans candidates that `pulith-fetch` can execute
- fetch -> store -> install handoff now has typed helpers with less manual path glue
- source specs can be derived directly from requested and resolved resources
- install staging now validates resolved versions against exact and requirement selectors
- workspace tests cover local fetch/store/install/activate, archive extract/install, activation switching, and interrupted install recovery
- install and fetch now have criterion benchmarks for end-to-end pipeline and multi-source strategy behavior
- store/import/install now use a measured size-threshold staging strategy for copy vs hardlink-or-copy

## Active Priorities

### 1. Tighten Typed Bridges

- reduce remaining ad hoc handoff code between `pulith-source`, `pulith-fetch`, `pulith-store`, and `pulith-install`
- standardize more pipeline receipts and handoff types where callers still reconstruct records manually
- keep improving end-to-end helpers without collapsing crate boundaries

### 2. Harden Advanced Paths

- keep `pulith-fetch` advanced modes explicit and trustworthy
- extend replace / upgrade / rollback semantics where install lifecycle edges remain awkward
- keep shim-oriented activation in adapters rather than embedding policy

### 3. Deepen Version-Aware Planning

- carry `pulith-version` intent more directly through source planning and install decisions
- tighten alias and preference handling where exact / requirement validation is already in place

### 4. Expand Integrated Verification

- add more cross-platform contract coverage
- keep strengthening persistence, recovery, and repeated lifecycle flow tests
- prefer end-to-end integration coverage over isolated surface growth

### 5. Keep Benchmark-Guided Optimization

- continue benchmarking advanced fetch strategies under realistic contention/failure scenarios
- refine copy-transition tuning on steadier CI runners before changing thresholds
- benchmark state growth and any remaining copy-heavy transitions before redesigning storage behavior

## Ordered Backlog

### Near-Term

1. Reduce the remaining source -> fetch planning glue with direct typed helpers.
2. Define more shared receipts and handoff types across fetch, store, archive, and install.
3. Extend install lifecycle ergonomics around replace / upgrade / rollback paths.
4. Carry version preference semantics more directly into planning and selection flows.
5. Add more workspace-level integration and cross-platform contract tests.

### Mid-Term

1. Keep improving store lookup, provenance, and pruning without absorbing install policy.
2. Keep improving lifecycle persistence ergonomics in `pulith-state`.
3. Expand shim-oriented activator adapters.
4. Re-run and compare performance tuning on quieter CI runners.

### Later

1. Keep validating the adapter-first architecture through thin backend examples.
2. Revisit state storage only if benchmarks show snapshot rewriting is a real bottleneck.
3. Add optional migration, backup, and trust-policy extensions only after the core pipeline is stable.

## Test Focus

- end-to-end resource -> source -> fetch -> store -> install -> activate flows
- archive extract -> store -> install flows
- reinstall, activation switching, and interrupted recovery flows
- windows replace behavior, symlink/junction behavior, and path/archive sanitization
- state growth and advanced fetch strategy cost under realistic workloads

## Risks

- widening APIs faster than integration quality improves
- keeping advanced fetch modes exposed beyond their validated guarantees
- letting caller glue drift back toward path-heavy manual orchestration
- tuning performance heuristics without stable benchmark evidence

## Success Criteria

Pulith succeeds when callers can compose a full resource-management flow with low glue overhead:

- describe a resource semantically
- derive and plan sources directly
- fetch and verify bytes
- store and extract artifacts
- install and activate safely
- persist lifecycle state atomically
- recover from interruption or failure

And do so without adopting a monolithic framework or rigid package model.
