//! I/O operations and effectful computations for HTTP fetching.
//!
//! This module contains all the functions that perform I/O operations,
//! network requests, file operations, and other effectful computations.
//! These functions follow the F3-F4 principles from AGENT.md: Pure Core,
//! Impure Edge and Explicit Effects.

mod http;
mod fetcher;
mod multi_source;
mod resumable;
mod segmented;
mod batch;
mod cache;
mod throttled;
mod conditional;
mod protocol;

pub use http::{HttpClient, BoxStream};
pub use fetcher::Fetcher;
pub use throttled::{ThrottledStream, AsyncThrottledStream};
pub use conditional::{ConditionalFetcher, RemoteMetadata, ConditionalOptions};
pub use protocol::{
    Protocol, Direction, TransferMetadata, TransferOptions, TransferStream, ProtocolClient,
    ProtocolRegistry, MockHttpClient, MockTransferStream,
};
#[cfg(feature = "reqwest")]
pub use http::ReqwestClient;