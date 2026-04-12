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
- canonical per-resource inspection reports support inspect-only detect/explain behavior against filesystem and store reality
- inspect-only report types are separate from repair-plan types so read paths stay explicit and non-mutating
- per-resource repair plans allow explicit cleanup of stale state facts without silently choosing broader reconciliation policy
- activation conflict inspection helps managers detect shared-target ownership problems before destructive cleanup logic is introduced
- store-key reference inspection helps managers protect referenced store entries before prune/cleanup behavior widens
- lifecycle-based store retention helpers help callers derive protected prune sets from semantic state instead of reconstructing retention lists manually
- state-driven metadata retention planning helps callers combine retention policy with store orphan inspection into one explicit cleanup plan
- canonical activation ownership reports provide inspect-only conflict entries with stable severity/reason fields
- reasoned retention plans expose explicit protected/removable metadata reasoning so cleanup decisions stay explainable and policy-light
- composable ownership + retention planning remains non-mutating and suitable for inspect/preview workflows
- reusable `StateAnalysisIndex` allows repeated ownership/reference/inspection workflows to amortize index-building cost
- state snapshots now carry explicit schema versions and validate at load boundaries

## How To Use It

- initialize with `StateReady::initialize(...)`
- persist/update facts with `ensure_*`, `patch_*`, and `upsert_*` methods
- use `inspect_resource(...)` and ownership/retention report methods for non-mutating analysis
- use `build_analysis_index()` when repeated analysis is expected in one process cycle
- use `capture_resource_state(...)` / `restore_resource_state(...)` when higher layers need typed per-resource recovery payloads

## Contracts

Guarantees:

- inspect and planning APIs are non-mutating by contract (`inspect_*`, ownership reports, retention plans, and combined ownership+retention planning)
- state restore helpers reapply captured per-resource facts atomically at the state-file level (resource record + activation records for that resource)
- inspect output ordering and reason fields are stable enough for deterministic rendering and test assertions

Non-guarantees:

- inspect/planning helpers do not repair drift automatically; mutation remains explicit through repair/apply APIs
- restore scope is limited to captured resource facts and does not synthesize missing filesystem/install content by itself

## Design Boundary

`pulith-state` does not decide installation policy or dependency resolution.

It only persists resource lifecycle facts and activation state safely.
