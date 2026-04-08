# Pulith Roadmap

## Current State

- Workspace crates in `crates/` are the active codebase; `archive/` contains legacy non-workspace code.
- The project direction in `docs/design.md` is coherent, but several design docs currently overstate implementation completeness.
- The main release blockers are correctness issues in `pulith-archive`, CI-blocking lint failures in tests, formatting drift, and incomplete cross-platform CI coverage.

## Priority Plan

### P0 - Correctness and Security

1. Fix `pulith-archive` extraction hashing so hashes are computed from actual extracted file contents.
2. Apply symlink target sanitization during archive extraction instead of writing raw archive targets.
3. Fix Windows-specific workspace commit failure in `pulith-archive` tests.

### P1 - CI Baseline

1. Remove `clippy -D warnings` failures in `pulith-fs` and `pulith-platform` tests.
2. Apply `cargo fmt` across the workspace.
3. Replace the current CI workflow with a stricter workflow that checks formatting, clippy, docs, tests, and dependency policy.

### P2 - Cross-Platform Confidence

1. Add a GitHub Actions OS matrix for Linux, Windows, and macOS.
2. Add an MSRV check aligned with Rust 2024 edition support and then reconcile `docs/AGENT.md`.
3. Run tests with `--all-features` so optional code paths are covered.

### P3 - Design / Implementation Alignment

1. Reconcile `docs/design/fs.md` with the actual `pulith-fs` API.
2. Reconcile `docs/design/fetch.md` with the current modular `pulith-fetch` codebase.
3. Review stale dependencies and partial implementations in `pulith-fetch` and `pulith-platform`.

## Execution Notes

- Apply fixes in priority order so the repository reaches a stable CI baseline before broader feature work.
- Prefer targeted fixes that improve correctness first, then enforce them through CI.
- Keep the docs updated as APIs and workflows are corrected.
