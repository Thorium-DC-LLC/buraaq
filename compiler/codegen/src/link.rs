use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use thiserror::Error;

use crate::target::{OptLevel, TargetTriple};

#[derive(Debug, Error)]
pub enum LinkError {
    #[error("linker not found: install clang/LLVM and ensure it is on PATH")]
    ToolchainMissing,
    #[error("link failed: {0}")]
    Failed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Clone, Debug)]
pub struct LinkOptions {
    pub target: TargetTriple,
    pub opt: OptLevel,
    pub output: PathBuf,
    pub emit_asm: bool,
    pub emit_ir_only: bool,
    pub extra_libs: Vec<String>,
}

/// Compile LLVM IR (+ runtime) to a native executable via clang.
pub fn link_executable(
    llvm_ir: &str,
    runtime_c: &[PathBuf],
    opts: &LinkOptions,
) -> Result<PathBuf, LinkError> {
    let tmp = tempfile::tempdir().map_err(LinkError::Io)?;
    let ll_path = tmp.path().join("module.ll");
    std::fs::write(&ll_path, llvm_ir)?;

    if opts.emit_ir_only {
        std::fs::write(&opts.output, llvm_ir)?;
        return Ok(opts.output.clone());
    }

    let clang = find_clang()?;
    let mut cmd = Command::new(&clang);
    cmd.arg(ll_path.as_os_str());
    if !opts.emit_asm {
        for rt in runtime_c {
            cmd.arg(rt);
        }
    }
    cmd.arg(opts.opt.clang_opt())
        .arg("-target")
        .arg(opts.target.llvm_triple());
    if opts.opt == OptLevel::Debug && !opts.emit_asm {
        cmd.arg("-g");
    }
    if !opts.emit_asm {
        for flag in opts.opt.clang_link_flags() {
            cmd.arg(flag);
        }
    }

    if opts.emit_asm {
        cmd.arg("-S").arg("-o").arg(&opts.output);
    } else {
        cmd.arg("-o").arg(&opts.output);
    }

    for lib in &opts.extra_libs {
        cmd.arg(format!("-l{lib}"));
    }

    if opts.target == TargetTriple::X86_64WindowsMsvc {
        cmd.arg("-D_CRT_SECURE_NO_WARNINGS");
        cmd.arg("-fuse-ld=lld");
        cmd.arg("-lwininet");
        cmd.arg("-lws2_32");
        cmd.arg("-luser32");
        cmd.arg("-lgdi32");
        cmd.arg("-lole32");
        cmd.arg("-loleaut32");
        cmd.arg("-luiautomationcore");
        cmd.arg("-Wl,/STACK:16777216");
    } else {
        cmd.arg("-lpthread");
        cmd.arg("-lm");
        if posix_openssl_available() {
            cmd.arg("-DBURAAQ_OPENSSL");
            cmd.arg("-lssl");
            cmd.arg("-lcrypto");
        }
    }

    let output = cmd.output()?;
    if !output.status.success() {
        return Err(LinkError::Failed(
            String::from_utf8_lossy(&output.stderr).into_owned(),
        ));
    }

    Ok(opts.output.clone())
}

pub fn clang_available() -> bool {
    find_clang().is_ok()
}

/// Path to the clang the linker will use (PATH, BURAAQ_CLANG, or Buraaq's LLVM sidecar).
pub fn clang_path() -> Option<PathBuf> {
    find_clang().ok()
}

fn posix_openssl_available() -> bool {
    static CACHED: OnceLock<bool> = OnceLock::new();
    *CACHED.get_or_init(|| {
        if Command::new("pkg-config")
            .args(["--exists", "openssl"])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
        {
            return true;
        }
        const LIBS: &[&str] = &["libssl.so", "libssl.so.3", "libssl.so.1.1", "libssl.dylib", "libssl.a"];
        const DIRS: &[&str] = &[
            "/usr/lib",
            "/usr/lib64",
            "/usr/lib/x86_64-linux-gnu",
            "/usr/local/lib",
            "/opt/homebrew/lib",
            "/opt/homebrew/opt/openssl/lib",
        ];
        for dir in DIRS {
            for lib in LIBS {
                if Path::new(dir).join(lib).exists() {
                    return true;
                }
            }
        }
        false
    })
}

fn find_clang() -> Result<PathBuf, LinkError> {
    let mut candidates: Vec<PathBuf> = ["clang", "clang-18", "clang-17", "clang-cl"]
        .iter()
        .map(PathBuf::from)
        .collect();
    if let Ok(root) = std::env::var("BURAAQ_CLANG") {
        candidates.insert(0, PathBuf::from(root));
    }
    if let Ok(home) = std::env::var("BURAAQ_HOME") {
        let home = PathBuf::from(home);
        candidates.push(home.join("llvm/bin/clang.exe"));
        candidates.push(home.join("llvm/bin/clang"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(PathBuf::from(local).join("buraaq/llvm/bin/clang.exe"));
    }
    if let Ok(home) = std::env::var("HOME") {
        candidates.push(PathBuf::from(home).join(".local/share/buraaq/llvm/bin/clang"));
    }
    candidates.extend([
        PathBuf::from(r"C:\Program Files\LLVM\bin\clang.exe"),
        PathBuf::from(r"C:\Program Files\LLVM\bin\clang-cl.exe"),
        PathBuf::from("/usr/bin/clang"),
        PathBuf::from("/usr/local/bin/clang"),
        PathBuf::from("/opt/homebrew/bin/clang"),
    ]);
    for name in candidates {
        if Command::new(&name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Ok(name);
        }
    }
    Err(LinkError::ToolchainMissing)
}
