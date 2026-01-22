//! Transform trait and implementations for user-defined transformations.
//!
//! This module provides the core `Transform` trait that users implement for their
//! specific installation needs, along with built-in transform implementations.

use std::path::{Path, PathBuf};

use crate::effects::{CommandRunner, FileSystem};

/// Core trait for user-defined transformations.
///
/// Users implement this trait to define custom installation steps that are
/// type-safe and composable. Each transform operates on a staged directory
/// and can perform filesystem operations or run external processes.
pub trait Transform {
    /// Error type for this transform.
    type Error: std::error::Error + Send + Sync + 'static;

    /// Apply this transform to the staged directory.
    ///
    /// # Arguments
    /// * `staging_root` - Path to the staged directory containing the artifact
    /// * `fs` - Filesystem implementation for I/O operations
    /// * `runner` - Command runner for process execution
    ///
    /// # Returns
    /// Success if the transform completes, error otherwise.
    fn apply<F: FileSystem, R: CommandRunner>(
        &self,
        staging_root: &Path,
        fs: &F,
        runner: &R,
    ) -> Result<(), Self::Error>;
}

/// Helper trait for boxing transforms for dynamic dispatch.
pub trait BoxableTransform {
    /// Apply this transform to the staged directory.
    fn apply_boxed(
        &self,
        staging_root: &Path,
        fs: &dyn FileSystem,
        runner: &dyn CommandRunner,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

impl<T: Transform> BoxableTransform for T {
    fn apply_boxed(
        &self,
        staging_root: &Path,
        fs: &dyn FileSystem,
        runner: &dyn CommandRunner,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // This is a workaround for the dyn compatibility issue
        // We need to downcast the trait objects back to concrete types
        // This is not ideal but necessary for now
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            "BoxableTransform not implemented for this transform type"
        )))
    }
}

/// Built-in transform implementations for common operations.
pub mod builtins {
    use super::*;
    use std::io;

    /// Relocate files or directories within the staged area.
    #[derive(Debug, Clone)]
    pub struct RelocateTransform {
        pub from: PathBuf,
        pub to:   PathBuf,
    }

    impl RelocateTransform {
        pub fn new(from: impl Into<PathBuf>, to: impl Into<PathBuf>) -> Self {
            Self {
                from: from.into(),
                to:   to.into(),
            }
        }
    }

    impl Transform for RelocateTransform {
        type Error = RelocateError;

        fn apply<F: FileSystem, R: CommandRunner>(
            &self,
            staging_root: &Path,
            fs: &F,
            _runner: &R,
        ) -> Result<(), Self::Error> {
            let src_path = staging_root.join(&self.from);
            let dst_path = staging_root.join(&self.to);

            if !fs.exists(&src_path) {
                return Ok(()); // Nothing to relocate
            }

            if fs.is_dir(&src_path) {
                copy_dir_all(&src_path, &dst_path, fs)
                    .map_err(|e| RelocateError::Copy {
                        from: src_path.clone(),
                        to:   dst_path.clone(),
                        source: e,
                    })?;
                // Remove source directory after successful copy
                fs.remove_dir_all(&src_path).map_err(|e| RelocateError::Remove {
                    path: src_path.clone(),
                    source: e,
                })?;
            } else {
                fs.copy(&src_path, &dst_path).map_err(|e| RelocateError::Copy {
                    from: src_path.clone(),
                    to:   dst_path.clone(),
                    source: e,
                })?;
            }

            Ok(())
        }
    }

    /// Rewrite shebang lines in scripts.
    #[derive(Debug, Clone)]
    pub struct RewriteShebangTransform {
        pub files:       Vec<PathBuf>,
        pub interpreter: String,
    }

    impl RewriteShebangTransform {
        pub fn new(files: Vec<PathBuf>, interpreter: impl Into<String>) -> Self {
            Self {
                files,
                interpreter: interpreter.into(),
            }
        }
    }

    impl Transform for RewriteShebangTransform {
        type Error = ShebangError;

        fn apply<F: FileSystem, R: CommandRunner>(
            &self,
            staging_root: &Path,
            fs: &F,
            _runner: &R,
        ) -> Result<(), Self::Error> {
            for file in &self.files {
                let file_path = staging_root.join(file);
                rewrite_shebang(&file_path, &self.interpreter)
                    .map_err(|e| ShebangError {
                        file: file_path.clone(),
                        source: e,
                    })?;
            }
            Ok(())
        }
    }

    /// Patch RPATH in binaries (placeholder for future implementation).
    #[derive(Debug, Clone)]
    pub struct PatchRpathTransform {
        pub binaries:   Vec<PathBuf>,
        pub old_prefix: String,
        pub new_prefix: String,
    }

    impl PatchRpathTransform {
        pub fn new(
            binaries: Vec<PathBuf>,
            old_prefix: impl Into<String>,
            new_prefix: impl Into<String>,
        ) -> Self {
            Self {
                binaries,
                old_prefix: old_prefix.into(),
                new_prefix: new_prefix.into(),
            }
        }
    }

    impl Transform for PatchRpathTransform {
        type Error = RpathError;

        fn apply<F: FileSystem, R: CommandRunner>(
            &self,
            staging_root: &Path,
            _fs: &F,
            _runner: &R,
        ) -> Result<(), Self::Error> {
            for binary in &self.binaries {
                let binary_path = staging_root.join(binary);
                patch_rpath(&binary_path, &self.old_prefix, &self.new_prefix)
                    .map_err(|e| RpathError {
                        binary: binary_path.clone(),
                        source: e,
                    })?;
            }
            Ok(())
        }
    }

