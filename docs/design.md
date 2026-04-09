# Pulith Design Document

## Vision

Pulith is a Rust ecosystem for resource management primitives: version selection, source planning, fetching, verification, storage, extraction, installation, activation, and persistent state.

The project is mechanism-first. It should give tool authors reliable building blocks without forcing one package format, backend, or manager model.

In practical terms, Pulith is meant to be the substrate for tools that need to manage external resources with explicit, inspectable behavior rather than ad hoc script glue.

## Why It Exists

Most tools that manage external resources end up rebuilding the same layers:

- version parsing and selection
- source planning and fetching
- content verification
- atomic filesystem updates
- persistent state and activation
- cross-platform behavior

Pulith exists to make those layers reusable, composable, and correct.

## What A User Should Expect

If you adopt Pulith as a library stack, you should expect three things.

### 1. Clear API Shape

The public API should read like a resource-management pipeline:

- describe resources semantically
- derive and plan sources
- fetch and verify content
- register artifacts or extracted trees
- install and activate explicitly
- persist, inspect, repair, and retain lifecycle state

You should not need to reconstruct raw path/key/record glue at every layer just to compose a normal flow.

### 2. Clear Principles

Pulith should be predictable because its crates follow a small number of rules:

- primitives stay primitive
- workflow crates compose lower layers instead of absorbing them
- semantic crates describe facts and queries rather than hard-coding manager policy
- side effects stay explicit in fetch/store/install/state operations
- receipts, records, and reports should make behavior explainable after the fact

### 3. Clear Execution Story

Pulith should make it obvious how work happens:

- `pulith-resource` describes what a thing is
- `pulith-source` describes where it may come from
- `pulith-fetch` turns a plan into bytes
- `pulith-archive` turns archive bytes into a materialized tree
- `pulith-store` makes artifacts/extracts reusable and provenance-aware
- `pulith-install` places materialized content into an install root and optionally activates it
- `pulith-state` persists lifecycle facts and supports inspection, repair, and retention planning

That execution story should remain visible in both docs and public APIs.

## User-Centered Reframe

If I were building a real resource manager on top of Pulith - a system package manager, config manager, runtime/tool installer, or plugin manager - I would want a few things very clearly.

### Expected Behavior

- declare resources semantically instead of stitching together raw paths and file names
- resolve, fetch, verify, store, install, and activate through predictable composable steps
- support both file-like and directory-like resources
- support reinstall, upgrade, rollback, activation switching, and recovery after interruption
- preserve provenance so I can explain where a resource came from and how it was materialized
- adapt to different manager styles without forcing one package model or repository design
- detect, reconcile, repair, and reapply state when reality drifts from persisted facts
- handle ownership, conflicts, retention, and cleanup without guessing blindly
- support explainable decisions and auditable outcomes for operator-facing tools

In short: Pulith should help users build real managers, not just isolated crate demos.

### Expected Safety

- atomic replace/install behavior so partial writes do not become live state
- explicit verification and trust hooks rather than hidden best-effort checks
- rollback and recovery paths that restore both filesystem state and persisted lifecycle facts
- cross-platform behavior that is explicit where semantics differ, especially on Windows
- explicit effect boundaries so helpers do not hide network, filesystem, or activation side effects

### Expected Consistency

- the same semantic resource identity should drive planning, storage, state, and installation
- provenance should survive handoff boundaries instead of disappearing during workflow transitions
- lifecycle transitions should be expressed through small semantic operations, not repeated record reconstruction
- helpers should be named by role (`plan`, `resolve`, `lookup`, `register`, `record`, `activate`) and behave consistently across crates
- convenience should reduce orchestration, not introduce hidden policy
- repair, reconcile, and prune behavior should use the same identity/provenance model as install and activation

## Design Principles

1. atomicity first
2. composability over framework-style policy
3. cross-platform behavior as a primary constraint
4. semantic APIs over raw stringly glue
5. type-driven correctness where workflow ordering matters
6. proof-carrying validation where repeated checks would spread through the codebase
7. thin integration surfaces between crates instead of monolithic abstraction
8. semantic consistency across fetch, store, install, activate, and state
9. ergonomic helpers should remove repetition without hiding effects or policy

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

### 1a. Support and Template Layer

- `pulith-shim-bin`: thin shim-binary helper/template for turning `pulith-shim` resolution into a runnable executable boundary

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

### 6. Top-Level Integration Examples

- `examples/runtime-manager/`: partially practical multi-crate example that exercises the public workflow outside `crates/`

## Active Crates

| crate | maturity | role |
|-------|----------|------|
| `pulith-platform` | stable core | cross-platform helpers |
| `pulith-version` | stable core | version parsing and selection |
| `pulith-shim` | stable core | shim resolution |
| `pulith-shim-bin` | support/template | shim-binary execution helper |
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

