//! Locate the Buraaq standard library (sysroot).

use std::path::{Path, PathBuf};

/// Directories that may contain `src/*.bq` stdlib modules.
pub fn discover_sysroot(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        if looks_like_sysroot(p) {
            return Some(p.to_path_buf());
        }
    }
    if let Ok(env) = std::env::var("BURAAQ_SYSROOT") {
        let p = PathBuf::from(env);
        if looks_like_sysroot(&p) {
            return Some(p);
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            for candidate in [
                dir.join("sysroot"),
                dir.join("../sysroot"),
                dir.join("../../stdlib"),
                dir.join("../../../stdlib"),
            ] {
                if looks_like_sysroot(&candidate) {
                    return Some(candidate.canonicalize().unwrap_or(candidate));
                }
            }
        }
    }
    let from_pkg = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../stdlib");
    if looks_like_sysroot(&from_pkg) {
        return Some(from_pkg.canonicalize().unwrap_or(from_pkg));
    }
    None
}

pub fn looks_like_sysroot(path: &Path) -> bool {
    path.join("src").join("io.bq").exists() || path.join("src").is_dir() && path.join("runtime").exists()
}

pub fn stdlib_src(sysroot: &Path) -> PathBuf {
    sysroot.join("src")
}

/// Map `std.io` → `<sysroot>/src/io.bq`.
pub fn std_module_path(sysroot: &Path, dotted: &str) -> Option<PathBuf> {
    let rest = dotted.strip_prefix("std.")?;
    let mut p = stdlib_src(sysroot);
    for (i, part) in rest.split('.').enumerate() {
        if i + 1 == rest.split('.').count() {
            p.push(format!("{part}.bq"));
        } else {
            p.push(part);
        }
    }
    if p.exists() {
        Some(p)
    } else {
        None
    }
}
