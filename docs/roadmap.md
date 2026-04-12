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
- [x] Block T completed (stabilization and publish hardening)
- [x] Block U completed (serialization backend decoupling)
- [x] Block V completed (algorithmic scaling + ergonomic consolidation)
- [x] Block W completed (scaling guarantees + parity hardening)
- [x] Block X completed (typed workflow contracts + source normalization)

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

### Wave 4 (serialization backend decoupling, design-first)

1. define codec/backend abstraction for structured persistence boundaries (state/lock/store-facing data)
2. avoid repeated direct `serde_json` coupling across crates by introducing a single backend contract and adapter policy at composition edges
3. document deterministic encoding guarantees, schema-versioning expectations, and migration rules
4. add conformance tests so backend implementations (json baseline, future binary/sqlite adapters) preserve contract semantics

### Wave 5 (algorithmic scaling + ergonomic consolidation)

1. remove O(n^2)-like ownership/reference scans from state/store hot paths by introducing deterministic indexes/maps
2. consolidate isolated helper logic into crate-owned methods where it improves boundary clarity and testability
3. reduce type/clone boilerplate in state/store/resource mutation/planning paths while preserving explicit contracts
4. benchmark scaling after each change window and document regressions/threshold notes

### Wave 6 (scaling guarantees + parity hardening)

1. add cross-backend parity tests for lock/state/store persistence semantics
2. prototype reusable ownership/reference indexes for repeated inspection/planning loops
3. define benchmark guardrail notes for growth envelopes and alert conditions
4. finalize migration/fallback compatibility windows for backend evolution

### Wave 7 (typed workflow contracts + source normalization)

1. remove remaining bool-driven workflow capability/disposition APIs in `pulith-install`
2. reduce install workspace/staging leakage by keeping staging machinery internal and typed at receipt/plan boundaries
3. normalize overlapping remote source families in `pulith-source` behind shared remote-source vocabulary
4. continue replacing parse/format helper functions with `FromStr`/`Display` + crate-owned methods where string boundaries are first-class

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
- `docs/design.md`, `docs/AGENT.md`, `docs/publish/overview.md`, and `README.md` now reflect adapter/example scope without `pulith-shim-bin` after crate removal

---

Status: completed.

---

## Block T (Completed)

Theme: stabilization and publish hardening.

Execution checklist:

- [x] introduce optional runtime adapter surface for fetch internals beyond retry delay (task/concurrency boundaries) without changing semantic contracts
- [x] harden `pulith-lock` contract with validation and cross-crate integration touchpoints (`pulith-state`/`pulith-store` evidence path)
- [x] add benchmark evidence notes for lock diff and repair-plan scale behavior under `docs/benchmarks/`
- [x] align publish/readiness docs and crate metadata for new crate/layout moves (`pulith-lock`, `examples/pulith-backend-example`, top-level `pulith-shim`)
- [x] write serialization-backend architecture note and crate-boundary adoption plan to prepare Block U execution

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

Latest evidence update (in progress):

- removed `crates/pulith-shim-bin/` as redundant adapter template; shim boundary remains in `crates/pulith-shim/` and docs now point to direct example integration
- `crates/pulith-lock/src/lib.rs` now includes explicit lock validation (`validate`, `from_json_validated`) with typed `LockError` and schema/empty-field tests
- `docs/roadmap.md`, `docs/design.md`, `docs/AGENT.md`, `docs/publish/overview.md`, and `README.md` are synchronized with current crate/example layout
- `docs/design/serialization.md` now defines the design-first serialization backend abstraction, determinism requirements, and crate-by-crate adoption plan for Block U
- `crates/pulith-fetch/src/fetch/batch.rs` now removes runtime task-spawn coupling (`tokio::spawn`) from batch concurrency path and handles semaphore acquisition errors explicitly (no `unwrap` in library path)
- `crates/pulith-state/src/lib.rs` now exposes `export_lock_file()` to bridge resolved state facts into `pulith-lock` deterministically, with integration tests for resolved/unresolved record behavior
- `docs/benchmarks/block-t-2026-04.md` records lock-diff and repair-plan scale benchmark runs with command lines, environment notes, and baseline medians

