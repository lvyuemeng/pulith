//! Shim mechanism for command routing.
//!
//! # Architecture
//!
//! Shim is a mechanism, not policy. It only maps a command name
//! to an absolute binary path and executes it.
//!
//! The [`TargetResolver`] trait defines the contract for resolution policy.
//! Users implement this trait according to their needs.
//!
//! # Example
//!
//! ```
//! use std::path::PathBuf;
//! use pulith_shim::{TargetResolver, PairResolver};
//!
//! struct LocalResolver;
//! struct GlobalResolver;
//!
//! impl TargetResolver for LocalResolver {
//!     fn resolve(&self, command: &str) -> Option<PathBuf> {
//!         // Read from local project config
//!         Some(PathBuf::from(format!("/local/bin/{}", command)))
//!     }
//! }
//!
//! impl TargetResolver for GlobalResolver {
//!     fn resolve(&self, command: &str) -> Option<PathBuf> {
//!         // Read from global config
//!         Some(PathBuf::from(format!("/global/bin/{}", command)))
//!     }
//! }
//!
//! let resolver = PairResolver::new(LocalResolver, GlobalResolver);
//! assert!(resolver.resolve("npm").is_some());
//! ```

pub use error::{Error, Result};
pub use resolver::{PairResolver, TargetResolver, TripleResolver};

mod error;
mod resolver;
