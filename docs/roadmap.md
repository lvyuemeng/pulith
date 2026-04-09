# Pulith Roadmap

## Goal

Tighten Pulith into a cohesive, efficient, mechanism-first resource-management ecosystem.

The foundational crates now exist. The roadmap is no longer about adding missing layers. It is about making the existing layers integrate cleanly, stay policy-light, and hold up under real end-to-end workloads.

## Current Position

- the architecture is broadly correct
- the crate split is mostly right and should remain
- the main remaining gap is integration quality between crates, especially in advanced flows
- the next work should prioritize typed bridges, end-to-end tests, and performance evidence over adding more crate surface

## Current Priorities

### 1. Tighten Integration Between Existing Crates

- make `pulith-source` feed planned candidates directly into `pulith-fetch`
- make `pulith-fetch` outputs convert cleanly into store and install handoff types
- reduce path-level glue between `pulith-fetch`, `pulith-store`, `pulith-archive`, and `pulith-install`
- standardize receipts and handoff types across pipeline stages

### 2. Harden Advanced Execution Paths

- make `pulith-fetch` advanced modes explicit and trustworthy:
  - retry policy
  - resumable fetch
  - conditional fetch
  - multi-source execution
- add replace / upgrade / rollback semantics to `pulith-install`
- deepen shim-oriented activation through adapters, not embedded policy

### 3. Improve Version-Centric Resource Selection

- extend `pulith-version` with requirement matching
- add preference selection (`latest`, `lts`, exact, compatible, pinned)
- connect those semantics to `pulith-resource` and `pulith-source`

Current progress:

- typed version requirements and preference selection now exist in `pulith-version`
- `pulith-resource` version selectors now use typed requirements instead of raw strings

### 4. Add Integrated Testing

- end-to-end pipeline tests across crates
- cross-platform contract tests
- persistence and recovery tests
- activation idempotence tests
- source/fetch/store/install integration tests

Current progress:

- workspace integration tests now cover local source -> fetch -> store -> install -> activate
- archive extract -> store -> install is now covered end-to-end
- repeated activation switching is now covered at the workflow level

### 5. Add Performance Validation

- benchmark state growth and snapshot rewriting
- benchmark large artifact fetch/extract/install flows
- benchmark advanced fetch strategies under realistic workloads
- measure copy-heavy transitions and reduce them where possible

## Keep / Change Decisions

### Keep Separate

- `pulith-fs`, `pulith-verify`, `pulith-archive`, `pulith-fetch`
- `pulith-resource`, `pulith-store`, `pulith-state`
- `pulith-install`, `pulith-source`

### Improve Without Merging

- `pulith-source` <-> `pulith-fetch`
- `pulith-fetch` <-> `pulith-store`
- `pulith-store` <-> `pulith-install`
- `pulith-install` <-> `pulith-shim`
- `pulith-resource` <-> `pulith-version`

The roadmap assumes integration tightening, not crate collapse.

## Ordered Backlog

### Near-Term

1. Connect `pulith-source` planning directly into `pulith-fetch`.
2. Define shared receipts and handoff types across fetch, store, archive, and install.
3. Add rollback / replace / upgrade semantics to `pulith-install`.
4. Connect `pulith-version` selection semantics more directly into `pulith-source` and install planning.
5. Add end-to-end workspace integration tests.

### Mid-Term

1. Add store lookup, provenance, and pruning without forcing install policy into `pulith-store`.
2. Improve lifecycle persistence ergonomics in `pulith-state`.
3. Add shim-oriented activator adapters for `pulith-install`.
4. Benchmark and optimize copy-heavy pipeline transitions.

Current progress:

- `pulith-store` now supports artifact/extract lookup
- provenance can be persisted in store metadata sidecars
- orphaned metadata can be pruned without binding store layout to install policy
- `pulith-state` now supports higher-level ensure / lookup / patch / lifecycle helpers
- store import and install staging now prefer hardlink-or-copy to reduce redundant file copies
- `pulith-install` now has shim-oriented activation adapters built on `pulith-shim::TargetResolver`

### Later

1. Add thin backend example crates to validate the adapter-first architecture.
2. Revisit state storage structure only if benchmarks show snapshot rewriting is a real bottleneck.
3. Add optional migration / backup / trust-policy extensions once the core pipeline is stable.

Current progress:

- `pulith-backend-example` now demonstrates a thin adapter built on `pulith-resource`, `pulith-source`, and `pulith-install`
- the example backend shapes specs and activators without absorbing fetch/store/state policy into a framework
- `pulith-state` now has a dedicated `state_growth` benchmark for save/update cost across larger snapshots
- `pulith-install` now has optional backup/restore helpers for install roots and matching state facts

## Integrated Test Plan

### End-to-End

- resource -> source -> fetch -> store -> install -> activate
- resource -> source -> fetch -> archive -> store -> install
- reinstall and active-version switching
- interrupted install and recovery

### Cross-Platform

- windows replace and cleanup behavior
- symlink / junction activation behavior
- path and archive sanitization behavior

### Persistence

- repeated state updates through install flows
- restart from partial state
- repeated activation idempotence

### Performance

- large artifact fetch/extract/install
- state growth behavior
- store import and extract registration cost
- advanced fetch strategy overhead vs benefit

## Risks

- broadening the API surface faster than integration quality improves
- keeping advanced fetch modes exposed before they are fully trustworthy
- letting cross-crate glue become path-heavy and ad hoc
- optimizing too early without end-to-end benchmarks

## Success Criteria

Pulith is successful when a caller can compose a full resource-management flow with low glue overhead:

- describe a resource semantically
- plan sources
- fetch and verify bytes
- store and extract artifacts
- install and activate safely
- persist lifecycle state atomically
- recover from interruption or failure

And can do so without adopting a monolithic framework or a rigid package model.
