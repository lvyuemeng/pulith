//! Pure data types for install operations.

use std::path::PathBuf;

/// Context shared during pipeline execution.
/// Immutable data, mutated only at effect boundaries.
#[derive(Debug, Clone)]
pub struct InstallContext {
    pub staging_root:  PathBuf,
    pub active_root:   PathBuf,
    pub ops:           Vec<TransformOp>,
    pub staged_files:  Vec<PathBuf>,
    pub created_links: Vec<PathBuf>,
}

impl InstallContext {
    pub fn new(staging_root: PathBuf, active_root: PathBuf) -> Self {
        Self {
            staging_root,
            active_root,
            ops: vec![],
            staged_files: vec![],
            created_links: vec![],
        }
    }
}

/// Transform operations - semantically structured, mechanically generic.
/// These define what changes to make to staged artifacts.
#[derive(Debug, Clone)]
pub enum TransformOp {
    /// Relocate files/directories within the staged area.
    Relocate {
        from: PathBuf,
        to:   PathBuf,
    },
    /// Rewrite shebang lines in scripts.
    RewriteShebang {
        files:       Vec<PathBuf>,
        interpreter: String,
    },
    /// Patch RPATH in binaries (placeholder for future implementation).
    PatchRpath {
        binaries:   Vec<PathBuf>,
        old_prefix: String,
        new_prefix: String,
    },
    /// Run external process during transform.
    RunProcess {
        cmd:  String,
        args: Vec<String>,
        env:  Vec<(String, String)>,
    },
    /// Edit system registry (platform-specific, placeholder for Windows).
    EditRegistry {
        key:   String,
        value: String,
    },
    /// Set file permissions.
    SetPermissions {
        files: Vec<PathBuf>,
        mode:  u32,
    },
}

impl TransformOp {
    /// Create a relocate operation.
    pub fn relocate(from: impl Into<PathBuf>, to: impl Into<PathBuf>) -> Self {
        Self::Relocate {
            from: from.into(),
            to:   to.into(),
        }
    }

    /// Create a shebang rewrite operation.
    pub fn rewrite_shebang(files: Vec<PathBuf>, interpreter: impl Into<String>) -> Self {
        Self::RewriteShebang {
            files,
            interpreter: interpreter.into(),
        }
    }

    /// Create an RPATH patch operation.
    pub fn patch_rpath(
        binaries: Vec<PathBuf>,
        old_prefix: impl Into<String>,
        new_prefix: impl Into<String>,
    ) -> Self {
        Self::PatchRpath {
            binaries,
            old_prefix: old_prefix.into(),
            new_prefix: new_prefix.into(),
        }
    }

    /// Create a process execution operation.
    pub fn run_process(
        cmd: impl Into<String>,
        args: Vec<String>,
        env: Vec<(String, String)>,
    ) -> Self {
        Self::RunProcess {
            cmd: cmd.into(),
            args,
            env,
        }
    }

    /// Create a registry edit operation.
    pub fn edit_registry(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self::EditRegistry {
            key:   key.into(),
            value: value.into(),
        }
    }

    /// Create a permissions setting operation.
    pub fn set_permissions(files: Vec<PathBuf>, mode: u32) -> Self {
        Self::SetPermissions { files, mode }
    }
}
