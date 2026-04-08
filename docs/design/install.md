# pulith-install

Composable installation workflow primitives.

## Purpose

`pulith-install` composes the lower-level Pulith crates into reusable installation flows.

It does not define one global package model. It defines typed workflow steps that callers can use to build install, upgrade, and activation flows with compile-time ordering.

## Design Rules

- type-state for workflow ordering where misuse would be expensive
- semantic handles from `pulith-store`, not raw paths guessed from conventions
- persistent lifecycle updates through `pulith-state`
- activation remains a trait, not a hard-coded backend policy

## Core Types

- `InstallReady`
- `InstallSpec`
- `InstallInput`
- `PlannedInstall`
- `StagedInstall`
- `InstalledInstall`
- `ActivatedInstall`
- `Activator`
- `SymlinkActivator`

## Workflow Shape

```rust
let installed = PlannedInstall::new(ready, spec)
    .stage()?
    .commit()?;

let receipt = installed.activate(&SymlinkActivator)?.finish();
```

Compile-time states:

- planned
- staged
- installed
- activated

This keeps ordering explicit without turning persistence models into compile-time state machines.

## Activation Boundary

Activation is expressed as a trait:

- callers may provide their own activator
- the crate ships a basic `SymlinkActivator`
- future shim-oriented activators can compose on the same interface

## Current Scope

- stage from stored artifact or extracted directory handle
- commit into install root atomically through `pulith-fs::Workspace`
- update persistent state records on install and activation
- record activation history

## Deferred

- richer rollback journals
- upgrade / replace-with-previous semantics
- direct fetch integration
- source resolution orchestration
- deeper shim-specific activation policies
