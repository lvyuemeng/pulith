# Publish Overview

## Goal

Ship public Pulith crates to crates.io using dependency-order publish stages with reproducible dry-run evidence.

## Scope

Public crates:

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-lock`, `pulith-install`, `pulith-platform`, `pulith-shim`

Internal/non-publish crates:

- `pulith-backend-example`, `pulith-shim-bin`, `runtime-manager-example`

## Stages

1. Stage 1: `pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim`
2. Stage 2: `pulith-resource`, `pulith-platform`, `pulith-archive`
3. Stage 3: `pulith-source`, `pulith-store`
4. Stage 4: `pulith-fetch`, `pulith-state`
5. Stage 5: `pulith-install`

## Current Status

- stage 1: published (`pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim` at `0.1.0`)
- stage 2 dry-runs pass in crates.io-direct context
- stage 2 actual publish is blocked by environment/auth split (`ustc` replacement cannot resolve deps; crates.io-direct context lacks token)
- canonical detailed status: `docs/publish/readiness-matrix.md`
- active operational checklist: `docs/publish/checklist.md`