The main issue is no longer missing layers. The main issue is integration maturity between the newer crates, especially where callers still have to reconstruct semantic queries, lifecycle transitions, or composed plans by hand.

### What Is Working

- the crate split still maps well to real resource-management concerns
- the philosophy is still consistent across the workspace
- type-state is used in the right places so far: resource resolution, source planning, and install flow
- proof-carrying validation is present where it matters most today (`ValidUrl`, `ValidDigest`)
- the engineering baseline is strong: formatting, tests, docs, and CI all work across the workspace
- the composed system is starting to behave like a usable resource-management substrate rather than a loose set of primitives

### Main Design Debt

- `pulith-fetch` still mixes a dependable simple path with less mature advanced policy surfaces
- some remaining bridges between `pulith-source`, `pulith-fetch`, `pulith-store`, and `pulith-install` are still phrased as caller-side choreography instead of reusable semantic helpers
- some lifecycle transitions still require too much caller-side record construction even though the state model itself is already stable enough to support better helpers
- `pulith-version` now has typed requirement and preference primitives, but still needs stronger integration across real planners and adapter decisions
- `pulith-state` is intentionally simple, but snapshot rewriting may become expensive for larger registries
- some user-facing guarantees are still implicit rather than clearly stated: what is safe to retry, what is reversible, what provenance is durable, and where cross-platform semantics intentionally diverge
- discovery, reconciliation, ownership, retention, repair, and explainability are not yet first-class behavior families in the current design story even though real managers need them

## Current Goal Reframing

The next design goal should not be "add more crates" or even simply "add more typed bridges".

The current goal should be:

- keep the existing crate boundaries
- reduce repeated orchestration at crate boundaries
- make semantic queries and lifecycle transitions first-class helpers
- extend behavior only where the additional semantics are broadly reusable
- improve ergonomics and consistency without hiding effects or policy
- make Pulith feel trustworthy to build real managers on top of, not just correct in isolated crate tests

This matches `docs/AGENT.md` more closely:

- Functions First: move repeated orchestration into focused helpers with explicit inputs/outputs
- Pure Core, Impure Edge: keep policy-light semantic transformations reusable while leaving I/O at the boundary
- Composition Over Orchestration: favor pipeline helpers and record/query helpers over ad hoc caller loops
- Explicit Effects: keep fetch/store/install/state side effects visible rather than burying them in large controller types

## Additional Behavior Families To Design For

Install behavior is only one slice of a real resource manager. Pulith should also explicitly account for the following behavior families.

### Discovery and Inspection

- detect already-installed resources
- inspect store/state/install/activation facts without mutating them
- compare persisted facts against on-disk reality

### Drift, Reconciliation, and Repair

- detect when resources, installs, or activation targets drift from desired state
- reconcile persisted state with reality safely
- repair broken activations, partial installs, or stale metadata without inventing hidden policy

### Ownership, Conflict, and Retention

- express which resource owns which install root, activation target, or stored artifact
- detect collisions and ambiguous ownership instead of silently overwriting
- support pruning and retention with explicit ownership/protection semantics

### Migration and Convergence

- support install-root migration, state/schema migration, and semantic renames where needed
- support desired-state -> diff -> converge flows for config-manager-like use cases
- support batch or grouped application where several resource operations need one higher-level plan

### Explainability and Auditability

- explain why a source, version, store key, or activation target was chosen
- keep enough durable provenance and lifecycle history to support audit/debug flows
- prefer receipts, records, and small semantic events over opaque orchestration state

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
- `pulith-store` and `pulith-state` should expose more semantic lookup/transition helpers so callers stop reconstructing keys and lifecycle mutations manually
- future discovery/reconciliation/retention helpers should compose existing crates rather than collapse their roles into `pulith-install`

### Crates That Need the Most Refactor Attention

- `pulith-fetch`: make advanced execution modes explicit and trustworthy
- `pulith-install`: keep sharpening reusable workflow helpers while avoiding policy creep
- `pulith-version`: deepen integration of requirement matching and preference selection across callers
- `pulith-platform`: keep the crate narrow so it remains a host/platform helper layer rather than a grab-bag system toolkit
- `pulith-state`: improve transition ergonomics now, monitor snapshot scaling continuously, and avoid premature storage redesign until benchmarks justify change
- `pulith-store`: improve semantic lookup/provenance ergonomics without absorbing install policy
- the workspace overall: define where reconciliation, ownership, retention, and audit semantics belong without turning the install layer into a catch-all

## Role Refinement For Core Support Crates

### `pulith-version`

- should stay a pure primitive crate
- should own generic comparison, matching, and preference selection only
- resource-specific alias meaning and planner-specific policy mapping should stay outside it
- it should help planners and adapters choose, not decide repository/package-manager policy

### `pulith-platform`

