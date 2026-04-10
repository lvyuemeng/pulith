# Pulith Engineering Guide

This document defines coding, testing, CI, and publishing expectations for contributors.
It is aligned with `docs/design.md` and `docs/roadmap.md`.

## Language and Tooling

- **Rust Edition**: 2024
- **MSRV**: 1.88.0
- **Formatter**: `cargo fmt --all`
- **Linter**: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- **Docs**: `cargo doc --workspace --all-features --no-deps` with `RUSTDOCFLAGS=-D warnings`
- **Task runner**: `just` (`just --list` for available commands)

## Philosophy (Design-Aligned)

### 1) Mechanism-first, policy-free core

Pulith provides composable primitives and workflows, not a hidden framework.

- Keep crate APIs mechanism-oriented and explicit.
- Do not embed manager policy (ranking, trust, channels, cleanup decisions) in helpers.
- Prefer data and typed contracts over implicit conventions.

### 2) Explicit composition contract

Code and APIs should support this pipeline directly:

1. describe resource semantics
2. plan or derive sources
3. fetch and verify material
4. register/store or extract
5. install and optionally activate
6. persist lifecycle facts
7. inspect drift and apply explicit repair plans

If a change forces callers to rebuild key/path/provenance glue manually, the design likely needs adjustment.

### 3) Pure reasoning, explicit effects

- Keep decision logic deterministic where possible.
- Keep I/O, network, filesystem, and platform effects at clear boundaries.
- Surface side effects and randomness in interfaces and types.
- Prefer typed receipts/reports over ad hoc reconstruction.

### 4) Guarantees and non-guarantees must stay honest

- Only claim guarantees backed by tests and documented behavior.
- Preserve explicit non-guarantees (no dependency solving, no hidden repair, no global rollback journal).
- Platform differences should surface as typed, explainable behavior.

## Architecture and Workspace

```text
pulith/
├── Cargo.toml                 # Workspace manifest
├── justfile                   # Local command entrypoints
├── crates/
│   ├── pulith-*/              # Library and adapter crates
│   └── ...
├── examples/
│   └── runtime-manager/       # Composition reference
└── .github/workflows/         # CI and benchmark workflows
```

Crate roles should remain narrow and composable:

