//! Download strategies and fetch implementations.

pub mod fetcher;
pub mod segmented;
pub mod multi_source;
pub mod batch;
pub mod resumable;
pub mod conditional;

pub use fetcher::Fetcher;
pub use segmented::{SegmentedFetcher, SegmentedOptions};
pub use multi_source::MultiSourceFetcher;
pub use batch::{BatchFetcher, BatchOptions, BatchDownloadJob};
pub use resumable::{ResumableFetcher, DownloadCheckpoint};
pub use conditional::{ConditionalFetcher, RemoteMetadata, ConditionalOptions};
