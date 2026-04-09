# pulith-install

Typed installation and activation workflow primitives.

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

See `docs/design/install.md`.
