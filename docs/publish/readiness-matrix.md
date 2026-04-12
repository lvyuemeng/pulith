# Publish Readiness Matrix

Current source of truth for crates.io publish gating.

## Stage Summary

| Stage | Crates | Gate state | Blocker |
|---|---|---|---|
| 1 | `pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim` | published | published to crates.io (`pulith-verify` now also at `0.2.0`) |
| 2 | `pulith-resource`, `pulith-platform`, `pulith-archive` | published | stage published (`pulith-archive` at `0.2.0`) |
| 3 | `pulith-serde-backend`, `pulith-source` | published | stage published (`0.1.0`) |
| 4 | `pulith-fetch`, `pulith-lock`, `pulith-store` | published | stage published (`pulith-fetch 0.2.0`, `pulith-lock 0.1.0`, `pulith-store 0.1.0`) |
| 5 | `pulith-state` | published | stage published (`0.1.0`) |
| 6 | `pulith-install` | published | stage published (`0.1.0`) |

## Stage 1 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-fs` | `0.1.0` | pass | ready |
| `pulith-version` | `0.1.0` | pass | ready |
| `pulith-verify` | `0.2.0` | pass | published |
| `pulith-shim` | `0.1.0` | pass | ready |

## Stage 2 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-resource` | `0.1.0` | pass | published |
| `pulith-platform` | `0.1.0` | pass | published |
| `pulith-archive` | `0.2.0` | pass | published |

## Stage 3 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-serde-backend` | `0.1.0` | pass | published |
| `pulith-source` | `0.1.0` | pass | published |

## Stage 4 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-fetch` | `0.2.0` | pass | published |
| `pulith-lock` | `0.1.0` | pass | published |
| `pulith-store` | `0.1.0` | pass | published |

## Stage 5 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-state` | `0.1.0` | pass | published |

## Stage 6 Detail

| Crate | Version | Last crates.io dry-run | Status |
|---|---|---|---|
| `pulith-install` | `0.1.0` | pass | published |

## Operational Links

- overview: `docs/publish/overview.md`
- checklist: `docs/publish/checklist.md`
