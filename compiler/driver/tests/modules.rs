use std::fs;
use std::process::Command;

use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_driver::{compile_project, BuildOptions};
use buraaq_frontend::analyze_project;
use buraaq_pkg::{create_new, Project};

fn clang_ok() -> bool {
    for name in [
        "clang",
        r"C:\Program Files\LLVM\bin\clang.exe",
        "/usr/bin/clang",
    ] {
        if Command::new(name)
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return true;
        }
    }
    false
}

fn write_pkg(root: &std::path::Path, files: &[(&str, &str)]) {
    for (rel, src) in files {
        let path = root.join(rel);
        if let Some(p) = path.parent() {
            fs::create_dir_all(p).unwrap();
        }
        fs::write(path, src).unwrap();
    }
}

#[test]
fn two_modules_analyze_and_compile() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("twomod", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[
            (
                "src/main.bq",
                r#"use util.add

fn main() {
    print(add(2, 3))
}
"#,
            ),
            (
                "src/util.bq",
                r#"pub fn add(a: int, b: int) -> int {
    a + b
}
"#,
            ),
        ],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let world = analyze_project(&project, &handler, None);
    assert!(!world.had_errors, "analyze errors: {:?}", handler.diagnostics());
    assert!(world.unit("util").is_some());
    assert!(world.unit("main").is_some());

    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    opts.output = Some(project.root.join("twomod.ll"));
    let out = compile_project(&project, &handler, &opts).expect("emit IR for two modules");
    assert!(
        out.llvm_ir.contains("define") && (out.llvm_ir.contains("@add") || out.llvm_ir.contains("@main")),
        "expected lowered functions in LLVM IR"
    );
    assert!(
        !out.llvm_ir.contains("alloca void"),
        "void results must not be stack-allocated"
    );

    assert!(
        clang_ok(),
        "clang must be available for native multi-module 1.0 verification"
    );
    opts.emit_ir = false;
    opts.output = Some(project.root.join("twomod_out.exe"));
    let linked = compile_project(&project, &handler, &opts).expect("compile two modules");
    let exe = linked.executable.expect("executable");
    let run = Command::new(&exe).output().expect("run two-module exe");
    assert!(run.status.success(), "exe failed: {}", String::from_utf8_lossy(&run.stderr));
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "5",
        "two-module program should print add(2,3)"
    );
}

#[test]
fn private_access_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("priv", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[
            (
                "src/main.bq",
                "use database.connect\nfn main() { connect() }\n",
            ),
            ("src/database.bq", "fn connect() { }\n"),
        ],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    let codes: Vec<_> = handler
        .diagnostics()
        .iter()
        .filter_map(|d| d.code.clone())
        .collect();
    assert!(
        codes.iter().any(|c| c == "E0412"),
        "expected E0412, got {codes:?} {:?}",
        handler.diagnostics()
    );
}

#[test]
fn unresolved_import_is_error() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("missing", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            "use no_such_module.foo\nfn main() {}\n",
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    assert!(handler
        .diagnostics()
        .iter()
        .any(|d| d.code.as_deref() == Some("E0403")));
}

#[test]
fn cyclic_modules_reported() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("cycle", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[
            ("src/main.bq", "use a.ping\nfn main() { ping() }\n"),
            ("src/a.bq", "use b.pong\npub fn ping() { pong() }\n"),
            ("src/b.bq", "use a.ping\npub fn pong() { ping() }\n"),
        ],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    assert!(
        handler
            .diagnostics()
            .iter()
            .any(|d| d.code.as_deref() == Some("E0401")),
        "expected cycle E0401, got {:?}",
        handler.diagnostics()
    );
}

#[test]
fn ten_modules_analyze() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("ten", tmp.path()).unwrap();
    let mut files = vec![(
        "src/main.bq".to_string(),
        "use m0.f0\nfn main() { print(f0()) }\n".to_string(),
    )];
    for i in 0..10 {
        let next = if i + 1 < 10 {
            format!(
                "use m{}.f{}\npub fn f{i}() -> int {{ f{}() }}\n",
                i + 1,
                i + 1,
                i + 1
            )
        } else {
            format!("pub fn f{i}() -> int {{ 42 }}\n")
        };
        files.push((format!("src/m{i}.bq"), next));
    }
    let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    write_pkg(&project.root, &refs);
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let world = analyze_project(&project, &handler, None);
    assert!(!handler.has_errors(), "{:?}", handler.diagnostics());
    assert!(world.units.len() >= 11);

    assert!(clang_ok(), "clang required for 10-module native gate");
    let mut opts = BuildOptions::default();
    opts.output = Some(project.root.join("ten.exe"));
    let exe = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("10-module compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe");
    let run = Command::new(&exe).output().expect("run 10-module");
    assert!(
        run.status.success(),
        "10-module exe failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&run.stdout).trim(), "42");
}

