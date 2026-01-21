//! Cross-platform system utilities for resource management.
//!
//! Provides OS, architecture, shell, and path helpers.

pub use self::platform::{Arch, Distro, OS, Shell};

mod platform;
