# pulith

A crate ecosystem providing everything a rust tool needs to fetch, verify, store, and track external resources - packages, config files, tools, plugins, or any versioned artifacts.

> Currently, it's **under-development**. Welcome contribution!

## why this exists

80% of tools that manage external resources reinvent the same primitives:
- version parsing and comparison
- http downloads with progress and verification
- atomic file operations and staging
- state tracking with rollback
- cross-platform correctness

this ecosystem provides battle-tested building blocks so developers can focus on their unique value proposition.

## Layout

- [docs/](./docs/) - documents
- [crates/](./crates/) — workspace

## Contribution 

- Clone the repo.
- Use any AI agent tool to read [AGENT.md](./docs/AGENT.md) starting coding.
