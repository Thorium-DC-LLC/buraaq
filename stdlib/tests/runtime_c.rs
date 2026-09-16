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
fn c_runtime_unit_tests() {
    let Some(clang) = find_clang() else {
        eprintln!("no C compiler on PATH — skipping buraaq_std runtime tests");
        return;
    };

    let root = stdlib_root();
    let out_dir = std::env::var("OUT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let mut exe = out_dir.join("buraaq_std_test");
    if cfg!(windows) {
        exe.set_extension("exe");
    }

    let status = Command::new(&clang)
        .arg(root.join("runtime/buraaq_std.c"))
        .arg(root.join("runtime/buraaq_grid.c"))
        .arg(root.join("runtime/buraaq_hold.c"))
        .arg(root.join("runtime/buraaq_stream.c"))
        .arg(root.join("tests/support/test_runtime.c"))
        .arg(format!("-I{}", root.join("runtime").display()))
        .arg("-o")
        .arg(&exe)
        .args(if cfg!(windows) {
            vec!["-D_CRT_SECURE_NO_WARNINGS", "-lws2_32", "-lwininet"]
        } else {
            vec!["-lm", "-lpthread"]
        })
        .status()
        .expect("compile runtime test harness");

    assert!(status.success(), "failed to compile buraaq_std.c test harness");

    let run = Command::new(&exe).status().expect("run runtime test harness");
    assert!(run.success(), "buraaq_std runtime assertions failed");
}