#[test]
fn imported_struct_emits_ir() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("istruct", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[
            (
                "src/main.bq",
                r#"use geom.Point

fn main() {
    p = Point { x: 1, y: 2 }
    print(p.x)
}
"#,
            ),
            (
                "src/geom.bq",
                r#"pub struct Point {
    x: int
    y: int
}
"#,
            ),
        ],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let world = analyze_project(&project, &handler, None);
    assert!(!handler.has_errors(), "{:?}", handler.diagnostics());
    assert!(world.unit("geom").is_some());
    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    opts.output = Some(project.root.join("istruct.ll"));
    compile_project(&project, &handler, &opts).expect("imported struct should lower");
}

#[test]
fn imported_enum_and_trait_analyze() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("ienum", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[
            (
                "src/main.bq",
                "use color.Color\nuse color.Show\nfn main() {}\n",
            ),
            (
                "src/color.bq",
                r#"pub enum Color {
    Red,
    Blue,
}

pub trait Show {
    fn to_text(self) -> text
}
"#,
            ),
        ],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    assert!(!handler.has_errors(), "{:?}", handler.diagnostics());
}

#[test]
fn stdlib_import_resolves() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("stdio", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            "use std.io.println\nfn main() { println(\"hi\") }\n",
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    assert!(
        !handler.has_errors(),
        "std.io.println should resolve: {:?}",
        handler.diagnostics()
    );
}

#[test]
fn auto_import_std_without_use() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("autoio", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            r#"fn main() {
    page("/", "public/index.html")
    api("items", "title, body")
}
"#,
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    assert!(
        !handler.has_errors(),
        "page/api should auto-import: {:?}",
        handler.diagnostics()
    );
}

#[test]
fn explicit_use_still_works() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("explicit", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            "use std.keel.{page, api, run}\nfn main() { page(\"/\", \"public/index.html\") }\n",
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    assert!(
        !handler.has_errors(),
        "explicit use should still work: {:?}",
        handler.diagnostics()
    );
}

#[test]
fn duplicate_public_symbol() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("dup", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[
            ("src/main.bq", "fn main() {}\n"),
            ("src/a.bq", "pub fn clash() {}\n"),
            ("src/b.bq", "pub fn clash() {}\n"),
        ],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    assert!(
        handler
            .diagnostics()
            .iter()
            .any(|d| d.code.as_deref() == Some("E0404")),
        "expected E0404, got {:?}",
        handler.diagnostics()
    );
}

#[test]
fn imported_generic_emits_specialization() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("igen", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[
            (
                "src/main.bq",
                "use util.max\nfn main() { print(max(3, 9)) }\n",
            ),
            (
                "src/util.bq",
                r#"pub fn max[T](a: T, b: T) -> T {
    if a > b {
        return a
    }
    b
}
"#,
            ),
        ],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let world = analyze_project(&project, &handler, None);
    assert!(!world.had_errors, "{:?}", handler.diagnostics());
    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    opts.output = Some(project.root.join("igen.ll"));
    let out = compile_project(&project, &handler, &opts).expect("imported generic");
    assert!(
        out.llvm_ir.contains("max__i32") || out.llvm_ir.contains("@max__i32"),
        "expected monomorphized max__i32 in IR:\n{}",
        out.llvm_ir
    );
}

#[test]
fn nested_module_path() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("nest", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[
            (
                "src/main.bq",
                "use auth.jwt.token\nfn main() { token() }\n",
            ),
            ("src/auth/jwt.bq", "pub fn token() { }\n"),
        ],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    analyze_project(&project, &handler, None);
    assert!(
        !handler.has_errors(),
        "nested import failed: {:?}",
        handler.diagnostics()
    );
}

