# Publish Overview

## Goal

Ship public Pulith crates to crates.io using dependency-order publish stages with reproducible dry-run evidence.

## Scope

Public crates:

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-install`, `pulith-platform`, `pulith-shim`

Internal/non-publish crates:

- `pulith-backend-example`, `pulith-shim-bin`, `runtime-manager-example`

## Stages

1. Stage 1: `pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim`
2. Stage 2: `pulith-resource`, `pulith-platform`, `pulith-archive`
3. Stage 3: `pulith-source`, `pulith-store`
4. Stage 4: `pulith-fetch`, `pulith-state`
5. Stage 5: `pulith-install`

## Current Status

- stage 1 gate: stage-ready (all stage-1 crates have clean-worktree crates.io dry-run pass)
- stage 2 dry-runs now reach dependency gating and fail on missing published stage-1 crates in resolved registry path
- stage-1 actual publish is pending and is the next release-order gate
- canonical detailed status: `docs/publish/readiness-matrix.md`
- active operational checklist: `docs/publish/checklist.md`
