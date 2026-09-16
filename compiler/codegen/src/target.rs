use std::fmt;

/// Supported and planned target triples.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TargetTriple {
    X86_64LinuxGnu,
    X86_64WindowsMsvc,
    /// Reserved for future support.
    Aarch64AppleDarwin,
    Aarch64LinuxGnu,
    Wasm32Unknown,
}

impl TargetTriple {
    pub fn detect_host() -> Self {
        if cfg!(target_os = "windows") {
            Self::X86_64WindowsMsvc
        } else if cfg!(target_os = "macos") {
            Self::Aarch64AppleDarwin
        } else {
            Self::X86_64LinuxGnu
        }
    }

    pub fn llvm_triple(&self) -> &'static str {
        match self {
            Self::X86_64LinuxGnu => "x86_64-unknown-linux-gnu",
            Self::X86_64WindowsMsvc => "x86_64-pc-windows-msvc",
            Self::Aarch64AppleDarwin => "aarch64-apple-darwin",
            Self::Aarch64LinuxGnu => "aarch64-unknown-linux-gnu",
            Self::Wasm32Unknown => "wasm32-unknown-unknown",
        }
    }

    /// Native layout so clang SCEV/unroll see i32 as a machine integer.
    pub fn llvm_datalayout(&self) -> &'static str {
        match self {
            Self::X86_64WindowsMsvc => {
                "e-m:w-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
            }
            Self::X86_64LinuxGnu => {
                "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-i128:128-f80:128-n8:16:32:64-S128"
            }
            Self::Aarch64AppleDarwin | Self::Aarch64LinuxGnu => {
                "e-m:e-i8:8:32-i16:16:32-i64:64-i128:128-n32:64-S128"
            }
            Self::Wasm32Unknown => "e-m:e-p:32:32-p10:8:8-p20:8:8-i64:64-n32:64-S128-ni:1:10:20",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "x86_64-unknown-linux-gnu" | "x86_64-linux-gnu" | "linux" => Some(Self::X86_64LinuxGnu),
            "x86_64-pc-windows-msvc" | "windows" => Some(Self::X86_64WindowsMsvc),
            "aarch64-apple-darwin" | "macos" => Some(Self::Aarch64AppleDarwin),
            "aarch64-unknown-linux-gnu" => Some(Self::Aarch64LinuxGnu),
            "wasm32-unknown-unknown" | "wasm" => Some(Self::Wasm32Unknown),
            _ => None,
        }
    }

    pub fn exe_suffix(&self) -> &'static str {
        match self {
            Self::X86_64WindowsMsvc => ".exe",
            _ => "",
        }
    }
}

impl fmt::Display for TargetTriple {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.llvm_triple())
    }
}

/// Optimization profile matching ADR 0005.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub enum OptLevel {
    #[default]
    Debug,
    Release,
    ReleaseFast,
    Size,
}

impl OptLevel {
    pub fn from_release_flag(release: bool) -> Self {
        if release {
            Self::Release
        } else {
            Self::Debug
        }
    }

    pub fn clang_opt(&self) -> &'static str {
        match self {
            Self::Debug => "-O0",
            Self::Release => "-O2",
            Self::ReleaseFast => "-O3",
            Self::Size => "-Os",
        }
    }

    /// Extra linker flags (LTO, etc.).
    pub fn clang_link_flags(&self) -> &'static [&'static str] {
        match self {
            Self::ReleaseFast => &["-flto=thin"],
            _ => &[],
        }
    }

    pub fn llvm_opt(&self) -> &'static str {
        match self {
            Self::Debug => "-O0",
            Self::Release => "-O2",
            Self::ReleaseFast => "-O3",
            Self::Size => "-Os",
        }
    }

    pub fn mir_passes_enabled(&self) -> bool {
        !matches!(self, Self::Debug)
    }
}