#[test]
fn stdlib_fs_lowers_and_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("stdfs", tmp.path()).unwrap();
    let data = project.root.join("payload.txt");
    std::fs::write(&data, "hello-stdlib").unwrap();
    let path = data.to_string_lossy().replace('\\', "/");
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            &format!(
                r#"
use std.fs.read

fn main() {{
    print(read("{path}"))
}}
"#
            ),
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    opts.output = Some(project.root.join("stdfs.ll"));
    let out = compile_project(&project, &handler, &opts).unwrap_or_else(|e| {
        panic!("stdlib fs IR: {e:?} diags={:?}", handler.diagnostics())
    });
    assert!(
        !handler.has_errors(),
        "stdlib fs analyze failed: {:?}",
        handler.diagnostics()
    );
    assert!(
        out.llvm_ir.contains("buraaq_file_read") && out.llvm_ir.contains("buraaq_fn_read"),
        "expected lowered std.fs.read:\n{}",
        out.llvm_ir
    );

    if !buraaq_codegen::clang_available() {
        return;
    }
    let mut run_opts = BuildOptions::default();
    run_opts.mir_opt = false;
    run_opts.output = Some(project.root.join("stdfs.exe"));
    let exe = compile_project(&project, &handler, &run_opts)
        .expect("stdlib fs link")
        .executable
        .expect("exe");
    let output = std::process::Command::new(&exe).output().expect("run stdfs");
    assert!(
        output.status.success(),
        "stdfs exit {}: {}\nIR:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stderr),
        out.llvm_ir
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "hello-stdlib"
    );
}

#[test]
fn stdlib_math_lowers() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("stdmath", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            r#"
use std.math.abs_int

fn main() {
    n = abs_int(4)
    print(n)
}
"#,
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    opts.output = Some(project.root.join("stdmath.ll"));
    let out = compile_project(&project, &handler, &opts).unwrap_or_else(|e| {
        panic!("math/json IR: {e:?} diags={:?}", handler.diagnostics())
    });
    assert!(
        !handler.has_errors(),
        "{:?}",
        handler.diagnostics()
    );
    assert!(
        out.llvm_ir.contains("buraaq_math_abs_i32"),
        "expected math extern:\n{}",
        out.llvm_ir
    );
}

#[test]
fn stdlib_http_file_url_runs() {
    if !buraaq_codegen::clang_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("stdhttp", tmp.path()).unwrap();
    let payload = project.root.join("httpbody.json");
    std::fs::write(&payload, "{\"name\":\"buraaq\"}").unwrap();
    let url = format!("file:///{}", payload.to_string_lossy().replace('\\', "/"));
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            &format!(
                r#"
use std.http.get

fn main() {{
    r = get("{url}")
    print(r.body)
}}
"#
            ),
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.output = Some(project.root.join("stdhttp.exe"));
    let exe = match compile_project(&project, &handler, &opts) {
        Ok(o) => o.executable.expect("exe"),
        Err(e) => panic!("http compile failed: {e:?} diags={:?}", handler.diagnostics()),
    };
    let output = std::process::Command::new(&exe).output().expect("run http");
    assert!(
        output.status.success(),
        "http exit {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("buraaq"),
        "stdout={}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn stdlib_http_https_get_attempts() {
    if !buraaq_codegen::clang_available() {
        return;
    }
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("stdhttps", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            r#"
use std.http.get

fn main() {
    r = get("https://example.com")
    print(r.body)
}
"#,
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.output = Some(project.root.join("stdhttps.exe"));
    let exe = match compile_project(&project, &handler, &opts) {
        Ok(o) => o.executable.expect("exe"),
        Err(e) => panic!("https compile failed: {e:?} diags={:?}", handler.diagnostics()),
    };
    let output = std::process::Command::new(&exe).output().expect("run https");
    assert!(
        output.status.success(),
        "https exit {}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    let body = String::from_utf8_lossy(&output.stdout);
    let ok = body.to_ascii_lowercase().contains("html")
        || body.contains("Example Domain")
        || body.contains("example");
    if ok {
        eprintln!("HTTPS GET succeeded ({} bytes)", body.len());
    } else {
        eprintln!(
            "HTTPS GET did not return a document (network/TLS/OpenSSL). body={}",
            body.chars().take(200).collect::<String>()
        );
    }
}

#[test]
fn spawn_emits_and_optionally_runs() {
    let tmp = tempfile::tempdir().unwrap();
    let project = create_new("spawndemo", tmp.path()).unwrap();
    write_pkg(
        &project.root,
        &[(
            "src/main.bq",
            r#"
fn work() {
    print(7)
}

fn main() {
    t = spawn {
        work()
    }
    t.wait()
}
"#,
        )],
    );
    let project = Project::discover(&project.root).unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    opts.output = Some(project.root.join("spawn.ll"));
    let out = compile_project(&project, &handler, &opts).expect("spawn IR");
    assert!(
        out.llvm_ir.contains("buraaq_task_submit") && out.llvm_ir.contains("__spawn_"),
        "spawn must lower to task submit:\n{}",
        out.llvm_ir
    );
}
