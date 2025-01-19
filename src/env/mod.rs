#[derive(Debug, Clone, Copy)]
enum OS {
    Windows,
    Macos,
    Linux(Linux),
}
#[derive(Debug, Clone, Copy)]
enum Linux {
    Debian,
    Ubuntu,
    LinuxMint,
    Fedora,
    RedHatEnterpriseLinux,
    CentOS,
    ArchLinux,
    Manjaro,
    OpenSUSE,
    Gentoo,
    AlpineLinux,
    KaliLinux,
    Unknown,
}

#[derive(Debug, Clone, Copy)]
enum Arch {
    X86,
    X86_64,
    ARM,
    ARM64,
    Unknown,
}

impl From<&str> for Linux {
    fn from(s: &str) -> Linux {
        match s {
            "Debian GNU/Linux" => Linux::Debian,
            "Ubuntu" => Linux::Ubuntu,
            "Linux Mint" => Linux::LinuxMint,
            "Fedora" => Linux::Fedora,
            "Red Hat Enterprise Linux" => Linux::RedHatEnterpriseLinux,
            "CentOS Linux" => Linux::CentOS,
            "Arch Linux" => Linux::ArchLinux,
            "Manjaro Linux" => Linux::Manjaro,
            "openSUSE Leap" | "openSUSE Tumbleweed" => Linux::OpenSUSE,
            "Gentoo" => Linux::Gentoo,
            "Alpine Linux" => Linux::AlpineLinux,
            "Kali Linux" => Linux::KaliLinux,
            _ => Linux::Unknown,
        }
    }
}

impl From<&str> for Arch {
    fn from(s: &str) -> Self {
        match s {
            "i386" | "i686" => Arch::X86,
            "x86_64" => Arch::X86_64,
            "arm" | "armv7l" => Arch::ARM,
            "aarch64" => Arch::ARM64,
            _ => Arch::Unknown,
        }
    }
}
