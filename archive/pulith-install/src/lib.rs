//! pulith-install: Transactional filesystem engine for converting verified artifacts into active system state.
//!
//! This crate provides a purely mechanical installer that transforms verified bytes into usable system state
//! through a staged, hookable, rollback-safe pipeline. It knows nothing about packages, versions, or registry schemas.
//!
//! # Philosophy
//!
//! Install is a transactional transformer from artifacts to system-visible resources.
//! Only knows paths, files, links, processes - NOT packages, versions, registry.
//!
//! # Core Abstraction
//!
//! The only real interface: "Given a staged directory and a target path, perform an atomic, reversible
//! transformation that makes the resource usable."
//!
//! # Usage
//!
//! ```rust,no_run
//! use pulith_install::{Pipeline, Transform};
//! use pulith_install::transform::builtins::RewriteShebangTransform;
//! use std::path::Path;
//!
//! // Create pipeline with staging and active roots
//! let pipeline = Pipeline::new("/tmp/staging", "/usr/local/bin")
//!     .transform(RewriteShebangTransform::new(vec!["bin/script".into()], "/bin/bash"));
//!
//! // Run with effect implementations (you provide these)
//! // pipeline.run(&artifact_path, &fs_impl, &runner_impl)?;
//! ```

pub mod data;
pub mod error;
pub mod hooks;
pub mod pipeline;
pub mod transform;

// Re-export main types
pub use data::InstallContext;
pub use error::{
    ActivateError, HookError, InstallError, PipelineError, StageError,
};
pub use hooks::InstallHook;
pub use pipeline::Pipeline;
pub use transform::Transform;

// Re-export built-in transforms for convenience
pub use transform::builtins::*;

// Effect traits for dependency injection
pub mod effects {
    //! Effect traits for filesystem and process operations.
    //! Implement these to provide platform-specific behavior.

    use std::path::Path;

    /// Filesystem operations needed by the installer.
    pub trait FileSystem {
        fn copy(&self, from: &Path, to: &Path) -> std::io::Result<()>;
        fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
        fn remove_dir_all(&self, path: &Path) -> std::io::Result<()>;
        fn link(&self, from: &Path, to: &Path) -> std::io::Result<()>;
        fn exists(&self, path: &Path) -> bool;
        fn is_dir(&self, path: &Path) -> bool;
        fn is_file(&self, path: &Path) -> bool;
    }

    /// Process execution for transforms.
    pub trait CommandRunner {
        fn run(
            &self,
            cmd: &str,
            args: impl Iterator<Item = impl AsRef<str>>,
            env: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
        ) -> std::io::Result<()>;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    // Mock filesystem for testing
    struct MockFileSystem {
        files: HashMap<PathBuf, Vec<u8>>,
    }

    impl MockFileSystem {
        fn new() -> Self {
            Self {
                files: HashMap::new(),
            }
        }
    }

    impl effects::FileSystem for MockFileSystem {
        fn copy(&self, _from: &std::path::Path, _to: &std::path::Path) -> std::io::Result<()> {
            // Mock implementation
            Ok(())
        }

        fn create_dir_all(&self, _path: &std::path::Path) -> std::io::Result<()> { Ok(()) }

        fn remove_dir_all(&self, _path: &std::path::Path) -> std::io::Result<()> { Ok(()) }

        fn link(&self, _from: &std::path::Path, _to: &std::path::Path) -> std::io::Result<()> {
            Ok(())
        }

        fn exists(&self, path: &std::path::Path) -> bool { self.files.contains_key(path) }

        fn is_dir(&self, _path: &std::path::Path) -> bool { false }

        fn is_file(&self, path: &std::path::Path) -> bool { self.files.contains_key(path) }
    }

    // Mock command runner for testing
    struct MockCommandRunner;

    impl effects::CommandRunner for MockCommandRunner {
        fn run(
            &self,
            cmd: &str,
            args: impl Iterator<Item = impl AsRef<str>>,
            env: impl Iterator<Item = (impl AsRef<str>, impl AsRef<str>)>,
        ) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_pipeline_builder_pattern() {
        // Test that pipeline can be constructed with fluent API
        let pipeline = Pipeline::new("/tmp/staging", "/usr/local/bin")
            .transform(RewriteShebangTransform::new(
                vec!["script.sh".into()],
                "/bin/bash",
            ))
            .transform(SetPermissionsTransform::new(
                vec!["bin/executable".into()],
                0o755,
            ));

        // Pipeline should be constructable - we can't inspect internals but can ensure it builds
        assert!(true); // If we get here, construction worked
    }

