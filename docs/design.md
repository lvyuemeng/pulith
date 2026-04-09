# Pulith Design Document

## Vision

Pulith is a Rust ecosystem for resource management primitives: version selection, source planning, fetching, verification, storage, extraction, installation, activation, and persistent state.

The project is mechanism-first. It should give tool authors reliable building blocks without forcing one package format, backend, or manager model.

## Why It Exists

Most tools that manage external resources end up rebuilding the same layers:

- version parsing and selection
- source planning and fetching
- content verification
- atomic filesystem updates
- persistent state and activation
- cross-platform behavior

Pulith exists to make those layers reusable, composable, and correct.

## Design Principles

1. atomicity first
2. composability over framework-style policy
3. cross-platform behavior as a primary constraint
4. semantic APIs over raw stringly glue
5. type-driven correctness where workflow ordering matters
6. proof-carrying validation where repeated checks would spread through the codebase
7. thin integration surfaces between crates instead of monolithic abstraction

## Current Architecture

Pulith now has four logical layers:

### 1. Primitive Layer

- `pulith-platform`: small cross-platform helpers
- `pulith-version`: version parsing, comparison, and selection
- `pulith-fs`: atomic file and workspace primitives
- `pulith-verify`: verification primitives
- `pulith-archive`: archive extraction primitives
- `pulith-fetch`: transfer execution primitives
- `pulith-shim`: shim resolution primitives

### 2. Semantic Model Layer

- `pulith-resource`: shared resource semantics
- `pulith-resource`: shared resource semantics and optional trust policy description
- `pulith-store`: local artifact and extract storage
- `pulith-state`: persistent resource lifecycle state

### 3. Workflow Layer

- `pulith-install`: typed installation and activation workflow

### 4. Source Layer

- `pulith-source`: source definitions, planning, and expansion

### 5. Backend Example Layer

- `pulith-backend-example`: thin adapter example built on top of resource, source, and install crates

## Active Crates

| crate | maturity | role |
|-------|----------|------|
| `pulith-platform` | stable core | cross-platform helpers |
| `pulith-version` | stable core | version parsing and selection |
| `pulith-shim` | stable core | shim resolution |
| `pulith-fs` | maturing core | atomic filesystem and workspace primitives |
| `pulith-verify` | stable core | content verification |
| `pulith-archive` | maturing core | archive extraction |
| `pulith-fetch` | maturing core | transfer execution |
| `pulith-resource` | emerging core | resource semantics |
| `pulith-store` | emerging core | artifact storage |
| `pulith-state` | emerging core | persistent lifecycle state |
| `pulith-install` | emerging core | installation workflow |
| `pulith-source` | emerging core | source planning |
| `pulith-backend-example` | example | adapter-first backend composition |

## Dependency Shape

The dependency shape is intentionally layered:

- primitives should stay independently usable
- semantic crates should stay policy-light
- workflow crates should compose lower layers rather than absorb them

Current high-level relationships:

- `pulith-fetch` depends on `pulith-fs` and `pulith-verify`
- `pulith-archive` depends on `pulith-fs`
- `pulith-store` depends on `pulith-fs` and `pulith-resource`
- `pulith-state` depends on `pulith-fs`, `pulith-resource`, and `pulith-store`
- `pulith-install` depends on `pulith-fs`, `pulith-resource`, `pulith-store`, and `pulith-state`
- `pulith-source` depends on `pulith-resource`
- `pulith-backend-example` depends on `pulith-resource`, `pulith-source`, and `pulith-install`

## Current Assessment

The architecture is broadly correct.

The main issue is no longer missing layers. The main issue is integration maturity between the newer crates.

### What Is Working

- the crate split still maps well to real resource-management concerns
- the philosophy is still consistent across the workspace
- type-state is used in the right places so far: resource resolution, source planning, and install flow
- proof-carrying validation is present where it matters most today (`ValidUrl`, `ValidDigest`)
- the engineering baseline is strong: formatting, tests, docs, and CI all work across the workspace

### Main Design Debt

