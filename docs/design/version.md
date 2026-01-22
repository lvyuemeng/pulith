# pulith-version

Version parsing, comparison, display. SemVer, CalVer, partial versions. Pure core, no I/O.

## API

```rust
// Unified version type
VersionKind { SemVer(semver::Version), CalVer(CalVer), Partial(Partial) }

VersionKind::parse("1.2.3") -> Result<VersionKind, VersionError>;

// SemVer (uses semver::Version internally)
VersionKind::SemVer(semver::Version::parse("1.2.3")?)

// CalVer (YYYY, YYYY.MM, YYYY.MM.DD)
CalVer::parse("2024.01")?
CalVer::from_ymd(2024, 1, 15)?

// Partial (major, major.minor, etc.)
Partial::parse("18")?
Partial.matches(&version)  // "18" matches 18.0.0, 18.5.2, etc.
```

## Formats

```
SemVer:  1.2.3, 1.2.3-alpha+build
CalVer:  2024, 2024.01, 2024.01.15
Partial: 18, 3.11, 3.11.0, 18lts
```

## Example

```rust
use pulith_version::VersionKind;

let v1: VersionKind = "1.2.3".parse().unwrap();
let v2: VersionKind = "2.0.0".parse().unwrap();
assert!(v1 < v2);
```

## Dependencies

```
semver, thiserror
```
