//! Gate H — `new` / path-dep compile / typed drop IR.

use std::fs;
use std::process::Command;

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, BuildOptions};
use buraaq_pkg::{create_new, Manifest, Project, run_tests};

fn clang_ok() -> bool {
    buraaq_codegen::clang_available()
}

#[test]
fn path_dependency_compiles_and_runs() {
    assert!(clang_ok(), "clang required");
    let tmp = tempfile::tempdir().unwrap();
    let lib = create_new("utilib", tmp.path()).unwrap();
    fs::write(
        lib.root.join("src/extra.bq"),
        "pub fn extra() -> int {\n    9\n}\n",
    )
    .unwrap();

    let app = create_new("appdep", tmp.path()).unwrap();
    let mut manifest = Manifest::load(&app.root.join("buraaq.pkg")).unwrap();
    manifest.dependencies.insert(
        "utilib".into(),
        buraaq_pkg::DependencySpec::Detailed(buraaq_pkg::DetailedDep {
            version: None,
            path: Some(std::path::PathBuf::from("../utilib")),
            git: None,
            rev: None,
            registry: None,
        }),
    );
    manifest.save(&app.root.join("buraaq.pkg")).unwrap();
    fs::write(
        app.root.join("src/main.bq"),
        "use extra.extra\n\nfn main() {\n    print(extra())\n}\n",
    )
    .unwrap();

    let project = Project::discover(&app.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.output = Some(app.root.join("appdep.exe"));
    let exe = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("path-dep compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe");
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "9");
}

#[test]
fn while_counter_increments() {
    if !clang_ok() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("loopinc", tmp.path()).unwrap();
    fs::write(
        project.root.join("src/main.bq"),
        r#"
fn main() {
    i = 0
    while i < 5 {
        i = i + 1
    }
    print(i)
}
"#,
    )
    .unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.mir_opt = false;
    opts.output = Some(project.root.join("loopinc.exe"));
    let exe = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("loop compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe");
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "5");
}

#[test]
fn while_param_increments() {
    if !clang_ok() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("loopparam", tmp.path()).unwrap();
    fs::write(
        project.root.join("src/main.bq"),
        r#"
fn walk(pos: int, n: int) -> int {
    while pos < n {
        pos = pos + 1
    }
    pos
}

fn main() {
    print(walk(0, 5))
}
"#,
    )
    .unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.mir_opt = false;
    opts.output = Some(project.root.join("loopparam.exe"));
    let exe = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("param loop: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe");
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "5");
}

#[test]
fn created_project_test_blocks_pass() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("smokeapp", tmp.path()).unwrap();
    let results = run_tests(&project.root).expect("run_tests");
    assert_eq!(results.failed, 0, "{:?}", results.failures);
    assert!(results.passed >= 1);
}

#[test]
fn typed_drop_lowers_to_struct_drop() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("droper", tmp.path()).unwrap();
    fs::write(
        project.root.join("src/main.bq"),
        r#"
struct Holder {
    n: int
    drop {
        print(self.n)
    }
}

fn main() {
    h = Holder { n: 3 }
}
"#,
    )
    .unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    opts.output = Some(project.root.join("droper.ll"));
    let out = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("drop IR: {e:?} {:?}", handler.diagnostics()));
    assert!(
        out.llvm_ir.contains("Holder_drop") || out.llvm_ir.contains("@Holder_drop"),
        "typed drop must lower:\n{}",
        out.llvm_ir
    );
}

#[test]
fn stdlib_sha256_is_real() {
    if !clang_ok() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("sha", tmp.path()).unwrap();
    fs::write(
        project.root.join("src/main.bq"),
        r#"
use std.crypto.sha256

fn main() {
    print(sha256("hi"))
}
"#,
    )
    .unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.output = Some(project.root.join("sha.exe"));
    let exe = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("sha compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe");
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    let hex = String::from_utf8_lossy(&run.stdout).trim().to_string();
    assert_eq!(
        hex,
        "8f434346648f6b96df89dda901c5176b10a6d83961dd3c1ac88b59b2dc327aa4"
    );
}

#[test]
fn stdlib_json_number_field() {
    if !clang_ok() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("jsn", tmp.path()).unwrap();
    fs::write(project.root.join("payload.json"), "{\"n\":42}").unwrap();
    let path = project
        .root
        .join("payload.json")
        .to_string_lossy()
        .replace('\\', "/");
    fs::write(
        project.root.join("src/main.bq"),
        format!(
            r#"
use std.fs.read
use std.json.parse

fn main() {{
    v = parse(read("{path}"))
    print(v.field("n"))
}}
"#
        ),
    )
    .unwrap();
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.output = Some(project.root.join("jsn.exe"));
    let exe = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("json compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe");
    let run = Command::new(&exe).output().unwrap();
    assert!(run.status.success(), "{}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "42");
}
