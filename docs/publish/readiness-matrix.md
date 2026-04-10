# Publish Readiness Matrix

Current source of truth for crates.io publish gating.

## Stage Summary

| Stage | Crates | Gate state | Blocker |
|---|---|---|---|
| 1 | `pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim` | stage-ready | all stage-1 crates have clean-worktree crates.io dry-run pass; awaiting actual publish |
| 2 | `pulith-resource`, `pulith-platform`, `pulith-archive` | blocked | stage 1 crates not published in resolved registry path (`pulith-version`/`pulith-fs` unavailable) |
| 3 | `pulith-source`, `pulith-store` | waiting | stage 2 publish not complete |
| 4 | `pulith-fetch`, `pulith-state` | waiting | stage 3 publish not complete |
| 5 | `pulith-install` | waiting | stage 4 publish not complete |

## Stage 1 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-fs` | `0.1.0` | pass | ready |
| `pulith-version` | `0.1.0` | pass | ready |
| `pulith-verify` | `0.1.0` | pass | ready |
| `pulith-shim` | `0.1.0` | pass | ready |

## Stage 2 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-resource` | `0.1.0` | fail (`pulith-version` not found in resolved registry) | blocked |
| `pulith-platform` | `0.1.0` | fail (`pulith-fs` not found in resolved registry) | blocked |
| `pulith-archive` | `0.2.0` | fail (`pulith-fs` not found in resolved registry) | blocked |

## Operational Links

- overview: `docs/publish/overview.md`
- checklist: `docs/publish/checklist.md`
