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
- [ ] Block R active (Phase-1 functional blockers)

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

## Block R (Active)

Theme: close Phase-1 functional blockers from design inventory.

Execution checklist:

- [x] remove rigid mutation-surface markers from install core; retain extension-stage composition pattern
- [x] complete HTTP transport contract in `pulith-fetch` (resume offset, explicit retry policy, progress surface)
- [x] expand archive formats in `pulith-archive` (`tar.xz`, `tar.zst`) with path-traversal/escape fixtures
- [x] add tracing baseline across fetch/install hot paths
- [ ] add real URL end-to-end integration path (`fetch->verify->extract->install->activate->inspect`)

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

1. stream-hash verification tightening in `pulith-verify`
2. `pulith-lock` crate (deterministic lock file + lock diff)
3. state/store drift + repair plan hardening evidence

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

- `pulith-fs`, `pulith-version`, `pulith-resource`, `pulith-source`, `pulith-verify`, `pulith-archive`, `pulith-fetch`, `pulith-store`, `pulith-state`, `pulith-install`, `pulith-platform`, `pulith-shim`

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
