#![allow(clippy::module_inception)]

//! File segmentation and validation.

pub mod segment;
pub mod validation;

pub use segment::{Segment, calculate_segments};
pub use validation::is_redirect;
