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
- `InstallMode`
- `RollbackReceipt`
- `Activator`
- `SymlinkActivator`
- `CopyFileActivator`
- `InstalledShimResolver`
- `ShimCommand`
- `ShimLinkActivator`
- `ShimCopyActivator`

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
- the crate also ships explicit copy-based file activators for privilege-sensitive Windows cases
- shim-oriented activators can compose on the same interface through `pulith-shim::TargetResolver`

## Current Scope

- stage from stored artifact or extracted directory handle
- stage from fetch receipts and direct archive extraction outputs
- register fetched artifacts or extracted archive trees into `pulith-store` through typed workflow helpers that preserve provenance/metadata sidecars
- register fetched archive extractions into `pulith-store` through a typed workflow helper that merges fetch provenance with archive metadata
- commit into install root atomically through `pulith-fs::Workspace`
- support create-only, replace, and upgrade install modes
- support rollback to the previous install snapshot after replacement
- restore previous activation history when rolling back replaced or upgraded installs, including cleanup of activation targets created only by the reverted activation step
- update persistent state records on install and activation
- record activation history
- replace existing activation targets consistently for both file-like and directory-like installs, relying on `pulith-fs::atomic_symlink` for symlink/junction behavior
- remove existing activation links in a link-aware way so reinstall flows can replace prior symlink/junction targets without traversing or deleting the linked install content
- clear Windows read-only attributes before deleting prior install/activation targets during replace and rollback paths
- surface Windows file-symlink privilege failures as a dedicated install error so callers can choose an alternate activator policy when needed
- offer explicit copy-based activators for file targets instead of hiding file-link fallback inside the default link activators
- provide shim-oriented activation adapters without embedding resolver policy into `pulith-install`
- provide optional backup/restore helpers for install roots and matching state facts
- repeated copy-based activation over the same target is covered in workspace integration tests so non-link activation behavior is treated as a first-class contract, not a fallback afterthought
- archive-inclusive replace/activate/rollback recovery is also covered in workspace integration tests so recovery guarantees are exercised across both extracted-directory and fetched-archive flows
- repeated symlink-based file activation over the same target is also covered in workspace integration tests so file-target link activation is exercised as its own contract, not inferred from directory-link behavior

## Deferred

- richer rollback journals beyond single previous-install snapshots
- richer backup retention and pruning policy
- source resolution orchestration
- deeper shim-specific activation policies
