# Stabilization Decisions

This document records initial Block S decisions for open stabilization topics.

These are mechanism-first defaults, not hard policy commitments.

## 1) Dispatch strategy

- Initial choice: generic/static dispatch within crate-internal hot paths; trait-object dispatch at adapter/example boundaries where plugin-like composition matters.
- Compatibility note: public contracts remain trait-based so either dispatch strategy can evolve without changing semantic behavior.

## 2) Async runtime coupling

- Initial choice: runtime-independent contract surfaces with non-invasive async hooks where waiting/scheduling behavior is needed.
- Current implementation note: fetch retry backoff accepts caller-provided async delay provider (`RetryDelayProvider`), with crate-default delay behavior when none is supplied.
- Compatibility note: runtime handles are not exposed in public semantic/workflow contracts, keeping adapter-level runtime choice open.

## 3) Resolution scope

- Initial choice: single-resource canonical contracts in semantic/workflow crates; multi-resource orchestration remains explicit caller composition.
- Compatibility note: does not block future graph-solving crates because current receipts and plans remain composable.

## 4) State backend strategy

- Initial choice: JSON-backed deterministic baseline in `pulith-state`.
- Compatibility note: persistence behavior is documented via typed plans/reports so alternate backend implementations (for example SQLite) can preserve contract semantics.

## 5) Plugin protocol boundary

- Initial choice: subprocess boundary with structured JSON exchange as baseline.
- Compatibility note: wasm/capability-hosted protocols remain future extension points and should preserve behavior-axis and receipt invariants.
