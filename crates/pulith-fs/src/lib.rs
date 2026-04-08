pub mod align;
mod error;
pub mod permissions;
pub mod primitives;
pub mod resource;
pub mod workflow;

pub use error::{Error, Result};
pub use permissions::PermissionMode;
pub use primitives::copy_dir::copy_dir_all;
pub use primitives::hardlink::{FallBack, Options as HardlinkOrCopyOptions, hardlink_or_copy};
pub use primitives::replace_dir::{Options as ReplaceDirOptions, replace_dir};
pub use primitives::rw::{Options as AtomicWriteOptions, atomic_read, atomic_write};
pub use primitives::symlink::atomic_symlink;
pub use workflow::{Transaction, Workspace, WorkspaceReport};
