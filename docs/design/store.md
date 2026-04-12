# pulith-store

Composable local artifact storage.

## Purpose

`pulith-store` manages where resource bytes and extracted trees live on disk.

It is intentionally storage-focused, not install-focused.

## Availability Token

`StoreReady` is the initialization token for the store subsystem.

Creation ensures required directories exist before callers can write artifacts or extracts.

## Core Types

- `StoreRoots`
- `StoreReady`
- `StoreKey`
- `StoredArtifact`
- `ExtractedArtifact`
- `StoreProvenance`
- `StoreMetadataRecord`
- `PruneReport`
- `KeyDerivation`

## Storage Philosophy

- support multiple keying strategies
- avoid hard-wiring a single content-addressed model
- keep key derivation pluggable
- stay reusable for caches, installers, and registries

## Current Scope

- artifact byte storage
- extracted directory registration
- deterministic relative naming from semantic keys
- metadata-backed provenance lookup
- key-derivation-based lookup from `ResolvedResource` into artifacts, extracts, and metadata records when callers already have semantic resource identity
- orphaned metadata pruning
- orphaned metadata inspection before prune so callers can explain or preview cleanup behavior
- protected prune planning so callers can preserve metadata for store keys still referenced by semantic state
- protected prune planning composes naturally with lifecycle-based retention helpers from `pulith-state`
- store orphan inspection also composes with state-driven metadata retention planning so callers can build cleanup plans without re-deriving protection sets by hand
- store prune planning remains storage-focused while allowing `pulith-state` to attach explicit ownership/retention reasons for inspect-first cleanup previews
- hardlink-or-copy artifact import to reduce unnecessary copying on the same filesystem
- metadata persistence now routes through `pulith-serde-backend` with explicit schema-version validation at decode boundaries
- provenance metadata shaping is crate-owned through `StoreProvenance` constructors instead of free helper sprawl

## How To Use It

- initialize once with `StoreRoots` and `StoreReady::initialize(...)`
- register artifacts or extracts with typed `StoreKey` values
- prefer `register_*` APIs when provenance should be captured from fetch/archive receipts
- use metadata listing/orphan inspection/prune planning for explicit cleanup previews rather than blind deletion

Future policies like retention and pruning stay outside the core type model until they are better understood.
