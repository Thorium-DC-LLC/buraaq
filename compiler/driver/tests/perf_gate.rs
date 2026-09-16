//! Gate B — equivalent-workload Buraaq `--release` vs C++ `-O2`.
//! Times only the timed loop. Same `n` on both sides. Reports wins and losses.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, BuildOptions};
use buraaq_pkg::{create_new, Project};

const BENCHES: &[(&str, &str)] = &[
    ("numerical_loop", "100000000"),
    ("integer_sum", "100000000"),
    ("nested_loop", "10000"),
    ("float_saxpy", "10000000"),
    ("fib_iter", "100000000"),
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn clangxx() -> Option<PathBuf> {
    for name in [
        "clang++",
        r"C:\Program Files\LLVM\bin\clang++.exe",
        "/usr/bin/clang++",
    ] {
        if Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(PathBuf::from(name));
        }
    }
    None
}

fn parse_time_sec(stdout: &str) -> Option<f64> {
    for line in stdout.lines() {
        if !line.contains("BENCH") {
            continue;
        }
        for key in ["time_sec=", "elapsed_sec="] {
            if let Some(rest) = line.split(key).nth(1) {
                let num = rest
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_matches(|c: char| !c.is_ascii_digit() && c != '.' && c != '-');
                if let Ok(v) = num.parse::<f64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

fn compile_buraaq_bench(name: &str, src: &str, out_dir: &Path) -> PathBuf {
    let project = create_new(name, out_dir).unwrap();
    fs::write(project.root.join("src/main.bq"), src).unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.release = true;
    opts.output = Some(out_dir.join(format!("{name}_bq.exe")));
    compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("{name} buraaq compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe")
}

fn run_exe(exe: &Path) -> String {
    let out = Command::new(exe)
        .output()
        .unwrap_or_else(|e| panic!("run {}: {e}", exe.display()));
    assert!(
        out.status.success(),
        "{} failed: {}",
        exe.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn equivalent_workloads_within_2x_cpp_o2() {
    let cxx = clangxx().expect("clang++ required for Gate B");
    assert!(
        buraaq_codegen::clang_available(),
        "clang required for Gate B"
    );
    let root = repo_root();
    let tmp = tempfile::tempdir().unwrap();
    let mut wins = Vec::new();
    let mut losses = Vec::new();

    for (name, expected_n) in BENCHES {
        let bq_src = fs::read_to_string(root.join(format!("benchmarks/buraaq/{name}.bq")))
            .unwrap_or_else(|e| panic!("read {name}.bq: {e}"));
        assert!(
            bq_src.contains(expected_n),
            "{name}.bq must use n={expected_n}"
        );
        let cpp_src = root.join(format!("benchmarks/cpp/{name}.cpp"));
        let cpp_text = fs::read_to_string(&cpp_src).unwrap();
        assert!(
            cpp_text.contains(expected_n),
            "{name}.cpp must use n={expected_n}"
        );

        let cpp_exe = tmp.path().join(format!("{name}_cpp.exe"));
        let st = Command::new(&cxx)
            .args(["-O2", "-std=c++17"])
            .arg(&cpp_src)
            .arg("-o")
            .arg(&cpp_exe)
            .status()
            .expect("clang++");
        assert!(st.success(), "C++ -O2 compile failed for {name}");

        let bq_exe = compile_buraaq_bench(name, &bq_src, tmp.path());
        let cpp_out = run_exe(&cpp_exe);
        let bq_out = run_exe(&bq_exe);
        let cpp_t = parse_time_sec(&cpp_out).unwrap_or_else(|| panic!("no BENCH in C++:\n{cpp_out}"));
        let bq_t = parse_time_sec(&bq_out).unwrap_or_else(|| panic!("no BENCH in Buraaq:\n{bq_out}"));
        assert!(
            cpp_t > 0.0,
            "{name} C++ time must be positive cpp={cpp_t}"
        );
        let ratio = if bq_t <= 0.0 {
            0.0
        } else {
            bq_t / cpp_t
        };
        eprintln!(
            "GATE_B {name} cpp_o2={cpp_t:.6}s buraaq_release={bq_t:.6}s ratio={ratio:.3}x"
        );
        if ratio <= 2.0 {
            wins.push((*name, ratio));
        } else {
            losses.push((*name, ratio));
        }
        let _ = Duration::from_secs_f64(bq_t);
    }

    eprintln!("GATE_B wins={wins:?} losses={losses:?}");
    assert!(
        losses.is_empty(),
        "Buraaq --release must be within 2× C++ -O2 on equivalent n; losses={losses:?} wins={wins:?}"
    );
}

#[test]
fn integer_sum_orbit_matches_wrapping_loop_n10() {
    if !buraaq_codegen::clang_available() {
        return;
    }
    let src = r#"
fn main() {
    n = 10
    mut sum = 0
    mut i = 1
    while i <= n {
        sum = sum * 3 + i
        i = i + 1
    }
    print_int(sum)
}
"#;
    let tmp = tempfile::tempdir().unwrap();
    let exe = compile_buraaq_bench("orbit10", src, tmp.path());
    let out = run_exe(&exe);
    assert!(
        out.contains("44281"),
        "Orbit fold must match wrapping sum*3+i for n=10, got {out}"
    );
}
