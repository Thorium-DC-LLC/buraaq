//! Engineer suite — examples/engineer-suite/manifest.tsv (run mode, non-experimental).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, compile_to_executable, BuildOptions};
use buraaq_pkg::Project;

fn suite_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/engineer-suite")
}

fn clang_ok() -> bool {
    buraaq_codegen::clang_available()
}

struct Case {
    id: String,
    rel: PathBuf,
    kind: String,
    expect: String,
}

fn load_manifest() -> Vec<Case> {
    let text = fs::read_to_string(suite_root().join("manifest.tsv")).expect("manifest.tsv");
    let mut out = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 5 || cols[3] != "run" || cols[0].starts_with("exp_") {
            continue;
        }
        out.push(Case {
            id: cols[0].into(),
            rel: PathBuf::from(cols[1]),
            kind: cols[2].into(),
            expect: cols[4].into(),
        });
    }
    out
}

fn run_single(path: &Path) -> String {
    let handler = StandardHandler::new();
    let out = compile_to_executable(path, &handler, &BuildOptions::default())
        .unwrap_or_else(|e| panic!("compile {}: {e:?} {:?}", path.display(), handler.diagnostics()));
    let exe = out.executable.expect("exe");
    let run = Command::new(&exe).output().expect("run");
    assert!(
        run.status.success(),
        "{} stderr: {}",
        path.display(),
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).into_owned()
}

fn run_project(dir: &Path) -> String {
    let project = Project::discover(dir).expect("discover project");
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.output = Some(dir.join("target").join("suite.exe"));
    let out = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("project {}: {e:?} {:?}", dir.display(), handler.diagnostics()))
        .executable
        .expect("exe");
    let run = Command::new(&out)
        .current_dir(dir)
        .output()
        .expect("run project");
    assert!(
        run.status.success(),
        "{} stderr: {}",
        dir.display(),
        String::from_utf8_lossy(&run.stderr)
    );
    String::from_utf8_lossy(&run.stdout).into_owned()
}

#[test]
fn engineer_suite_required_programs_run() {
    if !clang_ok() {
        eprintln!("skipping engineer suite: clang not on PATH");
        return;
    }
    let root = suite_root();
    assert!(root.is_dir(), "missing {}", root.display());
    let mut failures = Vec::new();

    for case in load_manifest() {
        let stdout = match case.kind.as_str() {
            "single" => run_single(&root.join(&case.rel)),
            "project" => run_project(&root.join(&case.rel)),
            other => panic!("unknown kind {other}"),
        };

        if !case.expect.is_empty() && !stdout.contains(&case.expect) {
            failures.push(format!(
                "{}: stdout missing `{}` got {:?}",
                case.id, case.expect, stdout
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "engineer suite failures:\n{}",
        failures.join("\n")
    );
}
