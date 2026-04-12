# pulith-lock

Deterministic lock-file and diff primitives for reproducible resource workflows.

## Purpose

`pulith-lock` provides a stable lock representation and deterministic diffing.

It does not perform dependency solving or hidden resolution policy.

## Core Types

- `LockedResource`
- `LockFile`
- `LockDiff`
- `LockResourceChange`

## Contract

- lock serialization is deterministic via sorted key spaces
- lock diff is explicit (`added`, `removed`, `changed`) and key-addressed by resource id
- lock behavior is policy-free (callers decide how to react to diff output)
- lock serialization uses backend contract adapters (`pulith-serde-backend`) instead of hard-coding JSON APIs at semantic boundary

## Non-goals

- no dependency graph solving
- no implicit conflict resolution
- no automatic repair/apply policy
