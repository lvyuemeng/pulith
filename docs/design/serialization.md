# Serialization Backend Design

Design-first plan for decoupling persistence format backends from semantic/workflow crates.

## Problem

Multiple crates currently depend on `serde_json` directly for persistence boundaries.

This repeats format coupling across crates and makes backend evolution (binary codec, SQLite-backed blob column, alt JSON adapter) harder than necessary.

## Goals

- Keep persistence contracts mechanism-first and explicit.
- Concentrate concrete format dependencies in adapter layers.
- Preserve deterministic behavior regardless of backend.
- Keep schema/version checks explicit at each boundary.

## Non-goals

- No hidden automatic backend switching.
- No breaking rewrite of all persistence crates in one step.
- No policy logic in codec adapters.

## Proposed Contract Shape

Introduce a backend contract crate (planned in Block U), for example `pulith-serde-backend`:

- `Encoder` trait for typed encode into bytes/string target.
- `Decoder` trait for typed decode from bytes/string source.
- `DeterminismContract` docs/tests (ordering, canonicalization rules where needed).
- Typed backend errors with source chaining.

Boundary rule:

- semantic/workflow crates consume backend traits at persistence edges.
- concrete adapters (`json`, future `postcard` or sqlite-bound adapters) live in adapter modules/crates.

## Determinism Requirements

- schema version is explicit in serialized model types.
- map/set ordering is deterministic where contract requires stable snapshots.
- round-trip preserves semantic equivalence.
- diff-facing data remains stable under repeated encode/decode cycles.

## Adoption Plan by Crate

1. `pulith-lock`
   - Replace direct `serde_json` calls with backend trait entry points.
   - Keep `json` adapter as baseline default implementation.
2. `pulith-state`
   - Move snapshot read/write format operations behind backend boundary.
   - Keep current file lifecycle semantics unchanged.
3. `pulith-store`
   - Apply backend contract to metadata/provenance persistence paths.
4. `pulith-install`/`pulith-fetch`
   - Only consume backend abstractions where they persist durable structured data.

Current progress:

- `pulith-lock` now serializes via `pulith-serde-backend` JSON adapter
- `pulith-state` persistence path now encodes/decodes through backend helpers
- `pulith-store` metadata/provenance persistence now encodes/decodes through backend helpers
- `pulith-state` and `pulith-store` now enforce explicit schema-version validation at decode/load boundaries

## Compatibility Strategy

- JSON remains baseline adapter during migration.
- no default format changes during Block U.
- migrations are opt-in and explicit in docs and APIs.

## Migration and Fallback Windows

- current compatibility window: schema-versioned JSON remains the default durable format for lock/state/store persisted artifacts
- fallback rule: if backend-specific payload decoding fails or schema version is unsupported, the boundary returns typed errors; no silent format fallback is performed
- migration path: introduce new backend adapters behind explicit API selection and keep JSON decode support for documented compatibility windows
- deprecation rule: removal of legacy decode paths requires prior roadmap/docs notice and parity test coverage

## Test Strategy

- backend conformance tests (encode/decode round-trip)
- deterministic snapshot tests for lock/state/store structures
- cross-backend semantic parity tests (JSON baseline vs candidate backend)
- negative tests for schema/version mismatch behavior
