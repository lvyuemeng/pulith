# pulith-archive

Archive extraction with path sanitization and transactional staging.

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

See `docs/design/archive.md`.
