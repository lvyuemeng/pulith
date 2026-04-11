# Pulith Roadmap

## Goal

Deliver a trustworthy, mechanism-first resource-management substrate with explicit contracts, low API glue, and test-backed behavior.

## Current Stage

- Milestone 6: API ergonomics + lifecycle contract hardening (active)

Block status:

- [x] Block A-K completed
- [x] Block L completed
- [x] Block M completed
- [x] Block N completed
- [x] Block O completed
- [x] Block P completed
- [x] Block Q completed (workflow adaptation + boundary hardening)
- [x] Block R completed (Phase-1 functional blockers)
- [x] Block S completed (trust + reproducibility foundation)
- [ ] Block T active (stabilization and publish hardening)

---

## Block Q (Completed)

Theme: remove boundary leakage and formalize install workflow variants.

### Completed in Q

- [x] install boundary moved to materialized semantic inputs
- [x] backward compatibility helpers/aliases removed from install boundary
- [x] store owns provenance composition and registration absorption path
- [x] lifecycle envelope introduced for unified receipt consumption
- [x] state inspection unified to report/finding model
- [x] runtime example migrated to trait/method pipeline
- [x] variant planning is capability-based in core; external side effects are extension-stage composition

### Remaining in Q

- [x] add explicit variant examples: mirrored/air-gapped, pre-staged store, scoped install
- [x] add negative-path variant tests (offline fallback, activation unavailable, partial repair)
- [x] document typed variant capability/planning model in crate-level design docs

### Exit Criteria

- install API stays transport-agnostic
- store registration is trait-absorbed for fetch/archive evidence
- lifecycle output has one normalized consumption path
- at least four install variants are documented and test-backed
- fallback and limitation reasons are explicit and typed

### Evidence Targets

- `docs/design.md`
- `docs/design/install.md`
- `examples/runtime-manager/`
- `crates/pulith-install/tests/workspace_pipeline.rs`

Status: completed.

---

## Block R (Completed)

Theme: close Phase-1 functional blockers from design inventory.

Execution checklist:

- [x] remove rigid mutation-surface markers from install core; retain extension-stage composition pattern
- [x] complete HTTP transport contract in `pulith-fetch` (resume offset, explicit retry policy, progress surface)
- [x] expand archive formats in `pulith-archive` (`tar.xz`, `tar.zst`) with path-traversal/escape fixtures
- [x] add tracing baseline across fetch/install hot paths
- [x] add real URL end-to-end integration path (`fetch->verify->extract->install->activate->inspect`)

Exit criteria:

- Phase-1 blockers are test-backed and documented
- no hidden policy enters primitive crates
- end-to-end path exercises real transport + archive + install boundaries

Evidence targets:

- `crates/pulith-fetch/`
- `crates/pulith-archive/`
- `crates/pulith-install/tests/`
- `examples/runtime-manager/`

Latest evidence update:

- `crates/pulith-fetch/src/config/fetch_options.rs` now carries explicit `RetryPolicy` plus `expected_bytes` and `resume_offset`
- `crates/pulith-fetch/src/fetch/fetcher.rs` now executes retry policy explicitly and applies resume offset as `Range` request behavior
- `crates/pulith-fetch/src/fetch/fetcher.rs` tests cover retry exhaustion and resume-header application
- `crates/pulith-archive/Cargo.toml` enables `xz` and `zstd` in default feature set
- `crates/pulith-archive/src/format.rs` adds filename-based format detection covering `.tar.xz` and `.tar.zst`
- `crates/pulith-archive/src/extract.rs` tests now include malicious tar.xz/tar.zst fixtures for relative escape, absolute path, and symlink escape rejection
- `crates/pulith-fetch/src/fetch/fetcher.rs` now has tracing instrumentation on `head`, `fetch_with_receipt`, retry attempt path, and source-try path
- `crates/pulith-install/src/lib.rs` now has tracing instrumentation on `stage`, `commit`, `activate`, and rollback hot paths
- `examples/runtime-manager/src/main.rs` now includes `install-remote-archive` as real URL fetch->extract->store->install->activate->inspect path
- `crates/pulith-install/tests/workspace_pipeline.rs` now includes an opt-in ignored integration test (`real_url_end_to_end_pipeline_path`) gated by `PULITH_E2E_ARCHIVE_URL`

---

Status: completed.

---

## Next Concrete Implementation Plan

### Wave 1 (Q completion)

1. **Typed variant planning surface**
   - Add install capability model (offline, activation, writable scope, rollback expectation)
   - Keep non-filesystem side effects as caller extension pipeline stages
   - Produce a read-only plan/report type consumed before mutation

2. **Variant examples**
   - Add runtime-example commands for:
     - local direct artifact
     - pre-staged store install
     - air-gapped mirror/cache install
     - scoped install target (user/system layout)

3. **Negative-path integration tests**
   - no-network + cache-only behavior
   - activation unavailable handling
   - uninstall/reinstall repair boundaries

4. **Docs + contract sync**
   - align `docs/design/install.md` with capability planning and variant receipts
   - add evidence notes for each new variant test path

### Wave 2 (Phase-1 functional blockers from design inventory)