---

Status: completed.

---

## Block U (Completed)

Theme: serialization backend decoupling and persistence portability.

Execution checklist:

- [x] define a serialization backend contract crate (encode/decode traits + deterministic behavior requirements)
- [x] keep JSON as baseline adapter while removing repeated direct `serde_json` usage from semantic/workflow crates where a backend trait can be consumed
- [x] wire state/lock/store boundaries to backend contract with explicit schema/version checks
- [x] add compatibility tests for round-trip, deterministic ordering, and cross-backend semantic parity
- [x] document migration and fallback behavior (no hidden format switching)

Exit criteria:

- serialization behavior is mechanism-first and backend-agnostic at crate boundaries
- deterministic persistence guarantees are test-backed and format-independent
- direct format dependencies are concentrated in adapter layer(s), not repeated across semantic/workflow crates

Evidence targets:

- `docs/design.md`
- `docs/design/*.md`
- `crates/pulith-state/`
- `crates/pulith-lock/`
- `crates/pulith-store/`

Latest evidence update (in progress):

- `crates/pulith-serde-backend/src/lib.rs` introduces the first backend contract (`TextCodec`) with JSON baseline adapter (`JsonTextCodec`) and deterministic round-trip/ordering tests
- `crates/pulith-lock/src/lib.rs` now consumes `pulith-serde-backend` instead of direct `serde_json` APIs at the lock semantic boundary
- `crates/pulith-state/src/lib.rs` and `crates/pulith-store/src/lib.rs` now route persistence encode/decode through `pulith-serde-backend` helpers while keeping JSON as baseline adapter
- `crates/pulith-state/src/lib.rs` adds explicit `StateSnapshot` schema-version validation at load boundary
- `crates/pulith-store/src/lib.rs` adds explicit `StoreMetadataRecord` schema-version validation in metadata decode path
- `crates/pulith-serde-backend/src/lib.rs` now includes compact-json adapter parity tests and cross-codec semantic decode checks
- `crates/pulith-lock/src/lib.rs`, `crates/pulith-state/src/lib.rs`, and `crates/pulith-store/src/lib.rs` add compact-json compatibility tests to validate cross-codec payload parity at crate boundaries
- `docs/design/serialization.md` now records migration/fallback windows with explicit no-silent-fallback contract

---

Status: completed.

---

## Block V (Completed)

Theme: algorithmic scaling + ergonomic consolidation.

Execution checklist:

- [x] replace activation ownership O(n^2)-style scans in `pulith-state` with deterministic grouped indexing
- [x] replace store-key reference/protected-key linear search accumulation with keyed grouping in `pulith-state`
- [x] merge isolated metadata decoding helper into `StoreReady` method boundary in `pulith-store`
- [x] reduce boilerplate by moving locator string conversion into `ResolvedLocator` method in `pulith-resource`
- [x] run and record post-optimization scaling benchmark evidence for updated paths

Exit criteria:

- hotspot paths avoid repeated full-scan loops where deterministic indexing is available
- boundary methods are crate-owned and reduce helper sprawl
- behavior and contracts remain deterministic, test-backed, and policy-free

Evidence targets:

- `crates/pulith-state/src/lib.rs`
- `crates/pulith-store/src/lib.rs`
- `crates/pulith-resource/src/lib.rs`
- `docs/benchmarks/`

Latest evidence update:

- ownership and store-key planning paths in `pulith-state` now use grouped deterministic maps instead of repeated search loops
- `StoreReady::decode_metadata_file(...)` in `pulith-store` absorbs prior isolated decode helper into crate-owned boundary method
- `ResolvedLocator::as_string()` in `pulith-resource` removes cross-crate locator string helper duplication
- `docs/benchmarks/block-v-2026-04.md` records post-optimization scaling runs for ownership and repair-plan benchmarks

---

Status: completed.

---

## Block W (Completed)

Theme: scaling guarantees + parity hardening.