- should stay a narrow host/platform helper crate
- good scope: OS/arch/triple parsing, directory conventions, shell metadata, lightweight process helpers
- bad scope: richer package-manager policy, service management, installer orchestration, or broad system administration helpers
- the crate should expose predictable normalization and command/shell behavior, not become a generic toolbox

### `pulith-shim-bin`

- should be treated as a support/template crate, not a semantic model or workflow crate
- its role is to bridge `pulith-shim` into a runnable executable boundary with minimal policy
- if it grows substantially, it should evolve either into a clearer support crate or move toward examples/templates rather than distorting the primitive/workflow layers

## Practicality and Ergonomics

The current crate layout is practical for internal composition, but still slightly too verbose for end users.

The next ergonomics step should be better typed bridges, not fewer crates.

Focus areas:

- standardize receipts across fetch, store, extract, install, and activate
- reduce ad hoc path conversion between layers
- add ready-made adapters for common end-to-end flows
- make lifecycle persistence less repetitive for callers
- improve end-to-end examples that span multiple crates
- keep public helpers named by semantic role (`plan`, `derive`, `lookup`, `register`, `activate`, `record`) rather than by implementation detail

## Required Guarantees For Real Managers

For Pulith to be a strong substrate for system package managers, config managers, and plugin managers, it should converge on the following guarantees.

### Behavioral Guarantees

- repeatable end-to-end flows for fetch/store/install/activate
- explicit upgrade and rollback semantics
- provenance continuity across workflow boundaries
- thin but complete helpers for common file, archive, and shim-based flows
- support for inspect/detect/reconcile/repair flows without requiring custom ad hoc state plumbing

### Safety Guarantees

- atomic install-state updates where possible
- explicit failure surfaces for platform-specific activation constraints
- recoverable interrupted operations
- durable persistence of the facts needed to explain and recover resource state
- explicit conflict and ownership boundaries before destructive cleanup or replacement

### Consistency Guarantees

- semantic resource identity remains the backbone of the system
- version intent, provenance, install facts, and activation facts stay aligned
- helpers in adjacent crates compose naturally without caller-side translation layers
- crate roles remain narrow and predictable
- discovery, reconciliation, pruning, and repair use the same semantic resource model as initial installation

## Concrete Refactor Plan

The next refactor work should proceed in four compact phases.

### Phase 1: Semantic Query and Transition Ergonomics

- continue turning repeated `ResolvedResource` -> key/lookup glue into `pulith-store` helpers
- continue turning repeated lifecycle record updates into `pulith-state` helpers
- add paired metadata/provenance query helpers where callers still fetch records and unpack them manually
- keep these helpers policy-light and deterministic

### Phase 2: User-Facing Workflow Consistency

- make archive, file, extract, install, activate, upgrade, and rollback paths feel structurally similar to callers
- extend helpers only where they remove real repeated orchestration for manager authors
- preserve explicit effects and keep policy choices visible

### Phase 3: Discovery, Reconciliation, and Ownership

- design semantic inspect/detect/reconcile helpers instead of forcing every manager to rebuild drift handling
- add ownership/conflict/retention semantics where destructive actions otherwise become ambiguous
- keep these helpers explanatory and policy-light rather than turning them into a monolithic package model

### Phase 4: Planner and Adapter Integration

- thread `VersionSelector` -> `SelectionPolicy` helpers into more backend/example planning paths
- add thin adapter helpers only where they remove real repetition across composed flows
- keep `pulith-source` and `pulith-install` as composition points rather than monolithic orchestrators

### Phase 5: Safety, Contract Hardening, and Evidence

