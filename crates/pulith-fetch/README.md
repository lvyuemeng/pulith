# pulith-fetch

Transfer execution primitives for Pulith.

## Main APIs

- `Fetcher`
- `MultiSourceFetcher`
- `FetchReceipt`
- `FetchOptions`
- `ReqwestClient`

## Basic Usage

```rust
use pulith_fetch::{Fetcher, MultiSourceFetcher, ReqwestClient};

let fetcher = Fetcher::new(ReqwestClient::new()?, "workspace")?;
let _multi = MultiSourceFetcher::new(std::sync::Arc::new(fetcher));
# Ok::<(), pulith_fetch::Error>(())
```

See `docs/design/fetch.md`.
