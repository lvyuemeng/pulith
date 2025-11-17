# Pulith Migration

Pulith started as an experiment to create a cross-OS system package manager. After evaluating the ecosystem and the maintenance cost of building a full system package manager from scratch, this repository is being repurposed into a focused Rust workspace providing small, reusable building blocks for package management tasks.

Why we are pivoting
- There are established mature full-system managers (Nix, Homebrew, OS package managers).
- The highest-value, maintainable outcome is to provide reusable primitives other projects can integrate.

## New focus (initial)
- pulith-core: store layout, atomic install/uninstall, config, cross-OS path helpers
- pulith-shim: shim creation/removal, PATH activation, per-project activation helpers
- pulith-install: downloader, checksum/signature verification, extraction helpers and per-OS adapters
- pulith-cli: demo/UX wrapper showing how to use the crates

## Why we are pivoting
- There are well-established, mature full-system managers (Nix, Homebrew, OS package managers). Recreating a full package manager duplicates large, battle-tested ecosystems.
- Our highest probability of delivering value is to provide small, well-tested primitives that reduce duplication and are easy for other projects to adopt.
- Narrow, focused crates are easier to maintain, test, and document.

## Planned changes
- Convert the repository into a Cargo workspace with focused crates:
  - crates/pulith-core — store layout and atomic FS primitives
  - crates/pulith-install — downloader, verifier, extractor, staged install
  - crates/pulith-shim — shim and PATH/activation helpers
  - crates/pulith-cli — a minimal CLI demonstrating how to use the crates
- Keep a `pulith` meta-crate (optional) to re-export common functions for early adopters.
- Keep design notes and any useful code; refactor or rewrite where necessary for clarity and stability.
- Hide heavy deps behind Cargo features (e.g., signature verification, async HTTP, archive formats).

## How to contribute
- Open an issue describing what you want to help with (label it `help wanted` or `good-first-issue`).
- If you're unsure what to pick, comment on the design/planning issue and we'll help you find a starting task.
- For API design changes, open a discussion or an RFC-style issue first.

---
This file explains the pivot and gives guidance for contributors and users. See `README.md` for immediate contributor onboarding and development instructions.