- **Primitive**: `pulith-platform`, `pulith-version`, `pulith-fs`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-shim`
- **Semantic**: `pulith-resource`, `pulith-source`, `pulith-store`, `pulith-state`
- **Workflow**: `pulith-install`
- **Adapter/example**: `pulith-backend-example`, `pulith-shim-bin`, `examples/runtime-manager`

Module guidance:

- Name modules and types by functionality.
- Keep module count small and cohesive.
- Prefer `name.rs + name/*` layout for growing modules.

## Dependencies

- Declare shared dependencies in workspace root under `[workspace.dependencies]`.
- Reuse workspace dependencies from crates when possible.
- Gate heavy or optional behavior behind features.
- Pin versions intentionally when stability or compatibility requires it.

## Error Handling and Boundaries

### Error model

- Library code returns concrete crate-local error enums (`thiserror`).
- Application/binary code may use `anyhow` for top-level orchestration.
- Do not use `unwrap()`, `expect()`, or `panic!()` in library code paths.

### Boundary guideline (cross-crate)

When one Pulith crate composes another:

- own one public error enum per crate;
- wrap direct dependency errors as source-bearing variants (`#[from]`/`#[source]`);
- keep crate-specific policy/contract errors explicit;
- avoid mirroring every nested dependency variant in upper layers.

## Async and Concurrency

- Use shared runtime strategy; do not spawn ad hoc runtimes in libraries.
- Keep public types `Send + Sync` where cross-thread composition is expected.
- Prefer deterministic coordination and bounded concurrency for reproducible behavior.

## Cross-Platform Requirements

- Use `Path`/`PathBuf` for all filesystem paths.
- Keep OS-specific code behind `cfg` gates.
- Prefer `pulith-platform` for platform/distro detection and normalization.
- Write explicit tests for platform contracts (especially Windows activation behavior).
- In core crates, prefer `pulith-fs` for atomic/transactional filesystem behavior; `std::fs` is acceptable in top-level example orchestration paths where those guarantees are not being implemented.

## Serialization

- Use `serde` derive for stable structured data.
- Use `postcard`/binary or JSON intentionally per boundary contract.
- Keep serialized layout as an implementation contract, not accidental public API.

## Testing

### Required layers

- **Unit tests**: colocated `#[cfg(test)]` modules for local behavior.
- **Integration tests**: `tests/` for crate/public API composition.
- **Contract tests**: guarantee-focused behavior (archive safety, recovery, activation).
- **Property/corpus tests**: parser/selector correctness (notably version behavior).
- **Benchmarks**: criterion benches for threshold decisions and regressions.

### Local test commands

- `just test` -> `cargo test --workspace --all-features`
- `just ci` -> local CI parity (`quality + verify`)
- Targeted benches:
  - `cargo bench -p pulith-fetch --bench multi_source`
  - `cargo bench -p pulith-install --bench pipeline`
  - `cargo bench -p pulith-install --bench copy_transition`

### Test quality expectations

- Add tests for every behavior-affecting change.
- Prefer contract-oriented assertions over implementation-detail assertions.
- Keep guarantees/non-guarantees in docs synchronized with executable tests.

### Benchmark evidence expectations

- benchmark-driven changes must include command lines and environment notes (OS, CPU class, storage context)
- avoid changing thresholds from a single noisy run; use repeated runs and summarize spread
- when thresholds change, document interpretation in `docs/roadmap.md` or crate-level design docs
- check in benchmark notes under `docs/benchmarks/` for milestone/block evidence

## CI

CI is defined in `.github/workflows/ci.yml` and must remain aligned with local `just` tasks.

Current required checks:

- **Lint job (Ubuntu)**: fmt check, clippy `-D warnings`, docs build with rustdoc warnings denied.
- **Test matrix**: `cargo test --workspace --all-features` on Linux, Windows, and macOS.
- **MSRV job**: `cargo check --workspace --all-features` on Rust 1.88.0.
- **Security and dependency checks**:
  - `cargo audit`
  - `cargo deny --all-features check advisories bans sources`
  - `cargo tree --workspace --all-features -d`

Benchmark workflow lives in `.github/workflows/benchmark.yml` and is run on demand.

## Publish and crates.io Readiness

Pulith is in active hardening. Publishing should be deliberate and checklist-driven.

### Release policy

- Publish only crates intended for external consumption.
- Mark internal-only crates/examples/binaries with `publish = false` when not meant for crates.io.
- Keep crate metadata complete (`description`, `license`, `repository`, `readme`, categories/keywords as appropriate).
- Current internal/non-publish examples include `runtime-manager-example`, `pulith-shim-bin`, and `pulith-backend-example`.

### Pre-publish checklist

1. Run `just ci` locally and ensure parity with GitHub CI.
2. Verify docs build cleanly with rustdoc warnings denied.
3. Confirm API and error-boundary contracts are documented.
4. Ensure tests cover changed guarantees, including platform-specific behavior.
5. Dry-run publish for each crate:
   - `cargo publish -p <crate> --dry-run`
6. Confirm internal-only crates/examples are marked `publish = false`.
7. Mirror-friendly workflow: use your configured mirror registry for fast iteration dry-runs (for example `--registry ustc` when configured), then run a final crates.io-targeted dry-run (`--registry crates-io`) in release validation.

### Publish ordering

- Publish in dependency order (lower-level crates first).
- After each publish, validate downstream crates still dry-run cleanly before proceeding.

## Documentation

- All public items require `///` docs.
- Public APIs that can fail should include `# Errors` sections.
- Include runnable examples where practical.
- Keep `docs/design.md`, crate design docs, and behavior/tests consistent.

## Code Style

- Group imports by standard library, third-party, and crate-local modules.
- Run `cargo fmt` before commit.
- Keep lines readable (target ~100 columns unless rustfmt formats differently).
- Avoid unexplained `#[allow(...)]` attributes and magic numbers.

## Pull Request Expectations

1. Keep PRs scoped and reviewable.
2. Include tests and docs updates for behavioral/API changes.
3. Ensure CI passes across all required jobs.
4. Preserve mechanism-first boundaries; avoid policy leakage into core crates.

## References

- [README.md](../README.md)
- [docs/design.md](./design.md)
- [docs/roadmap.md](./roadmap.md)
- [docs/design/](./design/) (crate-level design docs)
