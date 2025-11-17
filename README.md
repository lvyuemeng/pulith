# pulith — package-manager primitives (pivot)

Pulith has been repurposed into a collection of small, focused Rust crates that provide primitives commonly needed when building package management tooling:
- pulith-core: store layout, atomic installs/uninstalls, configuration & platform helpers
- pulith-install: downloader + checksum/signature verification + archive extraction + staged installs
- pulith-shim: shim creation/removal, PATH activation, and per-project activation helpers
- pulith-cli: a minimal CLI that demonstrates how the crates can be used together

Why this pivot?
- Small, well-tested primitives are easy to adopt and reduce duplicated code.
- Rust is well-suited for cross-platform filesystem/networking correctness.
- Focused crates are easier to document, test, and maintain than a full package manager.

## Introduction

Quick links
- [MIGRATION](./docs/migration.md) — full explanation of the pivot and migration guidance
- [crates/](./crates/) — workspace crates (core, install, shim, cli)

### Get started (development)

1. Clone the repo
   git clone https://github.com/lvyuemeng/pulith
   cd pulith

2. Build the workspace
   cargo build --workspace

3. Run the demo CLI (after implementing the crate skeleton)
   cd crates/pulith-cli
   cargo run -- --help

## Road
- pulith-core
  - implement StoreLayout builder and ensure_layout
  - implement atomic_replace with tests (same-FS and copy-fallback)
- pulith-install
  - implement a simple downloader that saves to a temp file
  - implement extract for zip and tar.gz behind features
  - implement a staged_install example that uses pulith-core::atomic_replace
- pulith-shim
  - implement platform-independent shim script generation and removal
- pulith-cli
  - small demo to install a sample release, create a shim, list installed packages

## How to help (suggested workflow)

1. Pick an open issue with `help wanted` or `good-first-issue`. If none exist, open an issue describing a small task you'd like to tackle.
2. Fork, make a branch named `feature/<short-desc>`, and submit a PR with tests and examples.
3. Keep changes small and focused. If a change impacts the public API, open an issue describing the intended design first.

What success looks like
- A tiny, well-documented crate that solves a narrow, common problem (e.g., staged-install).
- Clear examples and integration tests.
- At least one real-world usage example or replacement of a script you/others use.