Execution checklist:

- [x] add cross-backend parity tests covering lock/state/store snapshot semantics
- [x] prototype optional reusable activation/store indexes for repeated inspect/ownership planning loops
- [x] define benchmark guardrail notes (expected growth envelopes and alert conditions)
- [x] finalize migration/fallback compatibility note for serialization backend evolution

Exit criteria:

- backend parity is test-backed beyond single JSON adapter path
- repeated planning paths have documented scaling strategy and evidence
- migration windows and fallback behavior are explicit and non-hidden

Evidence targets:

- `crates/pulith-serde-backend/`
- `crates/pulith-state/`
- `crates/pulith-store/`
- `docs/benchmarks/`
- `docs/design/serialization.md`

Latest evidence update:

- `crates/pulith-serde-backend/src/lib.rs` adds compact codec parity validation and cross-codec semantic checks
- `crates/pulith-lock/src/lib.rs`, `crates/pulith-state/src/lib.rs`, and `crates/pulith-store/src/lib.rs` include cross-codec compatibility tests for persisted boundaries
- `crates/pulith-state/src/lib.rs` adds optional reusable `StateAnalysisIndex` for repeated inspection/ownership/reference flows
- `docs/benchmarks/block-w-2026-04.md` defines guardrail expectations and alert conditions for scaling/parity regressions
- `docs/design/serialization.md` documents explicit migration/fallback windows with typed-error no-silent-fallback behavior
- `crates/pulith-state/benches/ownership_report.rs` now benchmarks direct vs indexed ownership-report paths; indexed path shows substantially lower repeated-call latency in current baseline

---

Status: completed.

---

## Block X (Completed)

Theme: typed workflow contracts + source normalization.

Execution checklist:

- [x] replace bool-driven install capability/disposition surfaces with typed enums where touched (`InstallCapabilities`, `UninstallOptions`, plan proceed check)
- [x] add `FromStr`/`Display`-style parsing/formatting contracts for first-class string boundary types in `pulith-resource`
- [x] group remote source families under shared remote-source vocabulary in `pulith-source`
- [x] reduce install workspace/staging API leakage at boundary types and receipts
- [x] reduce option-heavy/clone-heavy workflow shapes in install/source/store with more typed transitions and crate-owned methods

Exit criteria:

- install planning/uninstall contracts are typed and easier to reason about than raw booleans
- source planning model reduces overlapping remote family concepts without losing expressiveness
- parse/format boundaries use traits consistently where ergonomic and stable

Evidence targets:

- `crates/pulith-install/src/lib.rs`
- `crates/pulith-source/src/lib.rs`
- `crates/pulith-resource/src/lib.rs`
- `crates/pulith-store/src/lib.rs`
- `docs/design.md`

Latest evidence update:

- `crates/pulith-install/src/lib.rs` now uses typed capability/disposition enums instead of bool fields for planning and uninstall surfaces, and `InstallPlanReport` computes proceed state via method
- `crates/pulith-resource/src/lib.rs` now exposes `Display`/`FromStr` for `ResourceId` and `ValidUrl`
- `crates/pulith-source/src/lib.rs` now groups direct URL/mirror/git families under `RemoteSource` and introduces `SourcePath` with `Display`/`FromStr`
- `crates/pulith-store/src/lib.rs` continues helper-to-method consolidation through `StoreProvenance` crate-owned metadata shaping
- `crates/pulith-install/src/lib.rs` now hides raw `Workspace` staging behind internal `StagingArea` methods, removing free workspace helper leakage from install flow internals
- `crates/pulith-install/src/lib.rs` now serializes typed `ResourceStateSnapshot` backup payloads through `pulith-serde-backend`, removing bespoke option-heavy backup payload shape and direct JSON glue

---

Status: completed.

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
- `runtime-manager-example`

## Release Readiness

Ready for next publish wave when:

- Block Q exit criteria are met with evidence links
- docs and examples match canonical APIs
- clippy/tests are green for changed crates
- adaptation workflows are explicit, typed, and policy-free
