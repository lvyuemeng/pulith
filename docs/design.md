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
└── crates/
    ├── pulith-platform/   # OS, arch, shell, path helpers
    ├── pulith-version/    # Version parsing, comparison
    ├── pulith-fetch/      # HTTP downloads, progress, checksum
    ├── pulith-shim/       # Shim generation
    ├── pulith-install/    # Atomic ops, staging, activation
    ├── pulith-registry/   # Typed state, atomic saves
    └── pulith-ui/         # Progress, tables, spinners
```

### Implementation Status

| Crate | Status | Notes |
|-------|--------|-------|
| `pulith-platform` | ✅ Done | OS/distro, arch, shell, path helpers |
| `pulith-version` | ✅ Done | SemVer, CalVer, partial version parsing |
| `pulith-shim` | ✅ Done | Shim generation with three-layer pattern |
| `pulith-fetch` | 🔲 Part | HTTP downloads with verification |
| `pulith-install` | 🔲 Pending | Atomic file operations |
| `pulith-registry` | 🔲 Pending | State persistence |
| `pulith-ui` | 🔲 Pending | Progress and tables |

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
- Binary serialization
- Migration support

#### pulith-ui
User interface primitives:
- Progress bars (indicatif-based)
- Tables (tabled-based)
- Spinners and status indicators
- Composable builders

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
        │ version  │  │platform  │  │   shim   │
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
