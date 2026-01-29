//! Pure transformations and business logic for HTTP fetching.
//!
//! This module contains all the pure functions that transform data without
//! performing I/O operations. These functions follow the F1-F2 principles
//! from AGENT.md: Functions First and Immutability by Default.

mod retry;
mod bandwidth;
mod segment;
mod validation;

pub use retry::{retry_delay};
pub use bandwidth::TokenBucket;
pub use segment::{calculate_segments, Segment};
pub use validation::{is_redirect};