- `pulith-fetch` still mixes a dependable simple path with less mature advanced policy surfaces
- the bridge between `pulith-source`, `pulith-fetch`, `pulith-store`, and `pulith-install` is thinner than it should be
- some lifecycle transitions still require too much caller-side record construction and path-level glue
- `pulith-version` now has typed requirement and preference primitives, but still needs stronger integration across the full resource pipeline
- `pulith-state` is intentionally simple, but snapshot rewriting may become expensive for larger registries

## Crate Re-evaluation

### Keep As Separate Crates

These boundaries are still good and should remain:

- `pulith-fs`, `pulith-verify`, `pulith-archive`, `pulith-fetch`
- `pulith-resource`, `pulith-store`, `pulith-state`
- `pulith-install`, `pulith-source`

Merging these would reduce composability and make the shared model more rigid.

### Tighten Integration Without Merging

- `pulith-source` should feed planned candidates directly into `pulith-fetch`
- `pulith-fetch` should emit receipts that convert naturally into store handles
- `pulith-store` and `pulith-install` should share clearer handoff types
- `pulith-install` should gain shim-oriented activators without embedding shim policy into `pulith-shim`
- `pulith-resource` should keep tightening its version selector around `pulith-version` semantics

### Crates That Need the Most Refactor Attention

- `pulith-fetch`: make advanced execution modes explicit and trustworthy
- `pulith-install`: add upgrade and rollback semantics
- `pulith-version`: deepen integration of requirement matching and preference selection across callers
- `pulith-state`: monitor snapshot scaling and avoid premature complexity until benchmarks justify change

## Practicality and Ergonomics

The current crate layout is practical for internal composition, but still slightly too verbose for end users.

The next ergonomics step should be better typed bridges, not fewer crates.

Focus areas:

- standardize receipts across fetch, store, extract, install, and activate
- reduce ad hoc path conversion between layers
- add ready-made adapters for common end-to-end flows
- make lifecycle persistence less repetitive for callers
- improve end-to-end examples that span multiple crates

## Efficiency Direction

The main efficiency risks are in composition, not in the small primitive crates.

Priority areas:

- reduce repeated copying across fetch -> store -> install flows
- prefer hardlink, rename, or direct registration where semantics allow it
- benchmark state snapshot rewriting under realistic registry sizes
- benchmark advanced fetch modes instead of assuming concurrency helps
- avoid rematerializing extracted trees when store ownership is already sufficient

Current progress:

- store import and install staging now prefer hardlink-or-copy where the filesystem allows it
- copy-heavy transitions still need measurement, but the default path is now less wasteful on same-device flows
- `pulith-state` now has dedicated growth benchmarks so snapshot rewriting cost can be measured before changing storage architecture

## Integrated Testing Direction

The next quality step is more integrated testing, not more crate surface.

Required test layers:

- end-to-end pipeline tests:
  - resource -> source -> fetch -> store -> install -> activate
  - resource -> source -> fetch -> archive -> store -> install
- cross-platform contract tests:
  - windows replace behavior
  - symlink/junction activation differences
  - path sanitization behavior
- persistence and recovery tests:
  - interrupted install recovery
  - partial state recovery
  - repeated activation idempotence
- performance tests:
  - large artifact fetch/extract/install
  - store registration of large trees
  - state growth behavior

## Refactor Priorities

1. connect `pulith-source` planning directly to `pulith-fetch`
2. define shared receipts and handoff types across fetch, store, archive, and install
3. add replace / upgrade / rollback semantics to `pulith-install`
4. integrate `pulith-version` selection semantics more directly into source and install flows
5. add workspace-level end-to-end integration tests

## Architectural Conclusion

Pulith does not need fewer crates right now.

It needs stronger typed bridges between the crates it already has.

The split is mostly correct. The next phase should therefore focus on integration tightening, better end-to-end ergonomics, stronger advanced-path guarantees, and performance evidence for the composed system.

## Out of Scope

- package format definitions unless they are broadly reusable
- repository hosting
- authentication servers
- license management
- dependency resolution

## References

- `README.md`
- `docs/AGENT.md`
- `docs/design/platform.md`
- `docs/design/version.md`
- `docs/design/shim.md`
- `docs/design/fs.md`
- `docs/design/verify.md`
- `docs/design/archive.md`
- `docs/design/fetch.md`
- `docs/design/resource.md`
- `docs/design/store.md`
- `docs/design/state.md`
- `docs/design/install.md`
- `docs/design/source.md`
