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

## Crate Ecosystem

### Crate Structure

```
pulith/
├── crates/
│   ├── pulith-platform/   # ✅ Implemented - OS, arch, shell, path helpers
│   ├── pulith-version/    # ✅ Implemented - Version parsing, comparison, display
│   ├── pulith-fetch/      # HTTP downloads, progress, checksum
│   ├── pulith-install/    # Atomic ops, staging, activation
│   ├── pulith-registry/   # Typed state, atomic saves
│   ├── pulith-ui/         # Progress, tables, spinners
│   └── pulith-source/     # Source adapters (deferred design)
```

### Implementation Status

| Crate | Status | Description |
|-------|--------|-------------|
| `pulith-platform` | ✅ Done | OS/distro, arch, shell, path helpers |
| `pulith-version` | ✅ Done | SemVer, CalVer, partial version parsing |
| `pulith-fetch` | 🔲 Pending | HTTP downloads |
| `pulith-registry` | 🔲 Pending | State persistence |
| `pulith-ui` | 🔲 Pending | Progress and tables |
| `pulith-install` | 🔲 Pending | Atomic file operations |
| `pulith-source` | ⏸ Deferred | Source adapters |

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
- **Custom**: User-defined version schemes

#### pulith-fetch
HTTP downloading with verification:
- Progress tracking with callbacks
- SHA256 checksum verification
- Retry logic with backoff
- Redirect handling
- Proxy support

#### pulith-install
Atomic file system operations:
- Staged installs (download → verify → activate)
- Atomic file replacement (same-FS + copy fallback)
- Symlink and shim management
- PATH activation helpers
- Rollback on failure

#### pulith-registry
Typed state management with persistence:
- Auto-saving on drop
- Hash verification (detect external modification)
- Binary serialization with postcard
- Migration support

#### pulith-ui
User interface primitives:
- Progress bars (indicatif-based)
- Tables (tabled-based)
- Spinners and status indicators
- Composable builders

#### pulith-platform
Cross-platform helpers:
- OS and distribution detection
- Architecture detection
- Shell detection and invocation
- PATH manipulation
- Home directory resolution

#### pulith-source
Source adapters for fetching resources (design deferred):
- npm registry
- GitHub releases
- HTTP direct
- S3 and custom sources

## Crate Relationships

```
                    ┌─────────────────┐
                    │   User Tool     │
                    └────────┬────────┘
                             │
              ┌──────────────┼──────────────┐
              │              │              │
              ▼              ▼              ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │ pulith-  │  │ pulith-  │  │ pulith-  │
        │ version ✅│  │platform ✅│  │   ui     │
        └────┬─────┘  └──────────┘  └──────────┘
             │              │
             │      ┌───────┴───────┐
             │      │               │
             ▼      ▼               ▼
        ┌──────────┐         ┌──────────┐
        │ pulith-  │◄────────┤ pulith-  │
        │ fetch    │         │ registry │
        └────┬─────┘         └──────────┘
             │
             ▼
        ┌──────────┐
        │ pulith-  │
        │ install  │
        └──────────┘
```

## Design Directions (Deferred)

These areas require further design when needed:

### Source Adapters
- Trait definition for sources
- Locator syntax (github:owner/repo@v1.0.0)
- Authentication and caching
- Dynamic vs static registration

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
- [docs/migration.md](./migration.md) - Historical context and pivot
- [docs/AGENT.md](./AGENT.md) - Coding specifications
