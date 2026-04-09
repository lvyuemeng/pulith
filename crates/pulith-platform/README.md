# pulith-platform

Small host/platform helpers for Pulith.

## Role

`pulith-platform` exists to answer host questions and provide lightweight platform-oriented helpers.

It should stay narrow.

Good scope:

- operating system and distribution detection
- architecture and target-triple parsing
- shell metadata
- user directory conventions
- lightweight command/environment helpers

Bad scope:

- install orchestration
- package-manager policy
- service management
- broad system administration logic

## Main APIs

- `arch::{Arch, TargetTriple}`
- `os::{OS, Distro, detect_distro}`
- `shell::Shell`
- `dir::{user_home, user_config, user_data, user_cache, user_temp}`
- `env::{PathModifier, path_env, is_in_path}`
- `command::Command`

## Basic Usage

```rust
use pulith_platform::arch::TargetTriple;
use pulith_platform::os::OS;
use pulith_platform::shell::Shell;

let host = TargetTriple::host();
let os = OS::current();
let shell = Shell::current();

assert_eq!(host.os, os);
assert!(shell.is_some() || shell.is_none());
```

## How To Use It

Use this crate at the boundary where a resource manager needs to adapt to the host environment.

Examples:

- derive an activation shell
- choose a platform-specific artifact variant
- compute a config/cache directory
- construct a small process invocation with normalized shell flags

See `docs/design/platform.md`.
