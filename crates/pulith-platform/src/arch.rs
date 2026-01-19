//! Architecture detection.

/// CPU architecture types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86,
    X86_64,
    ARM,
    ARM64,
    Unknown,
}

/// Detect current architecture.
pub fn detect() -> Arch {
    let cpu_arch = sysinfo::System::cpu_arch();
    let arch_str = cpu_arch.as_str();

    match arch_str {
        "i386" | "i686" => Arch::X86,
        "x86_64" => Arch::X86_64,
        "arm" | "armv7l" => Arch::ARM,
        "aarch64" | "arm64" => Arch::ARM64,
        _ => Arch::Unknown,
    }
}

/// Convert to target triple format.
pub fn target_triple(arch: Arch) -> &'static str {
    match arch {
        Arch::X86 => "i686-unknown-linux-gnu",
        Arch::X86_64 => "x86_64-unknown-linux-gnu",
        Arch::ARM => "arm-unknown-linux-gnueabihf",
        Arch::ARM64 => "aarch64-unknown-linux-gnu",
        Arch::Unknown => "unknown-unknown-unknown",
    }
}
