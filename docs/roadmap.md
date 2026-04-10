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
- stage-1 clean-worktree dry-run gate is now satisfied for all stage-1 crates
- current publish blocker is stage-2 actual publish environment/auth split after stage-1 publish
- publish docs are compacted into overview/checklist/matrix and aligned with current gates

Evidence:

- `docs/publish/readiness-matrix.md`
- `docs/publish/overview.md`
- `docs/publish/checklist.md`

## Active Block (Block M)

Execution checklist:

- [x] clear stage-1 gate by re-running `pulith-version` crates.io dry-run from clean worktree (no `--allow-dirty`)
- [x] update stage summary in matrix/checklist from blocked to stage-ready after clean rerun passes
- [x] execute stage-2 dry-runs in dependency order and record blocker state in readiness matrix
- [x] compact publish docs and remove outdated per-block evidence files
- [x] proceed next phase by retrying stage-1 gate command and recording current blocker
- [x] clear stage-2 crate dirty-state blocker and re-run stage-2 dry-runs to confirm dependency-order gating
- [x] execute stage-1 actual publish for all stage-1 crates
- [x] retry stage-2 dry-runs and record registry-resolution blocker
- [x] validate stage-2 dry-runs in crates.io-direct context and classify actual publish blocker

Exit criteria:

- stage 1 is marked stage-ready with explicit clean-worktree evidence
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
