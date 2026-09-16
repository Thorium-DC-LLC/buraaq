use std::path::PathBuf;
use std::process::Command;

fn stdlib_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn find_clang() -> Option<String> {
    for name in ["clang", "clang-18", "clang-17", "gcc", "cc"] {
        if Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(name.to_string());
        }
    }
    None
}

#[test]
fn concurrency_runtime_unit_tests() {
    let Some(clang) = find_clang() else {
        eprintln!("no C compiler — skipping concurrency runtime tests");
        return;
    };

    let root = stdlib_root();
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let mut exe = out_dir.join("buraaq_runtime_concurrency_test");
    if cfg!(windows) {
        exe.set_extension("exe");
    }

    let mut cmd = Command::new(&clang);
    cmd.arg(root.join("runtime/buraaq_runtime.c"))
        .arg(root.join("tests/support/test_runtime_concurrency.c"))
        .arg(format!("-I{}", root.join("runtime").display()))
        .arg("-o")
        .arg(&exe);
    if !cfg!(windows) {
        cmd.arg("-lpthread");
    }
    let status = cmd.status().expect("compile concurrency test");
    assert!(status.success(), "failed to compile buraaq_runtime concurrency test");

    let run = Command::new(&exe).status().expect("run concurrency test");
    assert!(run.success());
}
