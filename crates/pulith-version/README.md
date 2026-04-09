# pulith-version

Version parsing, matching, and preference selection.

## Role

`pulith-version` is a pure primitive crate.

It owns:

- version parsing
- requirement matching
- generic preference selection

It does not own:

- resource-specific alias meaning
- repository policy
- planner-specific source policy

## Main APIs

- `VersionKind`
- `VersionRequirement`
- `VersionPreference`
- `SelectionPolicy`
- `select_preferred`

## Basic Usage

```rust
use pulith_version::{
    SelectionPolicy, VersionKind, VersionPreference, VersionRequirement, select_preferred,
};

let versions = [VersionKind::parse("1.0.0")?, VersionKind::parse("1.1.0")?];
let policy = SelectionPolicy {
    requirement: VersionRequirement::Any,
    preference: VersionPreference::HighestStable,
};
let selected = select_preferred(&versions, &policy).unwrap();
assert_eq!(selected.to_string(), "1.1.0");
# Ok::<(), pulith_version::VersionError>(())
```

## How To Use It

Use this crate when a caller already has candidate versions and needs a consistent, reusable way to:

- parse version strings
- match requirements
- prefer stable/latest/lowest/pinned variants

Let higher-level crates decide what aliases like `stable` or `lts` mean in resource-specific terms.

See `docs/design/version.md`.
