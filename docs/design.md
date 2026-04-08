# Pulith Design Document

## vision

**resource management primitives for rust**

a crate ecosystem providing everything a rust tool needs to fetch, verify, store, and track external resources - packages, config files, tools, plugins, or any versioned artifacts.

> "everything a rust tool needs to manage versioned external resources - built with best practices."

## why this exists

80% of tools that manage external resources reinvent the same primitives:
- version parsing and comparison
- http downloads with progress and verification
- atomic file operations and staging
- state tracking with rollback
- cross-platform correctness

this ecosystem provides battle-tested building blocks so developers can focus on their unique value proposition.

## target use cases

- **version managers**: rustup, nvm, pyenv, goenv
- **config managers**: dotfiles, config sync, .env managers
- **plugin managers**: vim-plug, neovim plugins, ide extensions
- **registry servers**: npm registry mirrors, pypi caches, internal registries
- **tool managers**: sdk managers, cli tool installers
- **artifact repositories**: container image caches, binary caches

## design principles

1. **atomicity**: all state-changing operations are atomic with rollback
2. **composability**: crates can be used independently or together
3. **cross-platform**: consistent behavior across windows, macos, linux
4. **extensibility**: higher-layer patterns (sources, backends) designed later
5. **best practices**: security, verification, and correctness baked in
6. **mechanism-only**: provide primitives to fetch, store, stage, and track external resources.

## completed crates

| crate | status | purpose | key features |
|-------|--------|---------|--------------|
| `pulith-platform` | ✅ | cross-platform helpers | os/arch detection, shell detection, path manipulation |
| `pulith-version` | ✅ | version parsing | semver, calver, partial versions with comparison |
| `pulith-shim` | ✅ | shim generation | targetresolver trait, composable resolvers |
| `pulith-fs` | ✅ | atomic filesystem | atomic_write, workspace, transaction, replace_dir |
| `pulith-verify` | ✅ | content verification | hasher trait, verifiedreader, sha256/blake3 |
| `pulith-archive` | ✅ | archive handling | format detection, streaming extraction, zip-slip protection |
| `pulith-fetch` | ✅ | http downloading | tee-reader streaming, atomic placement, progress callbacks |

## crate relationships

**dependency matrix:**

| crate | dependencies |
|-------|--------------|
| `pulith-fetch` | `pulith-fs`, `pulith-verify` |
| `pulith-archive` | `pulith-fs` (optional: `pulith-verify`) |
| `pulith-verify` | none (standalone) |
| `pulith-fs` | none (standalone) |
| `pulith-platform` | none (standalone) |
| `pulith-version` | none (standalone) |
| `pulith-shim` | none (standalone) |

## crate descriptions

### pulith-platform ✅
cross-platform helpers:
- os and distribution detection (windows, macos, linux distros)
- architecture detection (x86, x64, arm variants)
- shell detection and invocation
- path manipulation
- home and temp directory resolution

**design**: `docs/design/platform.md`

### pulith-version ✅
version parsing and comparison for multiple formats:
- **semver**: semantic versioning (1.2.3, 1.2.3-alpha+build)
- **calver**: calendar versioning (2024.01, 2024.01.15)
- **partial**: partial versions (18, 3.11, lts)

**design**: `docs/design/version.md`

### pulith-shim ✅
shim generation for version switching:
- targetresolver trait for custom resolution policies
- pairresolver and tripleresolver for fallback/chain patterns
- compile-time generic resolution (zero-cost abstraction)

**design**: `docs/design/shim.md`

### pulith-fs ✅

role: cross-platform atomic filesystem primitives. mechanism only: it does not know what a "tool" is. it only knows how to move bytes safely.

**core primitives:**

- `atomic_write(path, content)`: writes to a temp file, fsyncs, then renames.

- `atomic_symlink(target, link_path)`: creates a new link, then renames over the old one.

- `replace_dir(src, dest)`: atomic directory replacement. On Windows, handles the complex retry/rename dance required when files are locked.

- `hardlink_or_copy(src, dest)`: optimization primitive.

**workspace** (formerly stage)

role: a transactional workspace for preparing resources. philosophy: installation is a transaction. it either happens completely or not at all. mechanism only: no policy, no format enforcement.

example:

```rust
let workspace = workspace::new(temp_dir)?;

workspace.write("bin/tool", bytes)?;
workspace.create_dir("lib")?;
workspace.create_dir_all("nested/deep")?;

// this atomically moves the staged directory to the final destination.
// if this fails, the workspace is dropped and the temp dir is cleaned up.
workspace.commit(final_destination_path)?;
```

