# pulith-fetch

HTTP downloading with streaming verification and atomic placement. Tee-Reader pattern.

## API

```rust
pub trait HttpClient: Send + Sync {
    type Error;
    async fn stream(&self, url: &str) -> Result<BoxStream<Result<Bytes, Self::Error>>, Self::Error>;
    async fn head(&self, url: &str) -> Result<Option<u64>, Self::Error>;
}

pub struct Fetcher<C: HttpClient> {
    client: C,
    workspace_root: PathBuf,
    options: FetchOptions,
}

impl<C: HttpClient> Fetcher<C> {
    pub fn new(client: C, workspace_root: impl Into<PathBuf>) -> Self;
    pub fn with_options(self, FetchOptions) -> Self;
    pub async fn fetch(&self, url: &str, dest: &Path) -> Result<PathBuf, FetchError>;
}

FetchOptions {
    checksum: Option<Vec<u8>>,
    max_retries: u32,
    retry_backoff: Duration,
    timeouts: Timeouts,
    on_progress: Option<Arc<dyn Fn(Progress) + Send + Sync>>,
}

FetchPhase { Connecting, Downloading, Verifying, Committing, Completed }
Progress { phase, bytes_downloaded, total_bytes, retry_count }
}

#[cfg(feature = "reqwest")]
ReqwestClient::new() -> Result<Self, reqwest::Error>;
```

## Architecture

```
Network Stream → 128KB Chunks → [Async File + Hasher (Tee)]
                                     ↓
                              Workspace (pulith-fs)
                                     ↓
                              Atomic Commit
                                     ↓
                              Final Path
```

## Example

```rust
use pulith_fetch::{Fetcher, ReqwestClient, FetchOptions};

let client = ReqwestClient::new()?;
let fetcher = Fetcher::new(client, "/tmp");

let options = FetchOptions::default()
    .checksum(Some(expected_hash.to_vec()))
    .on_progress(Arc::new(|p| println!("{:?}", p)));

let path = fetcher.fetch(url, dest).await?;
```

## Dependencies

```
thiserror, tokio, async-trait, bytes, futures-util, hex

pulith-fs = { path = "../pulith-fs" }
pulith-verify = { path = "../pulith-verify" }

[features]
default = ["reqwest"]
reqwest = ["dep:reqwest"]
```

## Guarantees

| Guarantee | Implementation |
|-----------|----------------|
| Single-Pass | Tee-Reader hashes during stream |
| Atomicity | Workspace cleans up on error |
| Memory Bound | 128KB chunks |

## Relationship

```
pulith-fetch
    ├── HttpClient trait
    ├── Fetcher
    └── FetchOptions, Progress

Uses: pulith-fs (Workspace), pulith-verify (Sha256Hasher)
```
