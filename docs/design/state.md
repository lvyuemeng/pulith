# pulith-state

Transaction-backed persistent state for Pulith resources.

## Purpose

`pulith-state` records lifecycle facts about resources:

- what is declared
- what has been resolved
- what has been fetched or materialized
- what has been activated

It stores facts, not package-manager policy.

## Availability Token

`StateReady` is the initialized capability token.

It guarantees the backing file exists and can be used through `pulith-fs::Transaction`.

## Core Types

- `StateReady`
- `StateSnapshot`
- `ResourceRecord`
- `ResourceRecordPatch`
- `ResourceLifecycle`
- `ActivationRecord`

## Persistence Model

- JSON by default
- transaction-backed load/save/update operations
- caller-facing records remain semantic and composable
- ergonomic helpers for ensure / lookup / patch / lifecycle updates
- semantic upsert helpers from `ResolvedResource` reduce repeated record reconstruction in workflow crates
- per-resource state capture/restore helpers support recovery composition without pushing rollback logic into every workflow caller
- per-resource inspection helpers support initial detect/explain behavior against filesystem and store reality
- per-resource repair plans allow explicit cleanup of stale state facts without silently choosing broader reconciliation policy
- activation conflict inspection helps managers detect shared-target ownership problems before destructive cleanup logic is introduced
- store-key reference inspection helps managers protect referenced store entries before prune/cleanup behavior widens
- lifecycle-based store retention helpers help callers derive protected prune sets from semantic state instead of reconstructing retention lists manually
- state-driven metadata retention planning helps callers combine retention policy with store orphan inspection into one explicit cleanup plan

## Design Boundary

`pulith-state` does not decide installation policy or dependency resolution.

It only persists resource lifecycle facts and activation state safely.