**transaction** (formerly state)

role: concurrent-safe read-modify-write on a persistent file, without enforcing a schema. concrete-independent: it deals in opaque bytes only.

example:

```rust
let tx = transaction::open("registry.json")?;

// blocks other processes, reads current content, allows modification,
// and atomically writes back.
tx.execute(|bytes| {
    let data: MyCustomSchema = MyCustomSchema::from(bytes);
    data.last_update = now();
    Ok(data.to_bytes())
})?;
```

mechanism: handles file locking (flock/lockfile), read-modify-write cycles, and atomic replacement. it prevents two instances of your tool from corrupting the registry.

**design**: `docs/design/fs.md`

### pulith-verify ✅

content verification primitives for downloaded artifacts:
- **zero-copy verification**: cpu cache touches bytes only once (hashing + i/o)
- **hasher trait**: minimal interface for custom implementations (hardware accelerators, etc.)
- **verifiedreader**: streaming verification wrapper for any `read` source
- **built-in hashers**: sha256 (default via `sha2` crate), blake3 (optional via feature flag)

example:

```rust
use pulith_verify::{VerifiedReader, Sha256Hasher};

let expected_hash = hex::decode("...")?;
let hasher = Sha256Hasher::new();
let mut reader = VerifiedReader::new(file, hasher);

std::io::copy(&mut reader, &mut dest)?;
reader.finish(&expected_hash)?;
```

**design**: `docs/design/verify.md`

### pulith-archive ✅

archive extraction and creation primitives:
- **format detection**: magic number inspection (zip, tar.gz, xz, zstd)
- **single-pass extraction**: streams entries without loading entire archive into memory
- **path sanitization**: lexical normalization prevents zip-slip attacks
- **transaction-aware**: works with `pulith-fs::workspace` for atomic extraction

example:

```rust
use pulith_archive::{ArchiveFormat, Compression, unpacker};

let format = unpacker::detect_format_from_file(archive_path)?;

match format {
    ArchiveFormat::Tar(Compression::Gzip) => {
        let decoder = flate2::read::GzDecoder::new(file);
        unpacker::extract_tar_gz(decoder, destination)?;
    }
    _ => return Err(Error::UnsupportedFormat),
}
```

**design**: `docs/design/archive.md`

### pulith-fetch ✅

http downloading with streaming verification and atomic placement:
- **tee-reader pattern**: single-pass streaming from network → filesystem with concurrent sha256 hashing
- **atomic placement**: uses `pulith-fs::workspace` for guaranteed cleanup on error
- **httpclient trait**: abstraction for testability (reqwest implementation via feature flag)
- **progress callbacks**: mechanism-only, caller handles ui/throttling

example:

```rust
use pulith_fetch::{Fetcher, ReqwestClient, FetchOptions};

let client = ReqwestClient::new()?;
let fetcher = Fetcher::new(client, "/tmp");

let options = FetchOptions::default()
    .checksum(Some(expected_hash));

let path = fetcher.fetch(url, destination, options).await?;
```

**design**: `docs/design/fetch.md`

## design directions (deferred)

these areas require further design when needed:

### backend abstractions
- trait for package managers
- multi-manager orchestration
- flag resolution patterns

### migration and upgrades
- schema migration for registries
- in-place upgrade patterns
- backup and restore

## current assessment

the project direction is still valid: pulith should remain a mechanism-first ecosystem for resource management primitives, especially around version selection, artifact fetching, verification, storage, extraction, and activation.

the current design is suitable to proceed further, but it should be treated as a maturing foundation rather than a completed ecosystem.

### what is working well

- the crate split is still sensible: `version`, `fs`, `verify`, `archive`, `fetch`, `platform`, and `shim` cover the main primitive layers needed by version managers and tool installers.
- the overall philosophy remains correct: pure-ish primitives, explicit effects, cross-platform behavior, and mechanism over policy.
- the workspace is now close to a usable baseline for further work because formatting, tests, docs, and CI are in place.

### current defects and design debt

