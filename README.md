# pulith

Pulith is a Rust crate ecosystem for building resource managers.

It is for tools that need to work with external resources safely and consistently: choose versions, plan sources, fetch bytes, verify content, store artifacts, extract archives, install binaries, activate commands, persist lifecycle state, inspect drift, and plan repair or cleanup.

## What Problem It Solves

Many real tools keep rebuilding the same layers:

- version parsing and preference selection
- source planning and mirror handling
- content verification
- atomic filesystem updates
- artifact storage and extraction
- install and activation flow
- persistent state, recovery, and cleanup
- cross-platform behavior

Pulith exists to make those layers reusable without forcing one package format, backend shape, or repository model.

## Design Principles

Pulith follows a few strong rules:

- mechanism-first, not framework-first
- semantic APIs over raw path/string glue
- explicit effects over hidden orchestration
- composable crates over one monolithic manager
- cross-platform behavior treated as a primary constraint
- lifecycle, provenance, and version intent should remain aligned across crates

## How It Works

Pulith is split into small crates that can be used independently or composed into a larger workflow.

Typical flow:

1. define a resource semantically with `pulith-resource`
2. derive planned sources with `pulith-source`
3. fetch bytes with `pulith-fetch`
4. verify content with `pulith-verify`
5. extract archives with `pulith-archive`
6. register artifacts or extracted trees in `pulith-store`
7. install and activate with `pulith-install`
8. persist, inspect, repair, and retain through `pulith-state`

The crates are designed so that each step stays explicit. Pulith tries to reduce repeated orchestration, not hide policy.

## Crate Overview

Primitive crates:

- `pulith-version` - version parsing, matching, and preference selection
- `pulith-platform` - OS, arch, shell, env, and directory helpers
- `pulith-fs` - atomic filesystem and workspace primitives
- `pulith-verify` - streaming verification primitives
- `pulith-archive` - archive extraction with sanitization
- `pulith-fetch` - transfer execution primitives
- `pulith-shim` - shim resolution primitives
- `pulith-shim-bin` - thin shim-binary helper/template

Semantic and workflow crates:

- `pulith-resource` - semantic resource description
- `pulith-source` - source definitions and planning
- `pulith-store` - artifact and extracted-tree storage
- `pulith-state` - persistent lifecycle state, inspection, repair, retention planning
- `pulith-install` - typed install and activation workflow

Examples and adapters:

- `pulith-backend-example` - thin adapter-first backend example
- `examples/runtime-manager/` - partially practical multi-crate integration example

Each crate has its own `README.md` with basic usage and main APIs.

## What You Can Expect As A User

If you are building a:

- system package manager
- config manager
- plugin manager
- runtime/tool installer

Pulith aims to give you:

- predictable fetch/store/install/activate/rollback behavior
- explicit verification and provenance handling
- persistent lifecycle state that can be inspected and repaired
- cross-platform activation behavior that is explicit where semantics differ
- low-glue composition across crates without adopting a rigid package model

## Practical Example

The best current starting point is:

- `examples/runtime-manager/`

It demonstrates:

- semantic resource definition
- local archive fetch and extraction
- store registration with provenance
- install and activation
- inspection, repair planning, and prune planning

## Architecture And Roadmap

Design and architecture:

- `docs/design.md`

Detailed crate design notes:

- `docs/design/version.md`
- `docs/design/platform.md`
- `docs/design/fs.md`
- `docs/design/verify.md`
- `docs/design/archive.md`
- `docs/design/fetch.md`
- `docs/design/resource.md`
- `docs/design/source.md`
- `docs/design/store.md`
- `docs/design/state.md`
- `docs/design/install.md`
- `docs/design/shim.md`

Current priorities and phased work:

- `docs/roadmap.md`

## Status

- under active development
- crate boundaries are in place and increasingly validated
- integration, operational behavior, and contract coverage are still being tightened

## Local Workflow

Pulith uses standard Cargo commands and a `just`-based local workflow.

Install `just`:

```bash
cargo install just
```

Common commands:

```bash
just fmt
just check
just clippy
just test
just doc
just verify
just ci
```

## Compatibility And CI

The repository checks:

- formatting
- clippy
- tests on Linux, macOS, and Windows
- docs build
- MSRV on Rust `1.88.0`
- dependency-policy tooling (`cargo audit`, `cargo tree -d`, `cargo deny`)

## Contributing

Recommended flow:

1. read `docs/design.md` and `docs/roadmap.md`
2. keep changes composable and policy-light
3. add or update tests when behavior changes
4. run `just verify` before opening changes
5. run `just ci` when touching dependency or CI-related behavior

Project-specific coding guidance lives in `docs/AGENT.md`.

## License

Licensed under Apache License 2.0.

See [LICENSE](./LICENSE).
