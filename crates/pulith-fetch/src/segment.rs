//! File segmentation and validation.

pub mod segment;
pub mod validation;

pub use segment::{calculate_segments, Segment};
pub use validation::is_redirect;