1. HTTP transport completion in `pulith-fetch` (resume, retry policy, progress)
2. Archive support expansion in `pulith-archive` (`tar.xz`, `tar.zst`) with attack fixtures
3. Cross-crate tracing baseline on fetch/install hot paths
4. Real URL end-to-end integration path (fetch->verify->extract->install->activate->inspect)

### Wave 3 (trust + reproducibility foundation)

1. resource taxonomy -> essential behavior contract formalization
2. stream-hash verification tightening in `pulith-verify`
3. `pulith-lock` crate (deterministic lock file + lock diff)
4. state/store drift + repair plan hardening evidence
5. stabilization decision log (dispatch, runtime, resolution, state backend, plugin protocol)

---

## Block S (Completed)

Theme: trust + reproducibility foundation.

Execution checklist:

- [x] formalize resource behavior axis contract (materialization, activation, mutation scope, provenance, lifecycle) in crate-level APIs/tests
- [x] tighten stream-hash verification pipeline behavior in `pulith-verify`
- [x] introduce `pulith-lock` crate with deterministic serialization and lock diff
- [x] harden state/store drift + repair evidence for lifecycle recovery guarantees
- [x] decide and document open stabilization decisions (dispatch strategy, runtime coupling, resolution scope, state backend, plugin protocol)

Exit criteria:

- lock and verification contracts are documented and test-backed
- drift/repair behavior is explicit, deterministic, and evidence-linked
- no manager policy leakage into core semantic/workflow crates
- resource taxonomy maps to essential behaviors without rigid type explosion
- open stabilization decisions are recorded with explicit initial choices and compatibility notes

Evidence targets:

- `docs/design.md`
- `docs/design/*.md`
- `crates/pulith-verify/`
- `crates/pulith-lock/` (new)
- `crates/pulith-state/`

Latest evidence update (in progress):

- `crates/pulith-resource/src/lib.rs` adds explicit behavior-axis types (`ActivationModel`, `MutationScope`, `ProvenanceRequirement`, `LifecycleRequirements`) and `ResourceBehaviorContract`
- `crates/pulith-resource/src/lib.rs` extends `ResourceSpec` with behavior-axis fields and builders while keeping materialization semantics explicit
- `crates/pulith-resource/src/lib.rs` tests now assert default behavior contract, axis specialization, and requested->resolved contract continuity
- `crates/pulith-verify/src/reader.rs` now tracks streamed byte count and exposes `finish_with_constraints(...)` plus `verify_stream(...)` for digest+length verification
- `crates/pulith-verify/src/error.rs` introduces `VerifyError::SizeMismatch` to surface deterministic stream-length failures
- `crates/pulith-fetch/src/config/fetch_options.rs` adds runtime-agnostic retry delay injection (`RetryDelayProvider`) so retry waiting can be decoupled from any specific runtime in public contracts
- `crates/pulith-lock/src/lib.rs` improves lock quality and efficiency by removing duplicated resource identity from entries (`LockedResource` keyed by map id), pre-sizing diff buffers, and adding deterministic diff-empty coverage
- `crates/pulith-state/src/lib.rs` tests now assert repair-plan determinism and idempotent repair-apply behavior for lifecycle recovery guarantees
- `docs/design/stabilization.md` records initial decisions for dispatch strategy, runtime coupling, resolution scope, state backend, and plugin protocol

---

Status: completed.

---

## Block T (Active)

Theme: stabilization and publish hardening.

Execution checklist:

- [ ] introduce optional runtime adapter surface for fetch internals beyond retry delay (task/concurrency boundaries) without changing semantic contracts
- [ ] harden `pulith-lock` contract with validation and cross-crate integration touchpoints (`pulith-state`/`pulith-store` evidence path)
- [ ] add benchmark evidence notes for lock diff and repair-plan scale behavior under `docs/benchmarks/`
- [ ] align publish/readiness docs and crate metadata for new crate/layout moves (`pulith-lock`, `examples/pulith-backend-example`, top-level `pulith-shim`)

Exit criteria:

- runtime coupling remains non-invasive and explicit across touched boundaries
- lock model behavior is documented, deterministic, and benchmark-evidenced
- release/readiness docs are synchronized with workspace structure

Evidence targets:

- `crates/pulith-fetch/`
- `crates/pulith-lock/`
- `crates/pulith-state/`
- `docs/benchmarks/`
- `docs/publish/*.md`

---

## Quality Track (Always On)

- API gate: one canonical boundary path per crate role
- composition gate: no manual provenance/path glue in runtime hot path
- reliability gate: mutation-path changes add negative tests
- performance gate: touched hot paths run benchmark/strict validation
- policy gate: no hidden strategy selection in core crates

Validation routine for change windows:

- `cargo fmt --all`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test -p pulith-install -p pulith-store -p pulith-state -p pulith-resource`
- targeted integration tests for affected variant paths

---

## Publish Scope

Public-target crates:

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-lock`, `pulith-install`, `pulith-platform`, `pulith-shim`

Internal/non-publish crates:

- `pulith-backend-example`
- `pulith-shim-bin`
- `runtime-manager-example`

## Release Readiness

Ready for next publish wave when:

- Block Q exit criteria are met with evidence links
- docs and examples match canonical APIs
- clippy/tests are green for changed crates
- adaptation workflows are explicit, typed, and policy-free