- add cross-platform tests for the remaining Windows activation differences
- make retry/recovery/rollback guarantees clearer in both tests and docs
- rerun performance evidence for copy/hardlink/state-growth decisions on steadier environments
- redesign storage internals only if those measurements show the current model is the bottleneck

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
- install workflow tests now cover interrupted install recovery through backup/restore snapshots
- `pulith-install` now includes criterion benchmarks for large fetch/store/install and archive extract/store/install flows
- `pulith-fetch` now includes criterion benchmarks for multi-source priority and race strategy overhead
- transition benchmarks now measure hardlink-or-copy against copy-only for same-device store/install artifact movement
- current benchmark runs indicate the crossover favors copy-only for small files but shifts toward hardlink-or-copy once artifacts reach larger multi-megabyte sizes
- store/import/install paths now apply that evidence with a size-threshold strategy rather than always attempting hardlinks first
- additional threshold-variant benchmarks now exist for tuning, though current results are noisy enough that the chosen cutoff should remain provisional until repeated on calmer filesystems/CI runners
- `pulith-install` now exposes a typed fetch-receipt to stored-install-input bridge so callers need less manual path and file-name glue
- the fetch -> store bridge in `pulith-install` now preserves `StoreProvenance` (origin URL/local path and optional fetch checksum metadata) instead of dropping receipt context during import
- `pulith-install` now also exposes a typed archive-extraction -> stored-extract bridge so archive metadata is preserved when extracted trees are registered in `pulith-store`
- resource/source/install integration now carries more version intent: source specs can be derived from resources directly, and install staging validates resolved versions against exact and requirement selectors
- `pulith-source` now also exposes direct planned-source constructors from locators and requested/resolved resources, reducing the remaining source -> fetch planning glue for workflow callers and thin backends
- `pulith-resource` now derives shared `pulith-version::SelectionPolicy` values from exact, requirement, and common alias selectors, so version preference intent is no longer trapped in stringly selector handling
- upgrade installs now behave more intentionally: they require an existing install root and preserve `Active` lifecycle state when replacing an already-active install in place
- rollback now restores the prior activation snapshot as well as the prior install/root record state, including cleanup of activation targets created only by the reverted install
- workspace-level integration tests now cover a full local archive fetch -> extract -> store -> install path, not just direct extraction and direct local-file fetch cases
- `pulith-install` now has a typed fetched-archive-extraction -> stored-extract helper, so callers no longer need to manually stitch together fetch receipts, archive reports, store registration, and provenance merging for that common path
- recent internal cleanup also reduced repeated selector/planning/activation/provenance code across `pulith-resource`, `pulith-source`, and `pulith-install`, keeping the current API surface lighter-weight to maintain
- `pulith-store` now provides semantic lookup helpers keyed by `ResolvedResource` + `KeyDerivation`, which fits the current direction of reducing path/key reconstruction without moving install policy into the store layer
- `pulith-state` now provides semantic resolved-resource upsert helpers and richer patch composition, which reduces repeated lifecycle record construction while keeping persistence policy-light
- `pulith-state` now also provides per-resource state capture/restore helpers, which makes rollback and recovery flows more composable and less dependent on workflow-local record juggling
- `pulith-state` now also provides initial per-resource inspection helpers, which turns detect/explain behavior into an explicit semantic operation rather than an install-local concern
- `pulith-state` now also provides explicit per-resource repair planning/application for stale facts, which starts to turn reconciliation into a semantic operation without forcing policy-heavy repair into `pulith-install`
- `pulith-state` now also detects activation ownership conflicts, which begins to model shared-target conflict semantics without forcing cleanup policy into install or state persistence itself
- `pulith-state` now also exposes store-key references, and `pulith-store` can now plan protected prune operations from those references, which starts to make retention/prune safety explicit instead of implicit
- `pulith-state` now also exposes lifecycle-based store retention helpers, which lets callers derive protected prune sets from semantic state rather than hand-maintaining key lists
- `pulith-state` now also composes those retention helpers with store orphan inspection to produce explicit metadata retention plans, which moves cleanup planning closer to semantic state without collapsing store responsibilities
- `pulith-resource` now also offers preferred resolved-candidate selection through shared version-selection policy, and the backend example carries that helper into a real adapter-facing path
- workspace integration coverage now includes repeated copy-based activation over the same file target, which strengthens the explicit contract story for non-link activator behavior across platforms
- workspace integration coverage now also includes archive-inclusive replace/activate/rollback recovery, which strengthens the recovery contract beyond direct directory materialization paths
- workspace integration coverage now also includes repeated symlink-based file activation over the same target, which rounds out the contract story for file activation across both link-based and copy-based modes
- `pulith-store` now also supports orphaned-metadata inspection before pruning, which makes cleanup behavior more inspectable and better aligned with the emerging reconciliation/ownership story
- activation contract tests now cover replacement of pre-existing file and directory targets, strengthening cross-platform expectations around symlink/junction-backed activation
- activation target replacement now treats existing links more carefully, using link-aware removal so reinstall flows can replace prior symlink/junction activations without tripping over platform-specific directory-link semantics
- removal paths now clear Windows read-only attributes before replacing prior install or activation targets, so replacement flows are less likely to fail on permission-shaped leftovers from earlier installs
- Windows file activations now report a dedicated symlink-privilege error when link creation is denied, making the remaining platform-specific activator choice more explicit to callers
- `pulith-install` now includes explicit copy-based file activators as a policy choice beside link activators, so Windows callers can opt into file-copy activation without making fallback behavior implicit

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
- reconciliation and ownership tests:
  - detect drift between persisted state and filesystem reality
  - reject ambiguous ownership/conflict cases before destructive cleanup
  - explain prune/repair decisions through durable facts
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

It needs stronger semantic consistency and more trustworthy composition between the crates it already has.

The split is mostly correct. The next phase should therefore focus on integration tightening, manager-facing ergonomics, stronger safety guarantees, clearer consistency semantics, and performance evidence for the composed system.

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
