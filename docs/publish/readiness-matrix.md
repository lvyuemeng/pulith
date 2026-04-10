# Publish Readiness Matrix

## Scope

This matrix tracks Block I publish readiness for public-target crates.

Public targets:

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-install`, `pulith-platform`, `pulith-shim`

Internal/non-publish:

- `pulith-backend-example`, `pulith-shim-bin`, `runtime-manager-example`

## Status

| Crate | Publish intent | Metadata normalized | Mirror dry-run evidence | crates.io-targeted dry-run |
|---|---|---|---|---|
| `pulith-fs` | public | yes | blocked by source replacement | pending |
| `pulith-version` | public | yes | blocked by source replacement | pending |
| `pulith-resource` | public | yes | blocked by source replacement | pending |
| `pulith-source` | public | yes | blocked by source replacement | pending |
| `pulith-verify` | public | yes | blocked by source replacement | pending |
| `pulith-archive` | public | yes | blocked by source replacement | pending |
| `pulith-fetch` | public | yes | blocked by source replacement | pending |
| `pulith-store` | public | yes | blocked by source replacement | pending |
| `pulith-state` | public | yes | blocked by source replacement | pending |
| `pulith-install` | public | yes | blocked by source replacement | pending |
| `pulith-platform` | public | yes | blocked by source replacement | pending |
| `pulith-shim` | public | yes | blocked by source replacement | pending |

## Notes

- source replacement (`crates-io` -> `ustc`) in this environment blocks direct crates.io dry-run verification.
- prior dry-run attempt evidence is captured in `docs/publish/block-h-2026-04.md`.
- final release gate requires rerun in a crates.io-direct environment and updating this matrix.
