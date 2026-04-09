# pulith-archive

Archive extraction with sanitization and transactional staging.

## Role

`pulith-archive` owns archive materialization.

It provides:

- archive format handling
- extraction into a target/workspace
- path sanitization
- extraction reporting

It should not own:

- store policy
- install policy
- fetch policy

## Main APIs

- `extract_from_reader`
- `ArchiveFormat`
- `ArchiveReport`
- `ExtractOptions`
- `WorkspaceExtraction`

## Basic Usage

```rust
use pulith_archive::{extract_from_reader, ExtractOptions};

let zip_bytes = std::io::Cursor::new(Vec::<u8>::new());
let _ = extract_from_reader(zip_bytes, "out", &ExtractOptions::default());
```

## How To Use It

Use this crate when a fetched or otherwise available archive needs to become a sanitized file tree that another crate can store or install.

See `docs/design/archive.md`.