- some crate descriptions above overstate completeness, especially for `pulith-fs` and `pulith-fetch`.
- `pulith-fs` currently exposes a smaller `workspace` and `transaction` API than this document describes; the design intent is ahead of the implementation.
- `pulith-fetch` contains useful primitives, but several advanced capabilities are still partial or scaffold-level rather than production-complete:
  - retry policy is not yet a first-class execution model
  - multi-source behavior is not yet a trustworthy policy engine
  - resumable and conditional fetching are not yet fully modeled end-to-end
  - bench and extra-target hygiene still need cleanup before `clippy --all-targets` becomes the default gate
- cross-platform support is improving, but the design should continue assuming Windows behavior is a first-class constraint rather than a later validation pass.
- the docs need periodic reconciliation with the code so design intent and public API do not drift apart.

### what can be extended next

#### 1. strengthen the core primitives

- expand `pulith-fs::workspace` into the richer transactional staging API described here (`write`, `create_dir`, `create_dir_all`, manifest/report helpers).
- expand `pulith-fs::transaction` toward a true read-modify-write executor for opaque state files.
- harden `pulith-archive` around creation APIs, metadata preservation, and more explicit symlink / permission policy.

#### 2. turn fetch into a reliable resource pipeline

- make retries explicit and composable instead of option-only.
- make source selection a real planning layer with priority, race, mirror health, and consistency verification.
- complete resumable and conditional fetching around persistent metadata, checkpoint validity, and append-safe writes.
- separate transport concerns from fetch policy more clearly so `pulith-fetch` can support more backends over time.

#### 3. add higher-level resource management crates

the next meaningful expansion should not be more miscellaneous utilities. it should be a thin higher layer built on the current primitives.

possible additions:

- `pulith-store`: canonical local artifact store, content-addressed or version-addressed layouts, retention, and lookup.
- `pulith-resource`: typed description of an external resource (identity, version, source, checksum, unpack policy, install policy).
- `pulith-state`: persistent registries for installed resources, active versions, and provenance.
- `pulith-install`: installation / activation transaction that composes `fetch + verify + archive + fs + shim`.
- `pulith-source`: source abstractions for http releases, git-based artifacts, local files, and mirrors.

#### 4. improve version-centric workflows

because the project goal explicitly includes version-oriented resource management, the version layer should eventually support more than parsing.

future direction:

- version requirement matching and selection
- preference rules (`latest`, `lts`, exact, compatible, pinned)
- stable ordering across semver, calver, and partial versions
- source-side resolution hooks for choosing a concrete artifact from a version query

## proceed / no-go decision

yes, the project can proceed further.

however, the next phase should follow this order:

1. align docs with the current public API and actual maturity level
2. complete the missing core behavior in `pulith-fs`, `pulith-archive`, and `pulith-fetch`
3. introduce one higher-level resource crate only after the primitives are stable enough to compose cleanly

the main risk is not that the architecture is wrong. the main risk is expanding sideways before the core contracts are stable.

## next design plan

### phase 1 - stabilize primitives

- keep `platform`, `version`, `verify`, and `shim` small and dependable
- finish the intended `fs` transaction/workspace surface
- narrow `fetch` to the features that are truly reliable, then complete them one by one
- keep CI strict on the library surface

### phase 2 - define resource model

- design a shared resource identity model: name, source, version query, resolved version, checksum, storage key, install intent
- define a store model for downloaded and extracted artifacts
- define registry/state file conventions without binding callers to one policy

### phase 3 - compose into installation flows

- implement install / upgrade / rollback transactions on top of the primitive crates
- connect version resolution, fetch, verify, extract, stage, commit, and shim activation into one coherent flow
- expose this as reusable crates, not as one monolithic application framework

### phase 4 - source and backend ecosystem

- add source adapters and mirror strategies
- add richer backend patterns for version managers, plugin managers, and config managers
- keep package-format semantics out of pulith core unless a format is genuinely common and reusable

## Out of Scope

- Package format definitions (let sources define)
- Repository hosting
- Authentication servers
- License management
- Dependency resolution

## References

- [README.md](./README.md) - Project overview and getting started
- [docs/AGENT.md](./AGENT.md) - Coding specifications and development guidelines
- [docs/design/verify.md](./design/verify.md) - Content verification primitives design
- [docs/design/fetch.md](./design/fetch.md) - HTTP fetching design
- [docs/design/fs.md](./design/fs.md) - Filesystem primitives design
- [docs/design/archive.md](./design/archive.md) - Archive handling design
- [docs/design/version.md](./design/version.md) - Version parsing design
- [docs/design/platform.md](./design/platform.md) - Platform utilities design
- [docs/design/shim.md](./design/shim.md) - Shim generation design
