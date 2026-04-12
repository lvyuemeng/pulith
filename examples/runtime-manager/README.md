# runtime-manager-example

Practical top-level integration example for Pulith crates.

This example is intentionally partial: it shows how to compose the crates into a small runtime/tool manager workflow without pretending to be a full package manager.

## What It Does

It behaves like a very small runtime/tool manager for archive-based resources.

- installs a local archive into a managed workspace
- installs a local file artifact into a managed workspace through the adapter helper path
- records lifecycle state and provenance
- activates the installed runtime through a simple portable activator
- inspects persisted state against store/filesystem reality
- produces repair plans for stale state facts
- produces prune plans that respect retention policy derived from state
- demonstrates a manager-like reconcile/apply loop that applies explicit repair actions and protected metadata pruning

The goal is not to be feature-complete. The goal is to show a practical multi-crate usage path that a real manager could grow from.

## How It Works

The example composes Pulith crates as a thin integration layer:

1. `pulith-resource`

- defines the runtime semantically (`resource-id`, version, locator)

2. `pulith-source` + `pulith-fetch`

- plans the source and fetches the archive into a workspace download area

3. `pulith-archive`

- extracts the fetched archive into a temporary extracted tree

4. `pulith-store`

- registers the extracted tree with provenance so it can be inspected and reused safely

5. `pulith-install`

- stages the install, commits it into the managed install root, and activates it

6. `pulith-state`

- records lifecycle facts, supports inspection, repair planning, and retention-aware cleanup planning

## What It Demonstrates

- semantic resource definition
- source planning and local fetch through `pulith-fetch`
- archive extraction through `pulith-archive`
- non-archive file install through `pulith-backend-example` adapter helpers (`FetchReceipt` -> `InstallSpec`)
- stored extract registration with provenance through `pulith-store`
- install + activation through `pulith-install`
- inspection and repair planning through `pulith-state`
- retention-aware prune planning through `pulith-state` + `pulith-store`
- policy-light inspect -> repair-apply -> retention-prune reconcile flow

## Commands

```bash
cargo run -p runtime-manager-example -- install-local-archive <resource-id> <version> <archive-path> <workspace-root>
cargo run -p runtime-manager-example -- install-local-file <resource-id> <version> <file-path> <workspace-root>
cargo run -p runtime-manager-example -- install-remote-archive <resource-id> <version> <archive-url> <workspace-root>
cargo run -p runtime-manager-example -- inspect <resource-id> <workspace-root>
cargo run -p runtime-manager-example -- repair-plan <resource-id> <workspace-root>
cargo run -p runtime-manager-example -- prune-plan <workspace-root> [all|installed-active|active]
cargo run -p runtime-manager-example -- reconcile <resource-id> <workspace-root> [all|installed-active|active]
```

## Example Session

```bash
cargo run -p runtime-manager-example -- install-local-file example/runtime 1.0.0 ./runtime.bin ./workspace
cargo run -p runtime-manager-example -- inspect example/runtime ./workspace
cargo run -p runtime-manager-example -- repair-plan example/runtime ./workspace
```

## Notes

- activation uses a simple pointer-file activator for portability
- the example prefers explicit composition over hidden policy
- this example lives outside `crates/` on purpose so it can validate real multi-crate usage
