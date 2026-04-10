# Pulith Roadmap

## Goal

Make Pulith a trustworthy, mechanism-first substrate for real resource managers through stronger composition, explicit contracts, and test-backed behavior.

## Current Stage

- Milestone 5: contract and publish hardening (active)

Block status:

- [x] Block A-K completed (stabilization, integration, API evidence, metadata hardening)
- [x] Block L - Clean-Worktree Publish Decisioning (completed)
- [ ] Block M - Stage 1 Gate Clearance and Publish Execution (active)

## Current Reality

- publish train definition and readiness matrix are in place
- stage 1 crates.io dry-runs were executed without `--allow-dirty`
- current stage-1 decision is **no-go** because `pulith-version` was dirty during dry-run validation
- latest `pulith-version` stage-1 retry still fails on dirty crate files (gate unchanged)
- publish docs are compacted into overview/checklist/matrix and aligned with current gates

Evidence:

- `docs/publish/readiness-matrix.md`
- `docs/publish/overview.md`
- `docs/publish/checklist.md`

## Active Block (Block M)

Execution checklist:

- [ ] clear stage-1 gate by re-running `pulith-version` crates.io dry-run from clean worktree (no `--allow-dirty`)
- [ ] update stage summary in matrix/checklist from blocked to stage-ready after clean rerun passes
- [x] execute stage-2 dry-runs in dependency order and record blocker state in readiness matrix
- [x] compact publish docs and remove outdated per-block evidence files
- [x] proceed next phase by retrying stage-1 gate command and recording current blocker

Exit criteria:

- stage 1 is marked go with explicit clean-worktree evidence
- stage-2 crates have crates.io dry-run outcomes recorded against exact versions
- docs remain internally consistent across overview, checklist, and matrix

## Publish Scope

Public-target crates:

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-install`, `pulith-platform`, `pulith-shim`

Internal/non-publish crates:

- `pulith-backend-example`
- `pulith-shim-bin`
- `runtime-manager-example`

## Release Criteria

Pulith is ready for first publish wave when:

- stage-by-stage crates.io dry-runs pass in documented dependency order
- stage decisions (go/no-go) are evidence-backed and checked in
- docs, tests, and runtime behavior stay aligned on guarantees and boundaries
