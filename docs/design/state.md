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
- `ResourceLifecycle`
- `ActivationRecord`

## Persistence Model

- JSON by default
- transaction-backed load/save/update operations
- caller-facing records remain semantic and composable

## Design Boundary

`pulith-state` does not decide installation policy or dependency resolution.

It only persists resource lifecycle facts and activation state safely.
