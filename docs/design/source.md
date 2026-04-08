# pulith-source

Composable source abstractions and planning.

## Purpose

`pulith-source` widens Pulith beyond one-off locators. It models where a resource can come from and how a caller wants candidate sources to be planned.

It does not fetch data. It describes and expands candidate sources for later layers such as `pulith-fetch` and `pulith-install`.

## Design Rules

- semantic source definitions, not manager-specific package models
- type-state only for the source planning step
- keep planning separate from transfer policy
- allow custom source adapters without making the crate a framework

## Core Types

- `SourceDefinition`
- `SourceSet`
- `SourceSpec`
- `PlannedSources`
- `ResolvedSourceCandidate`
- `SelectionStrategy`
- `SourceAdapter`

## Supported Source Shapes

- direct HTTP release assets
- mirror sets
- local files and directories
- git references

## Planning Model

`pulith-source` uses a small type-state boundary:

- `SourceSpec` for declared sources
- `PlannedSources` for strategy-aware candidate lists

This keeps the ordering explicit without baking transfer or caching behavior into the crate.

## Backend Philosophy

Backend patterns should remain thin adapters over:

- `pulith-resource` for resource semantics
- `pulith-source` for source planning
- `pulith-fetch` for transfer
- `pulith-store` for material storage
- `pulith-install` for activation workflows

Examples of future thin backends:

- version-manager backend
- plugin-manager backend
- config-manager backend
- artifact-cache backend

The source layer stays policy-light so those adapters can differ without forking the core model.
