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

pub use http::{HttpClient, BoxStream};
pub use fetcher::Fetcher;
#[cfg(feature = "reqwest")]
pub use http::ReqwestClient;