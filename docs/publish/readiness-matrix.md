# Publish Readiness Matrix

Current source of truth for crates.io publish gating.

## Stage Summary

| Stage | Crates | Gate state | Blocker |
|---|---|---|---|
| 1 | `pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim` | blocked | `pulith-version` must pass clean-worktree crates.io dry-run (latest retry failed on dirty files) |
| 2 | `pulith-resource`, `pulith-platform`, `pulith-archive` | blocked | stage 1 not published + stage-2 crate files currently dirty |
| 3 | `pulith-source`, `pulith-store` | waiting | stage 2 publish not complete |
| 4 | `pulith-fetch`, `pulith-state` | waiting | stage 3 publish not complete |
| 5 | `pulith-install` | waiting | stage 4 publish not complete |

## Stage 1 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-fs` | `0.1.0` | pass | ready |
| `pulith-version` | `0.1.0` | fail (dirty `crates/pulith-version/Cargo.toml`, `crates/pulith-version/src/version.rs`) | blocked |
| `pulith-verify` | `0.1.0` | pass | ready |
| `pulith-shim` | `0.1.0` | pass | ready |

## Stage 2 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-resource` | `0.1.0` | fail (dirty `crates/pulith-resource/Cargo.toml`) | blocked |
| `pulith-platform` | `0.1.0` | fail (dirty `crates/pulith-platform/Cargo.toml`) | blocked |
| `pulith-archive` | `0.2.0` | fail (dirty files under `crates/pulith-archive/*`) | blocked |

## Operational Links

- overview: `docs/publish/overview.md`
- checklist: `docs/publish/checklist.md`
