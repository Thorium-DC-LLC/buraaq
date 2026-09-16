use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

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

fn main() {
    let Some(clang) = find_clang() else {
        eprintln!("no C compiler — skipping alloc bench");
        return;
    };

    let root = stdlib_root();
    let out_dir = std::env::temp_dir();
    let mut exe = out_dir.join("buraaq_alloc_bench");
    if cfg!(windows) {
        exe.set_extension("exe");
    }

    let status = Command::new(&clang)
        .arg(root.join("benches/support/alloc_bench.c"))
        .arg(root.join("runtime/buraaq_std.c"))
        .arg(format!("-I{}", root.join("runtime").display()))
        .arg("-O2")
        .arg("-o")
        .arg(&exe)
        .status()
        .expect("compile alloc bench");

    if !status.success() {
        eprintln!("failed to build alloc bench");
        std::process::exit(1);
    }

    let start = Instant::now();
    let run = Command::new(&exe).output().expect("run alloc bench");
    let elapsed = start.elapsed();

    if !run.status.success() {
        eprintln!(
            "bench failed:\n{}",
            String::from_utf8_lossy(&run.stderr)
        );
        std::process::exit(1);
    }

    print!(
        "{}",
        String::from_utf8_lossy(&run.stdout)
    );
    eprintln!("harness wall time: {elapsed:?}");
}
