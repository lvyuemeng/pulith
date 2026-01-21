//! Pipeline - transaction log with execution semantics.
//!
//! The pipeline orchestrates the install process through stages, ensuring
//! atomicity and rollback on failure.

use crate::data::InstallContext;
use crate::effects::{CommandRunner, FileSystem};
use crate::error::{ActivateError, HookError, PipelineError, StageError};
use crate::hooks::InstallHook;
use crate::transform::Transform;
use std::path::Path;
use uuid::Uuid;

/// The install pipeline orchestrator.
/// Composable builder pattern for constructing install operations.
pub struct Pipeline {
    staging_root: std::path::PathBuf,
    active_root:  std::path::PathBuf,
    transforms:   Vec<Box<dyn Transform<Error = Box<dyn std::error::Error + Send + Sync>>>>,
    hooks:        Vec<Box<dyn InstallHook>>,
}

impl Pipeline {
    /// Create a new pipeline with staging and active roots.
    pub fn new(staging_root: impl Into<std::path::PathBuf>, active_root: impl Into<std::path::PathBuf>) -> Self {
        Self {
            staging_root: staging_root.into(),
            active_root:  active_root.into(),
            transforms:   vec![],
            hooks:        vec![],
        }
    }

    /// Add a transform operation to the pipeline.
    pub fn transform<T: Transform + 'static>(mut self, transform: T) -> Self {
        self.transforms.push(Box::new(transform));
        self
    }

    /// Add a hook to the pipeline.
    pub fn hook<H: InstallHook + 'static>(mut self, hook: H) -> Self {
        self.hooks.push(Box::new(hook));
        self
    }

    /// Execute the pipeline with the given artifact source.
    ///
    /// # Arguments
    /// * `source` - Path to the verified artifact to install
    /// * `fs` - Filesystem implementation for I/O operations
    /// * `runner` - Command runner for process execution
    ///
    /// # Returns
    /// Success if the install completes atomically, error with rollback otherwise.
    pub fn run<F: FileSystem, R: CommandRunner>(
        &self,
        source: &std::path::Path,
        fs: &F,
        runner: &R,
    ) -> Result<(), PipelineError> {
        let mut ctx = InstallContext::new(self.staging_root.clone(), self.active_root.clone());

        // Track success state for rollback decisions
        let mut staged = false;
        let mut transformed = false;
        let mut activated = false;

        // Execute pipeline with automatic rollback on failure
        let result = self.run_with_rollback(source, fs, runner, &mut ctx, &mut staged, &mut transformed, &mut activated);

        // If pipeline failed, perform rollback
        if result.is_err() {
            // Call pre-rollback hooks
            for hook in &self.hooks {
                let _ = hook.pre_rollback(&mut ctx); // Ignore errors during rollback
            }

            // Perform actual rollback
            self.rollback(fs, &ctx, staged, transformed, activated);

            // Note: We don't call post-rollback hooks as rollback is best-effort
        }

        result
    }

    /// Internal pipeline execution with rollback state tracking.
    fn run_with_rollback<F: FileSystem, R: CommandRunner>(
        &self,
        source: &std::path::Path,
        fs: &F,
        runner: &R,
        ctx: &mut InstallContext,
        staged: &mut bool,
        transformed: &mut bool,
        activated: &mut bool,
    ) -> Result<(), PipelineError> {
        // Pre-stage hooks
        for hook in &self.hooks {
            hook.pre_stage(ctx).map_err(|e| HookError::HookFailed {
                name: hook.name().to_string(),
                source: Box::new(e),
            })?;
        }

        // Stage: copy artifact to temporary staging location
        let staging_path = self.staging_root.join(Uuid::new_v4().to_string());
        stage(source, &staging_path, fs)?;
        ctx.staged_files.push(staging_path.clone());
        *staged = true;

        // Post-stage hooks
        for hook in &self.hooks {
            hook.post_stage(ctx).map_err(|e| HookError::HookFailed {
                name: hook.name().to_string(),
                source: Box::new(e),
            })?;
        }

        // Transform: apply modifications to staged files
        for transform in &self.transforms {
            transform.apply(&staging_path, fs, runner).map_err(|e| {
                PipelineError::Transform(Box::new(e))
            })?;
        }
        *transformed = true;

        // Pre-activate hooks
        for hook in &self.hooks {
            hook.pre_activate(ctx).map_err(|e| HookError::HookFailed {
                name: hook.name().to_string(),
                source: Box::new(e),
            })?;
        }

        // Determine target path for activation
        let filename = source
            .file_name()
            .ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "source has no filename",
                )
            })
            .map_err(PipelineError::Rollback)?;
        let target = self.active_root.join(filename);

        // Activate: atomically make staged files available
        activate(&staging_path, &target, fs)?;
        ctx.created_links.push(target.clone());
        *activated = true;

        // Post-activate hooks
        for hook in &self.hooks {
            hook.post_activate(ctx).map_err(|e| HookError::HookFailed {
                name: hook.name().to_string(),
                source: Box::new(e),
            })?;
        }

        // Commit: finalize the transaction
        commit(&staging_path, &target, fs)?;

        // Post-commit hooks
        for hook in &self.hooks {
            hook.post_commit(ctx).map_err(|e| HookError::HookFailed {
                name: hook.name().to_string(),
                source: Box::new(e),
            })?;
        }

        Ok(())
    }

    /// Rollback failed installation to clean state.
    fn rollback<F: FileSystem>(
        &self,
        fs: &F,
        ctx: &InstallContext,
        staged: bool,
        _transformed: bool,
        activated: bool,
    ) {
        // Rollback in reverse order of operations

        // 1. Remove activated links (if activation completed)
        if activated {
            for link in &ctx.created_links {
                let _ = fs.remove_dir_all(link); // Best effort - ignore errors
            }
        }

        // 2. Clean up staged files (if staging completed)
        if staged {
            for staging_dir in &ctx.staged_files {
                let _ = fs.remove_dir_all(staging_dir); // Best effort - ignore errors
            }
        }

        // Note: Transform rollback is complex and depends on what operations were performed.
        // For now, we rely on the staged directory being cleaned up.
        // A more sophisticated implementation could track transform operations for undo.
    }
}

