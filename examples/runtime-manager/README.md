# runtime-manager-example

Practical top-level integration example for Pulith crates.

This example is intentionally partial: it shows how to compose the crates into a small runtime/tool manager workflow without pretending to be a full package manager.

## What It Demonstrates

- semantic resource definition
- source planning and local fetch through `pulith-fetch`
- archive extraction through `pulith-archive`
- stored extract registration with provenance through `pulith-store`
- install + activation through `pulith-install`
- inspection and repair planning through `pulith-state`
- retention-aware prune planning through `pulith-state` + `pulith-store`

## Commands

```bash
cargo run -p runtime-manager-example -- install-local-archive <resource-id> <version> <archive-path> <workspace-root>
cargo run -p runtime-manager-example -- inspect <resource-id> <workspace-root>
cargo run -p runtime-manager-example -- repair-plan <resource-id> <workspace-root>
cargo run -p runtime-manager-example -- prune-plan <workspace-root> [all|installed-active|active]
```

## Notes

- activation uses a simple pointer-file activator for portability
- the example prefers explicit composition over hidden policy
- this example lives outside `crates/` on purpose so it can validate real multi-crate usage
