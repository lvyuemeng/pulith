//! Download strategies and fetch implementations.

pub mod batch;
pub mod conditional;
pub mod fetcher;
pub mod multi_source;
pub mod resumable;
pub mod segmented;

pub use batch::{BatchDownloadJob, BatchFetcher, BatchOptions};
pub use conditional::{ConditionalFetcher, ConditionalOptions, RemoteMetadata};
pub use fetcher::Fetcher;
pub use multi_source::MultiSourceFetcher;
pub use resumable::{DownloadCheckpoint, ResumableFetcher};
pub use segmented::{SegmentedFetcher, SegmentedOptions};