    /// Run external process during transform.
    #[derive(Debug, Clone)]
    pub struct RunProcessTransform {
        pub cmd:  String,
        pub args: Vec<String>,
        pub env:  Vec<(String, String)>,
    }

    impl RunProcessTransform {
        pub fn new(
            cmd: impl Into<String>,
            args: Vec<String>,
            env: Vec<(String, String)>,
        ) -> Self {
            Self {
                cmd: cmd.into(),
                args,
                env,
            }
        }
    }

    impl Transform for RunProcessTransform {
        type Error = ProcessError;

        fn apply<F: FileSystem, R: CommandRunner>(
            &self,
            _staging_root: &Path,
            _fs: &F,
            runner: &R,
        ) -> Result<(), Self::Error> {
            runner.run(
                &self.cmd,
                self.args.iter(),
                self.env.iter().map(|(k, v)| (k.as_str(), v.as_str())),
            )
            .map_err(|e| ProcessError {
                cmd: self.cmd.clone(),
                source: e,
            })
        }
    }

    /// Set file permissions.
    #[derive(Debug, Clone)]
    pub struct SetPermissionsTransform {
        pub files: Vec<PathBuf>,
        pub mode:  u32,
    }

    impl SetPermissionsTransform {
        pub fn new(files: Vec<PathBuf>, mode: u32) -> Self {
            Self { files, mode }
        }
    }

    impl Transform for SetPermissionsTransform {
        type Error = PermissionsError;

        fn apply<F: FileSystem, R: CommandRunner>(
            &self,
            staging_root: &Path,
            fs: &F,
            _runner: &R,
        ) -> Result<(), Self::Error> {
            for file in &self.files {
                let file_path = staging_root.join(file);
                set_permissions(&file_path, self.mode)
                    .map_err(|e| PermissionsError {
                        file: file_path.clone(),
                        source: e,
                    })?;
            }
            Ok(())
        }
    }

    // Helper functions (moved from pipeline.rs)

    fn copy_dir_all<F: FileSystem>(
        src: &Path,
        dst: &Path,
        fs: &F,
    ) -> Result<(), io::Error> {
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

    fn rewrite_shebang(file: &Path, interpreter: &str) -> Result<(), io::Error> {
        if !file.exists() || !file.is_file() {
            return Ok(());
        }

        let content = std::fs::read_to_string(file)?;

        if !content.starts_with("#!") {
            return Ok(());
        }

        let new_content = format!("#!{}", interpreter);
        let rest = content.lines().skip(1).collect::<Vec<_>>().join("\n");
        let new_content = if rest.is_empty() {
            new_content
        } else {
            format!("{}\n{}", new_content, rest)
        };

        std::fs::write(file, new_content)
    }

    fn patch_rpath(binary: &Path, old_prefix: &str, new_prefix: &str) -> Result<(), io::Error> {
        // For now, this is a placeholder implementation
        // A real implementation would use tools like patchelf on Linux
        // or modify the binary directly

        // Check if binary exists and is executable
        if !binary.exists() {
            return Ok(()); // Skip if binary doesn't exist
        }

        let metadata = binary.metadata()?;

        if !metadata.is_file() {
            return Ok(()); // Skip if not a file
        }

        #[cfg(target_os = "linux")]
        {
            // Use patchelf if available, otherwise skip
            // This is a simplified version - real implementation would be more robust
            let output = std::process::Command::new("patchelf")
                .args(&["--print-rpath", &binary.to_string_lossy()])
                .output();

            if let Ok(output) = output {
                if output.status.success() {
                    let current_rpath = String::from_utf8_lossy(&output.stdout);
                    let new_rpath = current_rpath.replace(old_prefix, new_prefix);

                    if current_rpath != new_rpath {
                        std::process::Command::new("patchelf")
                            .args(&["--set-rpath", &new_rpath.trim(), &binary.to_string_lossy()])
                            .status()?;
                    }
                }
            }
            // If patchelf is not available, skip silently
        }

        #[cfg(not(target_os = "linux"))]
        {
            // On other platforms, RPATH patching might not be needed or use different tools
            let _ = (old_prefix, new_prefix);
        }

        Ok(())
    }

    fn set_permissions(file: &Path, mode: u32) -> Result<(), io::Error> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(mode))
        }
        #[cfg(windows)]
        {
            // Windows doesn't have simple chmod equivalent
            let _ = (file, mode);
            Ok(())
        }
    }

    // Error types

    #[derive(Debug, thiserror::Error)]
    pub enum RelocateError {
        #[error("failed to copy {from} to {to}: {source}")]
        Copy {
            from: PathBuf,
            to: PathBuf,
            source: io::Error,
        },
        #[error("failed to remove {path}: {source}")]
        Remove {
            path: PathBuf,
            source: io::Error,
        },
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ShebangError {
        #[error("failed to rewrite shebang for {file}: {source}")]
        Rewrite {
            file: PathBuf,
            source: io::Error,
        },
    }

    #[derive(Debug, thiserror::Error)]
    pub enum RpathError {
        #[error("failed to patch RPATH for {binary}: {source}")]
        Patch {
            binary: PathBuf,
            source: io::Error,
        },
    }

    #[derive(Debug, thiserror::Error)]
    pub enum ProcessError {
        #[error("failed to run process {cmd}: {source}")]
        Run {
            cmd: String,
            source: io::Error,
        },
    }

    #[derive(Debug, thiserror::Error)]
    pub enum PermissionsError {
        #[error("failed to set permissions for {file}: {source}")]
        Set {
            file: PathBuf,
            source: io::Error,
        },
    }
}
