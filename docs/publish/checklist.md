# Publish Checklist

Use this checklist as the single operational runbook.

## Latest Attempt (2026-04-10)

- attempted `cargo publish -p pulith-version --dry-run --registry crates-io`
- result: fail due dirty files `crates/pulith-version/Cargo.toml` and `crates/pulith-version/src/version.rs`
- next executable step: clear those changes (or commit them) and rerun the same command

## Stage 1 Gate Clearance

- [ ] ensure clean worktree for `crates/pulith-version/*`
- [ ] run `cargo publish -p pulith-version --dry-run --registry crates-io` and record pass
- [ ] confirm stage-1 crates all clean-worktree pass: `pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim`
- [ ] set stage-1 decision to go in this file and `docs/publish/readiness-matrix.md`

## Stage 1 Publish (After Gate Go)

- [ ] publish `pulith-fs` `0.1.0`
- [ ] publish `pulith-version` `0.1.0`
- [ ] publish `pulith-verify` `0.1.0`
- [ ] publish `pulith-shim` `0.1.0`

## Stage 2 Dry-Run + Publish

- [ ] ensure clean worktree for `crates/pulith-resource/*`, `crates/pulith-platform/*`, `crates/pulith-archive/*`
- [ ] run `cargo publish -p pulith-resource --dry-run --registry crates-io`
- [ ] run `cargo publish -p pulith-platform --dry-run --registry crates-io`
- [ ] run `cargo publish -p pulith-archive --dry-run --registry crates-io`
- [ ] publish in order: `pulith-resource` `0.1.0` -> `pulith-platform` `0.1.0` -> `pulith-archive` `0.2.0`

## Stage 3-5 Progression

- [ ] repeat clean-worktree dry-run gate per stage from `docs/publish/readiness-matrix.md`
- [ ] publish stage crates only after prior stage is published

## Evidence Update Rules

- [ ] update `docs/publish/readiness-matrix.md` immediately after each dry-run/publish event
- [ ] keep this checklist synchronized with the active blocker and next executable step
