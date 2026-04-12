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
- `InstallWorkflowVariant`
- `InstallWritableScope`
- `InstallCapabilities`
- `InstallPlanningRequest`
- `InstallPlanLimitation`
- `InstallPlanReport`
- `LifecycleOperationPhase`
- `LifecycleOperationDetails`
- `LifecycleOperationReceipt`
- `RollbackReceipt`
- `UninstallOptions`
- `UninstallReceipt`
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

## Variant Capability Planning

`pulith-install` provides a read-only capability planning surface before mutation:

- `InstallSpec::plan(InstallPlanningRequest) -> InstallPlanReport`
- planning is side-effect-free and can be used by callers to block or reroute workflow execution

Planning dimensions:

- desired workflow variant (`DirectLocalArtifact`, `PreStagedStore`, `AirGappedMirrorCache`, `ScopedInstall`)
- required writable scope (`User` or `System`)
- declared capabilities (`ConnectivityMode`, `ActivationSupport`, `InstallWritableScope`, `RollbackSupport`)

Typed limitations are emitted through `InstallPlanLimitation` so fallback boundaries are explicit and machine-readable.

Non-filesystem side effects (registry/service/env) should be modeled as caller-owned extension stages around install planning/execution, preserving pipeline composability without rigid core surface enums.

## Activation Boundary

Activation is expressed as a trait:

- callers may provide their own activator
- the crate ships a basic `SymlinkActivator`
- the crate also ships explicit copy-based file activators for privilege-sensitive Windows cases
- shim-oriented activators can compose on the same interface through `pulith-shim::TargetResolver`

## Guarantees / Non-Guarantees

How to use it:

- build `InstallSpec` from materialized semantic input (`InstallInput` or `IntoInstallInput`)
- call `plan(...)` first when callers need explicit machine-readable limitations
- drive `PlannedInstall -> StagedInstall -> InstalledInstall -> ActivatedInstall` in order
- use typed uninstall dispositions (`UninstallDisposition`) when partial cleanup is required
- use `create_backup(...)` / `restore_backup(...)` when caller-owned workflows need explicit install-root + state snapshot recovery

Guarantees:

- retrying replace/upgrade installs is safe when a rollback snapshot exists; commit/rollback paths restore the previous install root on failure boundaries where a snapshot was captured
- explicit rollback restores both install content and captured `pulith-state` facts for that resource (resource record + activation history)
- activation replacement is explicit: existing activation targets are removed before a new link/copy target is written, for both file-like and directory-like targets
- Windows file symlink privilege failures are surfaced as `InstallError::WindowsFileSymlinkPrivilege` instead of hidden fallback behavior
- uninstall composition is explicit and scope-controlled: default uninstall removes install root + activation targets + matching state facts, while `UninstallOptions` uses typed dispositions to preserve selected surfaces

Non-guarantees:

- rollback is not available for create-only flows or any flow where no previous install snapshot exists (`InstallError::RollbackUnavailable`)
- backup/restore scope is per-resource install tree + state facts; it does not restore unrelated resources or caller-owned side effects outside those boundaries
- `pulith-install` does not define fetch retry policy and does not provide multi-step rollback journals beyond a single previous-install snapshot
- uninstall does not implicitly prune store artifacts/metadata; store retention/prune remains explicit caller policy

## Current Scope

- stage from semantic install sources only: `StagedFile`, `StoredArtifact`, `ExtractedArtifact`, `ExtractedTree`
- keep fetch/archive materialization outside `pulith-install`; callers compose store registration first, then install using semantic handles
- use `IntoInstallInput` as the canonical pipe boundary so install staging does not absorb fetch/archive transport types
- provide typed read-only variant planning (`InstallPlanReport`) so downgrade/fallback reasons are explicit before mutation
- commit into install root atomically through `pulith-fs::Workspace`
- stage file transitions with install-tuned adaptive copy/link behavior (1 MiB copy-only threshold baseline for install staging)
- avoid duplicate per-file metadata probes during directory staging by reusing directory entry size metadata for adaptive transitions
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
- provide optional backup/restore helpers for install roots and matching typed `pulith-state::ResourceStateSnapshot` facts
- provide composed uninstall helper (`uninstall_resource`) with explicit scope options instead of hidden global cleanup policy
- provide additive unified lifecycle receipt envelope (`LifecycleOperationReceipt`) with operation context + phase-specific details, while keeping operation-specific receipts available
- repeated copy-based activation over the same target is covered in workspace integration tests so non-link activation behavior is treated as a first-class contract, not a fallback afterthought
- archive-inclusive replace/activate/rollback recovery is also covered in workspace integration tests so recovery guarantees are exercised across both extracted-directory and fetched-archive flows
- repeated symlink-based file activation over the same target is also covered in workspace integration tests so file-target link activation is exercised as its own contract, not inferred from directory-link behavior

## Deferred

- richer rollback journals beyond single previous-install snapshots
- richer backup retention and pruning policy
- source resolution orchestration
- deeper shim-specific activation policies
