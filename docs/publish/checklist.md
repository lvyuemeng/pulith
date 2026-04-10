# Publish Checklist

Use this checklist as the single operational runbook.

## Latest Attempt (2026-04-10 20:19 +08:00)

- reran stage-2 dry-runs with crates.io-direct cargo home (`CARGO_HOME=.tmp/cargo-home`) and all passed
- attempted real stage-2 publish with default config and it failed due `ustc` replacement not resolving `pulith-version`
- attempted real stage-2 publish with crates.io-direct cargo home and it failed due missing publish token (`cargo login` required)
- blocker class: publish environment mismatch (mirror replacement vs crates.io-direct auth)
- next executable step: run stage-2 publish in crates.io-direct environment with valid token

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
- [ ] publish in order: `pulith-resource` `0.1.0` -> `pulith-platform` `0.1.0` -> `pulith-archive` `0.2.0` (blocked by environment/auth setup)

## Stage 3-5 Progression

- [ ] repeat clean-worktree dry-run gate per stage from `docs/publish/readiness-matrix.md`
- [ ] publish stage crates only after prior stage is published

## Evidence Update Rules

- [ ] update `docs/publish/readiness-matrix.md` immediately after each dry-run/publish event
- [ ] keep this checklist synchronized with the active blocker and next executable step
