# Pulith Roadmap

## Goal

Make Pulith a trustworthy, mechanism-first substrate for real resource managers through stronger composition, clearer contracts, and test-backed behavior.

## Current Stage

- Milestone 3: repair/ownership/retention maturation (in progress)
- Milestone 4: planner/adapter integration (completed)
- Milestone 5: contract and publish hardening (active)

Block status:

- [x] Block A - Discovery Surface Stabilization
- [x] Block B - Ownership / Retention Hardening
- [x] Block C - Recovery Contract Publication
- [x] Block D - Milestone 4 Planner/Adapter Integration Kickoff
- [x] Block E - Milestone 4 Broader Adapter Coverage
- [x] Block F - Milestone 5 Hardening Kickoff
- [x] Block G - Milestone 5 Evidence Expansion
- [x] Block H - API Stabilization Evidence
- [ ] Block I - Pre-Publish Environment and Metadata Hardening (active)

## What Is Already Done

- cross-platform CI/doc warning enforcement and contract-focused test expansion
- recovery/rollback/activation guarantees tightened and documented
- archive traversal/symlink-escape protections test-backed
- manager-like reconcile/apply integration tests added
- version parser edge-case corpus improved
- publish intent split finalized (public vs internal crates)
- package metadata normalized for public-target crates

Evidence:

- benchmark notes: `docs/benchmarks/block-g-2026-04.md`
- publish dry-run evidence: `docs/publish/block-h-2026-04.md`

## Immediate Block (Block I)

Execution checklist:

- [ ] establish crates.io-direct dry-run environment (independent from mirror replacement)
- [ ] rerun publish dry-runs for public-target crates with both mirror path and crates.io path recorded
- [x] add one compact publish-readiness matrix (crate -> publish intent -> metadata -> dry-run status)

Exit criteria:

- crates.io-targeted dry-run evidence exists for every public-target crate
- mirror-based and crates.io-based dry-run outcomes are both documented
- publish-readiness matrix is checked in and linked from this roadmap

Readiness matrix:

- `docs/publish/readiness-matrix.md`

## Public vs Internal Split

Public-target crates:

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-install`, `pulith-platform`, `pulith-shim`

Internal/non-publish crates:

- `pulith-backend-example`
- `pulith-shim-bin`
- `runtime-manager-example`

## Decisions from External Review

Accepted and already acted on:

- cross-shell CI/doc enforcement
- archive safety contract testing (traversal/absolute path/symlink escape)
- stronger end-to-end integration evidence
- error-boundary guidance for cross-crate wrapping

Partially accepted (ongoing):

- expand version parser corpus/property confidence
- keep dispatch decisions explicit before API freeze
- keep shim invocation-time resolution guarantees explicit

## Risks

- publish confidence blocked by environment-specific registry replacement behavior
- over-claiming guarantees beyond current tests
- accidental policy leakage into helper crates

## Success Criteria

Pulith is ready for first publish wave when:

- public-target crates pass documented crates.io-targeted dry-runs
- docs, tests, and behavior align on guarantees/non-guarantees
- cross-platform contract behavior remains explicit and test-backed
