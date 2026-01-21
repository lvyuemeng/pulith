# Pulith Design Document

## Vision

**Resource Management Primitives for Rust**

A crate ecosystem providing everything a Rust tool needs to fetch, verify, store, and track external resources - packages, config files, tools, plugins, or any versioned artifacts.

> "Everything a Rust tool needs to manage versioned external resources - built with best practices."

## Why This Exists

80% of tools that manage external resources reinvent the same primitives:
- Version parsing and comparison
- HTTP downloads with progress and verification
- Atomic file operations and staging
- State tracking with rollback
- Cross-platform correctness

This ecosystem provides battle-tested building blocks so developers can focus on their unique value proposition.

## Target Use Cases

- **Version Managers**: rustup, nvm, pyenv, goenv
- **Config Managers**: dotfiles, config sync, .env managers
- **Plugin Managers**: vim-plug, neovim plugins, IDE extensions
- **Registry Servers**: npm registry mirrors, PyPI caches, internal registries
- **Tool Managers**: SDK managers, CLI tool installers
- **Artifact Repositories**: container image caches, binary caches

## Design Principles

1. **Atomicity**: All state-changing operations are atomic with rollback
2. **Composability**: Crates can be used independently or together
3. **Cross-Platform**: Consistent behavior across Windows, macOS, Linux
4. **Extensibility**: Higher-layer patterns (sources, backends) designed later
5. **Best Practices**: Security, verification, and correctness baked in
6. **Mechanism-only**: Provide primitives to fetch, store, stage, and track external resources.

## Crate Ecosystem

### Crate Descriptions

#### pulith-platform ✅
Cross-platform helpers:
- OS and distribution detection (Windows, macOS, Linux distros)
- Architecture detection (x86, x64, ARM variants)
- Shell detection and invocation
- PATH manipulation
- Home and temp directory resolution

#### pulith-version ✅
Version parsing and comparison for multiple formats:
- **SemVer**: Semantic versioning (1.2.3, 1.2.3-alpha+build)
- **CalVer**: Calendar versioning (2024.01, 2024.01.15)
- **Partial**: Partial versions (18, 3.11, lts)

#### pulith-shim ✅
Shim generation for version switching:
- Unix shell stubs (bash, zsh, fish)
- Windows batch and PowerShell scripts
- Platform-specific executable wrappers

#### pulith-fetch
HTTP downloading with verification:
- Progress tracking with callbacks
- SHA256 checksum verification
- Retry logic with backoff
- Redirect handling
- Proxy support

#### pulith-fs

Role: Cross-platform atomic filesystem primitives. Mechanism Only: It does not know what a "tool" is. It only knows how to move bytes safely.

Example:

- atomic_write(path, content): Writes to a temp file, fsyncs, then renames.

- atomic_symlink(target, link_path): Creates a new link, then renames over the old one.

- replace_dir(src, dest): The holy grail of installers. Atomically swaps a directory. On Windows, this handles the complex retry/rename dance required when files are locked.

- hardlink_or_copy(src, dest): Optimization primitive.

Managed workspace for preparing artifacts.

**Workspace** (formerly Stage)

Role: A transactional workspace for preparing resources. Philosophy: Installation is a transaction. It either happens completely or not at all. Mechanism only: no policy, no format enforcement.

Example:

```rust
let workspace = Workspace::new(temp_dir)?;

// 2. Do work (User Policy defines what happens here)
workspace.write("bin/tool", bytes)?;
workspace.create_dir("lib")?;
workspace.create_dir_all("nested/deep")?;

// 3. Commit (The Mechanism)
// This atomically moves the staged directory to the final destination.
// If this fails, the workspace is dropped and the temp dir is cleaned up.
workspace.commit(final_destination_path)?;
```

Why this fits: It doesn't care if you are installing a Node version, a VIM plugin, or a config file. It guarantees that final_destination_path never exists in a half-written state.

**Transaction** (formerly State)

Role: Concurrent-safe read-modify-write on a persistent file, without enforcing a schema. Concrete-Independent: It deals in opaque Bytes only.

Example:

```rust
let tx = Transaction::open("registry.json")?;

// Blocks other processes, reads current content, allows modification,
// and atomically writes back.
tx.execute(|bytes| {
    let data: MyCustomSchema = MyCustomSchema::from(bytes); // User defines Schema
    data.last_update = now();
    Ok(data.to_bytes()) // User handles serialization
})?;
```

Mechanism: Handles file locking (flock/LockFile), read-modify-write cycles, and atomic replacement. It prevents two instances of your tool from corrupting the data.
```

Mechanism: Handles file locking (flock/LockFile), read-modify-write cycles, and atomic replacement. It prevents two instances of your tool from corrupting the registry.

#### pulith-ui
User interface primitives:
- Progress bars (indicatif-based)
- Tables (tabled-based)
- Spinners and status indicators
- Composable builders

## Design Directions (Deferred)

These areas require further design when needed:

### Backend Abstractions
- Trait for package managers
- Multi-manager orchestration
- Flag resolution patterns

### Migration and Upgrades
- Schema migration for registries
- In-place upgrade patterns
- Backup and restore

## Out of Scope

- Package format definitions (let sources define)
- Repository hosting
- Authentication servers
- License management
- Dependency resolution

## References

- [README.md](./README.md) - Project overview and getting started
- [docs/AGENT.md](./AGENT.md) - Coding specifications
- [docs/design/*.md] - Design of subcrates
