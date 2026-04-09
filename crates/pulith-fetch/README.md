# pulith-fetch

Transfer execution primitives for Pulith.

## Role

`pulith-fetch` turns fetch plans into bytes and receipts.

It should own:

- transfer execution
- local-vs-remote fetch execution
- multi-source execution strategies
- fetch receipts

It should not own:

- source planning semantics
- storage policy
- install policy

## Main APIs

- `Fetcher`
- `MultiSourceFetcher`
- `FetchReceipt`
- `FetchSource`
- `FetchOptions`
- `ReqwestClient`

## Basic Usage

```rust
use pulith_fetch::{Fetcher, MultiSourceFetcher, ReqwestClient};

let fetcher = Fetcher::new(ReqwestClient::new()?, "workspace")?;
let _multi = MultiSourceFetcher::new(std::sync::Arc::new(fetcher));
# Ok::<(), pulith_fetch::Error>(())
```

## How To Use It

Pair `pulith-fetch` with:

- `pulith-source` for planned candidates
- `pulith-store` for durable registration
- `pulith-install` for workflow handoff

Fetch should stay explicit: the caller chooses strategy, destination, and later workflow steps.

See `docs/design/fetch.md`.
