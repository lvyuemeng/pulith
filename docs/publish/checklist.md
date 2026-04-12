# Publish Checklist

Use this checklist as the single operational runbook.

## Latest Attempt (2026-04-12 10:38 +08:00)

- published `pulith-serde-backend 0.1.0`
- published `pulith-lock 0.1.0`
- published `pulith-source 0.1.0`
- published `pulith-archive 0.2.0`
- published `pulith-verify 0.2.0`
- published `pulith-fetch 0.2.0`
- published `pulith-store 0.1.0`
- published `pulith-state 0.1.0`
- published `pulith-install 0.1.0`
- corrected publish-readiness gaps discovered during verification:
  - made `pulith-serde-backend` and `pulith-lock` publishable
  - updated `pulith-fetch` to depend on `pulith-verify 0.2.0`
  - removed `pulith-install` reliance on unpublished `pulith-fs` staging API for crates.io verification
- current blocker class: none for listed public crates in current release wave
- next executable step: commit manifest/docs updates and maintain next-version readiness matrix from this new baseline

## Stage 1 Gate Clearance

- [x] ensure clean worktree for `crates/pulith-version/*`
- [x] run `cargo publish -p pulith-version --dry-run --registry crates-io` and record pass
- [x] confirm stage-1 crates all clean-worktree pass: `pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim`
- [x] set stage-1 decision to go in this file and `docs/publish/readiness-matrix.md`

## Stage 1 Publish (After Gate Go)

- [x] publish `pulith-fs` `0.1.0`
- [x] publish `pulith-version` `0.1.0`
- [x] publish `pulith-verify` `0.1.0`
- [x] publish `pulith-shim` `0.1.0`

## Stage 2 Dry-Run + Publish

- [x] ensure clean worktree for `crates/pulith-resource/*`, `crates/pulith-platform/*`, `crates/pulith-archive/*`
- [x] run `cargo publish -p pulith-resource --dry-run --registry crates-io`
- [x] run `cargo publish -p pulith-platform --dry-run --registry crates-io`
- [x] run `cargo publish -p pulith-archive --dry-run --registry crates-io`
- [x] publish in order: `pulith-resource` `0.1.0` -> `pulith-platform` `0.1.0` -> `pulith-archive` `0.2.0`

## Stage 3-6 Progression

- [x] stage 3 dry-run gate: `pulith-serde-backend`, `pulith-source`
- [x] stage 4 dry-run gate: `pulith-fetch`, `pulith-lock`, `pulith-store`
- [x] stage 5 dry-run gate: `pulith-state`
- [x] stage 6 dry-run gate: `pulith-install`
- [x] publish stage crates only after prior stage is published

## Evidence Update Rules

- [x] update `docs/publish/readiness-matrix.md` immediately after each dry-run/publish event
- [x] keep this checklist synchronized with the active blocker and next executable step
