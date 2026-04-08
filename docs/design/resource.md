# pulith-resource

Semantic, composable resource description types.

## Purpose

`pulith-resource` defines the shared vocabulary for Phase 2.

It does not fetch, store, or install anything. It only describes:

- resource identity
- where a resource can be located
- how a version is selected
- what verification is required
- how the artifact should be materialized

## Design Rules

- no hard-coded resource categories like tool/plugin/config
- semantic structs and enums where meaning matters
- type aliases for plain bags like labels and metadata
- validated values should carry proof once parsed
- compile-time workflow only where it helps composition

## Core Types

- `ResourceId`
- `ResourceSpec`
- `ResourceLocator`
- `VersionSelector`
- `VerificationRequirement`
- `MaterializationSpec`
- `RequestedResource`
- `ResolvedResource`

## Validation Strategy

`ValidUrl` and `ValidDigest` are proof-carrying validated values.

They are checked once at construction and then reused across crates without repeated ad hoc validation.

## Workflow Shape

`pulith-resource` uses a light type-state pattern:

- `RequestedResource`
- `ResolvedResource`

This keeps compile-time ordering available for higher layers without forcing persistence or transport layers to mirror the same model.
