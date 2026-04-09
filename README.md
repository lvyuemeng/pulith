# pulith

Pulith is a Rust crate ecosystem for resource management primitives: version selection, source planning, fetching, verification, storage, extraction, installation, activation, and persistent state.

It is designed for tools that manage external resources without wanting to rebuild the same low-level machinery over and over again.

Status:

- under active development
- API shape is stabilizing, but several crates are still marked as maturing or emerging

## Motive

Many developer tools end up re-implementing the same hard parts:

- version parsing and selection
- source planning and download orchestration
- checksum and integrity verification
- atomic file replacement and staging
- install/activation flows with rollback
- persistent state tracking
- Windows/macOS/Linux behavior differences

Pulith exists so tool authors can compose those concerns from reusable crates instead of building a one-off internal framework.

## Design Direction

Pulith is intentionally:

- mechanism-first
- composable rather than framework-driven
- type-driven where workflow ordering matters
- policy-light so backend/tool-specific behavior stays outside the core crates

The current architectural view and roadmap live in:

- `docs/design.md`
- `docs/roadmap.md`

## Workspace Layout

- `docs/` - top-level design, roadmap, and crate-specific documents
- `crates/` - workspace crates

Current notable crates:

- `pulith-fs` - atomic filesystem and workspace primitives
- `pulith-verify` - verification primitives
- `pulith-archive` - archive extraction
- `pulith-fetch` - transfer execution
- `pulith-resource` - resource semantics
- `pulith-store` - artifact storage
- `pulith-state` - persistent lifecycle state
- `pulith-install` - typed installation workflow
- `pulith-source` - source planning

## CI and Dependency Policy

The repository CI currently checks:

- formatting
- clippy
- tests across Linux, macOS, and Windows
- docs build
- MSRV check on Rust `1.85.0`
- dependency policy with:
  - `cargo audit`
  - `cargo tree -d`
  - `cargo deny check advisories bans sources`

Notes:

- the MSRV job verifies the minimum supported Rust version we currently target; because the workspace uses Rust 2024 edition, `1.85.0` is the practical floor
- `cargo deny` is intentionally not used for license allowlisting in CI right now; the license check is stricter and more maintenance-heavy than the current project policy needs

## Development

Recommended local workflow uses `just` to keep common commands short and consistent.

If you do not have `just` installed yet:

```bash
cargo install just
```

Common commands:

```bash
just fmt
just fmt-check
just check
just clippy
just test
just doc
just audit
just tree
just deny
just verify
just ci
```

Equivalent raw cargo commands:

```bash
cargo fmt --all --check
cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo doc --workspace --all-features --no-deps
cargo audit
cargo tree --workspace --all-features -d
cargo deny check --all-features advisories bans sources
```

## Contributing

Contributions are welcome.

Recommended flow:

- read `docs/design.md` for the current architectural direction
- read `docs/roadmap.md` for active priorities
- read `docs/AGENT.md` for coding and review expectations
- keep changes composable and policy-light
- prefer improving integration quality and tests over widening surface area unnecessarily
- use `just verify` before opening changes
- use `just ci` when touching dependency policy, CI, or workspace-wide behavior

Suggested contribution checklist:

- implement the smallest coherent change
- add or update tests when behavior changes
- update docs when public workflow or crate roles change
- run validation locally with `just verify`
- run dependency checks with `just ci` when changing dependencies or CI-related files

## License

This repository is licensed under Apache License 2.0.

See [LICENSE](./LICENSE).