/// Stage phase: copy artifact to staging area.
fn stage<F: FileSystem>(
    source: &Path,
    staging: &Path,
    fs: &F,
) -> Result<(), StageError> {
    if !fs.exists(source) {
        return Err(StageError::SourceNotFound(source.to_path_buf()));
    }

    if fs.is_dir(source) {
        copy_dir_all(source, staging, fs)?;
    } else {
        fs.copy(source, staging).map_err(StageError::Io)?;
    }

    Ok(())
}

/// Copy directory recursively.
fn copy_dir_all<F: FileSystem>(
    src: &Path,
    dst: &Path,
    fs: &F,
) -> Result<(), std::io::Error> {
    if !fs.exists(dst) {
        fs.create_dir_all(dst)?;
    }

    // For simplicity, we'll assume we have access to std::fs for directory iteration
    // In a real implementation, this would need to be abstracted too
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path, fs)?;
        } else {
            fs.copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

/// Activate phase: atomically make staged files available.
fn activate<F: FileSystem>(
    staging: &Path,
    target: &Path,
    fs: &F,
) -> Result<(), ActivateError> {
    if fs.exists(target) {
        return Err(ActivateError::LinkExists(target.to_path_buf()));
    }
    fs.link(staging, target).map_err(ActivateError::Io)
}

/// Commit phase: finalize the transaction.
fn commit<F: FileSystem>(
    _staging: &Path,
    _target: &Path,
    _fs: &F,
) -> Result<(), ActivateError> {
    // For now, commit is a no-op since activation already made the files available
    // In a more sophisticated implementation, this might involve additional
    // durable state updates or cleanup
    Ok(())
}
