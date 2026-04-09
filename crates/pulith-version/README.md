# pulith-version

Version parsing, comparison, and preference selection.

## Main APIs

- `VersionKind`
- `VersionRequirement`
- `VersionPreference`
- `SelectionPolicy`
- `select_preferred`

## Basic Usage

```rust
use pulith_version::{SelectionPolicy, VersionPreference, VersionRequirement, VersionKind, select_preferred};

let versions = [VersionKind::parse("1.0.0")?, VersionKind::parse("1.1.0")?];
let policy = SelectionPolicy {
    requirement: VersionRequirement::Any,
    preference: VersionPreference::HighestStable,
};
let selected = select_preferred(&versions, &policy);
# Ok::<(), pulith_version::VersionError>(())
```

See `docs/design/version.md`.
