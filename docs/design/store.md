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

Future policies like retention and pruning stay outside the core type model until they are better understood.
