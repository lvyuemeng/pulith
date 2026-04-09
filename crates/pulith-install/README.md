# pulith-install

Typed installation and activation workflow primitives.

## Role

`pulith-install` coordinates placement and activation.

It composes lower crates but should not absorb all operational behavior.

## Main APIs

- `InstallReady`
- `InstallSpec`
- `InstallInput`
- `PlannedInstall`
- `SymlinkActivator`
- `CopyFileActivator`

## Basic Usage

```rust
use pulith_install::{InstallReady, InstallSpec, PlannedInstall};

# let ready: InstallReady = todo!();
# let spec: InstallSpec = todo!();
let _receipt = PlannedInstall::new(ready, spec).stage()?.commit()?.finish();
# Ok::<(), pulith_install::InstallError>(())
```

## How To Use It

Use this crate when bytes or extracted trees have already been materialized and you want to:

- stage an install
- commit it atomically
- activate it via link/copy/shim-oriented activators
- support replace/upgrade/rollback behavior

See `docs/design/install.md`.
