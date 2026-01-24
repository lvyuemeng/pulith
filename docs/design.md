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