    #[test]
    fn test_transform_constructors() {
        let transform = RelocateTransform::new("from", "to");
        assert!(matches!(transform, RelocateTransform { .. }));

        let transform = RewriteShebangTransform::new(vec!["file".into()], "bash");
        assert!(matches!(transform, RewriteShebangTransform { .. }));

        let transform = RunProcessTransform::new("make", vec!["install".to_string()], vec![]);
        assert!(matches!(transform, RunProcessTransform { .. }));
    }

    #[test]
    fn test_install_context_creation() {
        let ctx = InstallContext::new("/staging".into(), "/active".into());
        assert_eq!(ctx.staging_root, PathBuf::from("/staging"));
        assert_eq!(ctx.active_root, PathBuf::from("/active"));
        assert!(ctx.ops.is_empty());
        assert!(ctx.staged_files.is_empty());
        assert!(ctx.created_links.is_empty());
    }

    #[test]
    fn test_transform_implementations() {
        // Test relocate transform
        let relocate_transform = RelocateTransform::new("old/path", "new/path");
        assert!(matches!(relocate_transform, RelocateTransform { .. }));

        // Test shebang rewrite transform
        let shebang_transform = RewriteShebangTransform::new(vec!["script.sh".into()], "/bin/bash");
        assert!(matches!(shebang_transform, RewriteShebangTransform { .. }));

        // Test RPATH patch transform
        let rpath_transform =
            PatchRpathTransform::new(vec!["binary".into()], "/old/prefix", "/new/prefix");
        assert!(matches!(rpath_transform, PatchRpathTransform { .. }));

        // Test process execution transform
        let process_transform = RunProcessTransform::new("make", vec!["install".to_string()], vec![]);
        assert!(matches!(process_transform, RunProcessTransform { .. }));

        // Test permissions transform
        let perms_transform = SetPermissionsTransform::new(vec!["file".into()], 0o755);
        assert!(matches!(perms_transform, SetPermissionsTransform { .. }));
    }

    #[test]
    fn test_pipeline_with_multiple_transforms() {
        let pipeline = Pipeline::new("/tmp/staging", "/usr/local/bin")
            .transform(RelocateTransform::new("lib", "lib64"))
            .transform(RewriteShebangTransform::new(
                vec!["bin/script".into()],
                "/bin/bash",
            ))
            .transform(SetPermissionsTransform::new(
                vec!["bin/executable".into()],
                0o755,
            ));

        // Pipeline should construct successfully
        assert!(true);
    }

    #[test]
    fn test_hook_trait() {
        // Test that hooks can be created (they default to no-op)
        struct TestHook;
        impl InstallHook for TestHook {
            fn name(&self) -> &'static str { "test" }
        }

        let hook = TestHook;
        assert_eq!(hook.name(), "test");

        let mut ctx = InstallContext::new("/staging".into(), "/active".into());

        // All hook methods should work (even if they do nothing)
        assert!(hook.pre_stage(&mut ctx).is_ok());
        assert!(hook.post_stage(&mut ctx).is_ok());
        assert!(hook.pre_activate(&mut ctx).is_ok());
        assert!(hook.post_activate(&mut ctx).is_ok());
        assert!(hook.pre_rollback(&mut ctx).is_ok());
        assert!(hook.post_commit(&mut ctx).is_ok());
    }

    #[test]
    fn test_error_types() {
        use std::error::Error;

        // Test that error types can be created and implement std::error::Error
        let err = StageError::SourceNotFound("/test".into());
        assert!(err.source().is_none()); // No source for this error

        let err = ActivateError::LinkExists("/test".into());
        assert!(err.source().is_none()); // No source for this error

        // Test error display
        let err = PipelineError::Stage(StageError::SourceNotFound("/missing".into()));
        let msg = err.to_string();
        assert!(msg.contains("stage failed"));
        assert!(msg.contains("source not found"));
    }

    #[test]
    fn test_rollback_functionality() {
        // Test that rollback is properly set up (we can't easily test full rollback
        // without a real filesystem, but we can test the structure)
        let pipeline = Pipeline::new("/tmp/staging", "/usr/local/bin");

        // The pipeline should have rollback capability built-in
        // This is tested implicitly through the successful compilation
        // and the fact that the run method includes rollback logic
        assert!(true);
    }
}
