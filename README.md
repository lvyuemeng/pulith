# pulith

Pulith is a Rust crate ecosystem for resource management primitives.

It is built for tools that need to work with external resources safely and consistently: choose versions, plan sources, fetch bytes, verify content, store artifacts, extract archives, install binaries, activate commands, and persist state.

## Why Pulith

Many tools re-implement the same infrastructure:

- version parsing and selection
- download and mirror handling
- checksum verification
- atomic filesystem updates
- install and activation flow
- persistent state and recovery
- cross-platform behavior

Pulith aims to make those layers reusable instead of forcing every project to build its own internal framework.

## What It Provides

Pulith is split into small crates that can be used independently or together.

Core crates:

- `pulith-version` - version parsing and selection
- `pulith-source` - source planning
- `pulith-fetch` - transfer execution
- `pulith-verify` - content verification
- `pulith-fs` - atomic filesystem and workspace primitives
- `pulith-archive` - archive extraction
- `pulith-store` - artifact storage
- `pulith-state` - persistent lifecycle state
- `pulith-install` - typed installation workflow
- `pulith-shim` - shim resolution primitives

The current architectural summary and active priorities live in:

- `docs/design.md`
- `docs/roadmap.md`

## Status

- under active development
- crate boundaries are in place
- integration and workflow quality are still being tightened

## CI and Compatibility

The repository CI checks:

- formatting
- clippy
- tests on Linux, macOS, and Windows
- docs build
- MSRV on Rust `1.88.0`
- dependency policy with:
  - `cargo audit`
  - `cargo tree -d`
  - `cargo deny` for advisories, bans, and sources

## Local Workflow

The recommended local workflow uses `just`.

Install it with:

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

`just verify` runs the local dependency-policy checks.

## Contributing

Contributions are welcome.

Recommended flow:

1. Read `docs/design.md` and `docs/roadmap.md`.
2. Keep changes composable and policy-light.
3. Add or update tests when behavior changes.
4. Run `just verify` before opening changes.
5. Run `just ci` when touching dependency or CI-related behavior.

For project-specific coding guidance, see `docs/AGENT.md`.

## License

Licensed under Apache License 2.0.

See [LICENSE](./LICENSE).
