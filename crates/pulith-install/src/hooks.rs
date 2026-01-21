//! Hook traits for install lifecycle.
//!
//! Hooks allow injecting platform-specific or policy-specific behavior
//! without polluting the core mechanical logic.

use crate::data::InstallContext;
use crate::error::HookError;

/// Hook trait for install lifecycle events.
/// Implement this to add custom behavior at different pipeline stages.
pub trait InstallHook: Send + Sync {
    /// Name of this hook for error reporting.
    fn name(&self) -> &'static str;

    /// Called before staging begins.
    fn pre_stage(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(())
    }

    /// Called after staging completes.
    fn post_stage(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(())
    }

    /// Called before activation begins.
    fn pre_activate(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(())
    }

    /// Called after activation completes.
    fn post_activate(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(())
    }

    /// Called before rollback begins (on failure).
    fn pre_rollback(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(())
    }

    /// Called after commit completes successfully.
    fn post_commit(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(())
    }
}

/// Example hook implementations for common use cases.
/// Hook for Windows registry edits during activation.
pub struct WindowsRegistryHook {
    pub key:   String,
    pub value: String,
}

impl InstallHook for WindowsRegistryHook {
    fn name(&self) -> &'static str {
        "windows_registry"
    }

    #[cfg(target_os = "windows")]
    fn post_activate(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        // TODO: Implement Windows registry editing
        // For now, this is a placeholder
        Ok(())
    }

    #[cfg(not(target_os = "windows"))]
    fn post_activate(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(()) // No-op on non-Windows
    }
}

/// Hook for macOS codesigning after activation.
pub struct MacOSCodeSignHook {
    pub identity: String,
}

impl InstallHook for MacOSCodeSignHook {
    fn name(&self) -> &'static str {
        "macos_codesign"
    }

    #[cfg(target_os = "macos")]
    fn post_activate(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        // TODO: Implement codesign command
        Ok(())
    }

    #[cfg(not(target_os = "macos"))]
    fn post_activate(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(()) // No-op on non-macOS
    }
}

/// Hook for Linux ldconfig after activation.
pub struct LinuxLdconfigHook;

impl InstallHook for LinuxLdconfigHook {
    fn name(&self) -> &'static str {
        "linux_ldconfig"
    }

    #[cfg(target_os = "linux")]
    fn post_activate(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        // TODO: Run ldconfig
        Ok(())
    }

    #[cfg(not(target_os = "linux"))]
    fn post_activate(&self, _ctx: &mut InstallContext) -> Result<(), HookError> {
        Ok(()) // No-op on non-Linux
    }
}
