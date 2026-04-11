# pulith-resource

Semantic, composable resource description types.

## Purpose

`pulith-resource` defines the shared vocabulary for Phase 2.

It does not fetch, store, or install anything. It only describes:

- resource identity
- where a resource can be located
- how a version is selected
- what verification is required
- what trust policy should apply
- the resource behavior contract (materialization, activation, mutation scope, provenance, lifecycle)

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
- `TrustPolicy`
- `MaterializationSpec`
- `ActivationModel`
- `MutationScope`
- `ProvenanceRequirement`
- `LifecycleRequirements`
- `ResourceBehaviorContract`
- `RequestedResource`
- `ResolvedResource`

## Validation Strategy

`ValidUrl` and `ValidDigest` are proof-carrying validated values.

They are checked once at construction and then reused across crates without repeated ad hoc validation.

## Trust Policy

`pulith-resource` now includes an optional trust policy description layer.

This is intentionally lightweight:

- trust anchors can be based on digest, host, or metadata
- trust evaluation is descriptive and local
- the crate does not become a full trust framework or PKI system

## Workflow Shape

`pulith-resource` uses a light type-state pattern:

- `RequestedResource`
- `ResolvedResource`

This keeps compile-time ordering available for higher layers without forcing persistence or transport layers to mirror the same model.

## Behavior Axis Contract

`ResourceSpec` carries a behavior-axis contract that can be read through `behavior_contract()`.

The contract is explicit and composable:

- materialization shape stays in `MaterializationSpec`
- activation model is typed (`ActivationModel`)
- mutation scope is typed (`MutationScope`)
- provenance continuity expectation is typed (`ProvenanceRequirement`)
- lifecycle expectations are explicit booleans (`LifecycleRequirements`)

This keeps taxonomy flexible (runtime, plugin, service, SDK) while preserving deterministic behavior boundaries.
