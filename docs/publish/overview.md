# Publish Overview

## Goal

Ship public Pulith crates to crates.io using dependency-order publish stages with reproducible dry-run evidence.

This document now tracks the published baseline and the dependency order to reuse for the next version wave.

## Scope

Public crates:

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-lock`, `pulith-install`, `pulith-platform`, `pulith-shim`, `pulith-serde-backend`

Internal/non-publish crates:

- `pulith-backend-example`, `runtime-manager-example`

## Stages

1. Stage 1: `pulith-fs`, `pulith-version`, `pulith-verify`, `pulith-shim`
2. Stage 2: `pulith-resource`, `pulith-platform`, `pulith-archive`
3. Stage 3: `pulith-serde-backend`, `pulith-source`
4. Stage 4: `pulith-fetch`, `pulith-lock`, `pulith-store`
5. Stage 5: `pulith-state`
6. Stage 6: `pulith-install`

## Current Status

- stages 1-6 are now published on crates.io in dependency order
- this session published: `pulith-serde-backend 0.1.0`, `pulith-lock 0.1.0`, `pulith-source 0.1.0`, `pulith-archive 0.2.0`, `pulith-verify 0.2.0`, `pulith-fetch 0.2.0`, `pulith-store 0.1.0`, `pulith-state 0.1.0`, `pulith-install 0.1.0`
- readiness focus now shifts from environment/auth blockers to maintaining version/dependency coherence for the next release wave
- canonical detailed status: `docs/publish/readiness-matrix.md`
- active operational checklist: `docs/publish/checklist.md`
