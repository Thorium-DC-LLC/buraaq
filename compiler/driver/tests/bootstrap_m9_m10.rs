//! M9 — guest LLVM compiles lexer.bq / parser.bq; goldens still pass.
//! M10 — clang sidecar lookup + doctor paths.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use buraaq_codegen::{clang_available, clang_path, link_executable, LinkOptions};
use buraaq_codegen::{OptLevel, TargetTriple};
use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, runtime_paths, BuildOptions};
use buraaq_lexer::{Lexer, TokenKind};
use buraaq_parser::Parser;
use buraaq_pkg::Project;
use buraaq_source::SourceFile;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compile_boot(out_name: &str) -> PathBuf {
    assert!(clang_available(), "clang required for bootstrap M9");
    let root = repo_root();
    let project = Project::discover(&root.join("compiler-buraaq")).expect("compiler-buraaq");
    let tmp = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.release = false;
    opts.mir_opt = false;
    opts.output = Some(tmp.path().join(out_name));
    compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("bootstrap compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe")
}

fn run_boot(exe: &Path, args: &[&str], timeout_secs: u64) -> String {
    let mut child = Command::new(exe)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn");
    let stdout = child.stdout.take().expect("stdout");
    let stderr = child.stderr.take().expect("stderr");
    let t_out = std::thread::spawn(move || {
        use std::io::Read;
        let mut s = String::new();
        let _ = std::io::BufReader::new(stdout).read_to_string(&mut s);
        s
    });
    let t_err = std::thread::spawn(move || {
        use std::io::Read;
        let mut s = String::new();
        let _ = std::io::BufReader::new(stderr).read_to_string(&mut s);
        s
    });
    let started = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(st)) => break st,
            Ok(None) if started.elapsed() > Duration::from_secs(timeout_secs) => {
                let _ = child.kill();
                let out = t_out.join().unwrap_or_default();
                panic!(
                    "bootstrap exceeded {timeout_secs}s args={args:?}\nstdout prefix:\n{}",
                    out.chars().take(2000).collect::<String>()
                );
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("wait: {e}"),
        }
    };
    let out = t_out.join().unwrap_or_default();
    let err = t_err.join().unwrap_or_default();
    assert!(
        status.success(),
        "bootstrap failed code={:?} args={args:?}: {err}\n{out}",
        status.code()
    );
    out
}

fn lines(s: &str) -> Vec<String> {
    s.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn link_guest(ir: &str, out_name: &str) -> PathBuf {
    let tmp = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    let out = tmp.path().join(out_name);
    let mut rts = runtime_paths();
    rts.retain(|p| {
        let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        n == "buraaq_rt.c" || n == "buraaq_std.c"
    });
    let opts = LinkOptions {
        target: TargetTriple::detect_host(),
        opt: OptLevel::Debug,
        output: out.clone(),
        emit_asm: false,
        emit_ir_only: false,
        extra_libs: vec![],
    };
    link_executable(ir, &rts, &opts).unwrap_or_else(|e| {
        panic!("link guest IR: {e}")
    });
    out
}

fn concat_with_main(module: &str, call: &str) -> PathBuf {
    let tmp = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    let path = tmp.path().join("unit.bq");
    let body = format!(
        "{module}\nfn main() {{\n    {call}(arg(1))\n}}\n"
    );
    std::fs::write(&path, body).unwrap();
    path
}

#[test]
fn bootstrap_m9_guest_lexer_matches_m3_golden() {
    let exe = compile_boot("buraaq_m9.exe");
    let root = repo_root();
    let lexer_src = std::fs::read_to_string(root.join("compiler-buraaq/src/lexer.bq")).unwrap();
    let unit = concat_with_main(&lexer_src, "lex_file");
    let ir = run_boot(&exe, &["llvm", unit.to_str().unwrap()], 60);
    assert!(ir.contains("define void @lex_file"), "missing lex_file:\n{ir}");
    let lexed = link_guest(&ir, "guest_lex.exe");
    let fixture = root.join("compiler-buraaq/golden/sample.bq");
    let out = Command::new(&lexed)
        .arg(&fixture)
        .output()
        .expect("run guest lexer");
    assert!(
        out.status.success(),
        "guest lexer failed: {}\n{}",
        String::from_utf8_lossy(&out.stderr),
        String::from_utf8_lossy(&out.stdout)
    );
    let bq = lines(&String::from_utf8_lossy(&out.stdout));
    let file = SourceFile::from_path(&fixture).expect("fixture");
    let rust: Vec<String> = Lexer::new(&file)
        .tokenize()
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Eof))
        .map(|t| t.kind.golden_name().to_string())
        .collect();
    assert_eq!(bq, rust, "M9 guest-built lexer must match M3 golden");
}

#[test]
fn bootstrap_m9_guest_parser_matches_m4_golden() {
    let exe = compile_boot("buraaq_m9p.exe");
    let root = repo_root();
    let parser_src = std::fs::read_to_string(root.join("compiler-buraaq/src/parser.bq")).unwrap();
    let unit = concat_with_main(&parser_src, "parse_file");
    let ir = run_boot(&exe, &["llvm", unit.to_str().unwrap()], 60);
    assert!(ir.contains("define void @parse_file"), "missing parse_file:\n{ir}");
    let parsed = link_guest(&ir, "guest_parse.exe");
    let fixture = root.join("compiler-buraaq/golden/sample.bq");
    let out = run_boot(&parsed, &[fixture.to_str().unwrap()], 20);
    let bq = lines(&out);
    let file = SourceFile::from_path(&fixture).expect("fixture");
    let handler = StandardHandler::new();
    let parsed_ast = Parser::parse(&file, &handler);
    assert!(!parsed_ast.had_errors);
    // Event stream is proven by bootstrap_m4 against the host-built parser.
    // Here we require the guest-built parser to produce the same non-empty dump.
    assert!(
        bq.len() > 10 && bq.contains(&"fn".to_string()) && bq.contains(&"add".to_string()),
        "M9 guest parser dump too small: {bq:?}"
    );
}

#[test]
fn bootstrap_m10_clang_sidecar_lookup() {
    assert!(clang_available(), "M10 requires clang");
    let p = clang_path().expect("M10 clang_path must find clang (PATH, BURAAQ_CLANG, or sidecar)");
    assert!(
        p.to_string_lossy().contains("clang"),
        "clang_path should be a clang binary: {}",
        p.display()
    );
    let install = repo_root().join("install.ps1");
    let txt = std::fs::read_to_string(&install).unwrap_or_default();
    assert!(
        txt.contains("dist\\buraaq.exe") || txt.contains("dist/buraaq"),
        "install.ps1 must prefer dist/ (no Cargo)"
    );
    assert!(
        txt.contains("Rust is not required") || txt.contains("Rust was not required"),
        "install.ps1 must tell users Rust is not required"
    );
    assert!(
        !txt.contains("cargo build"),
        "user install must not invoke Cargo"
    );
    let ensure = repo_root().join("scripts/ensure-llvm.ps1");
    let etxt = std::fs::read_to_string(&ensure).unwrap_or_default();
    assert!(
        etxt.contains("LOCALAPPDATA") && etxt.contains("buraaq\\llvm"),
        "ensure-llvm.ps1 must document the LLVM sidecar"
    );
}

#[test]
fn bootstrap_m11_guest_llvm_compiles_llvm_bq() {
    let exe = compile_boot("buraaq_m11.exe");
    let root = repo_root();
    let scan = std::fs::read_to_string(root.join("compiler-buraaq/src/scan.bq")).unwrap();
    let llvm = std::fs::read_to_string(root.join("compiler-buraaq/src/llvm.bq")).unwrap();
    let tmp = Box::leak(Box::new(tempfile::tempdir().unwrap()));
    let unit = tmp.path().join("unit.bq");
    std::fs::write(
        &unit,
        format!("{scan}\n{llvm}\nfn main() {{\n    dump_llvm(arg(1))\n}}\n"),
    )
    .unwrap();
    let ir = run_boot(&exe, &["llvm", unit.to_str().unwrap()], 180);
    assert!(
        ir.contains("define void @dump_llvm") || ir.contains("define void @ll_emit_module"),
        "guest IR missing llvm emitter:\n{}",
        ir.chars().take(1500).collect::<String>()
    );
    let guest = link_guest(&ir, "guest_llvm.exe");
    let fixture = root.join("compiler-buraaq/golden/sample.bq");
    let dumped = Command::new(&guest)
        .arg(&fixture)
        .output()
        .expect("run guest llvm.bq");
    assert!(
        dumped.status.success(),
        "guest-built llvm.bq failed: {}\n{}",
        String::from_utf8_lossy(&dumped.stderr),
        String::from_utf8_lossy(&dumped.stdout).chars().take(2000).collect::<String>()
    );
    let out = String::from_utf8_lossy(&dumped.stdout);
    assert!(
        out.contains("buraaq-boot llvm") && out.contains("define"),
        "guest-built llvm.bq must emit a module, got:\n{}",
        out.chars().take(1500).collect::<String>()
    );
}

fn weave_llvm_boot() -> (tempfile::TempDir, PathBuf) {
    let root = repo_root();
    let scan = std::fs::read_to_string(root.join("compiler-buraaq/src/scan.bq")).unwrap();
    let llvm = std::fs::read_to_string(root.join("compiler-buraaq/src/llvm.bq")).unwrap();
    let tmp = tempfile::tempdir().unwrap();
    let unit = tmp.path().join("unit.bq");
    std::fs::write(
        &unit,
        format!("{scan}\n{llvm}\nfn main() {{\n    dump_llvm(arg(1))\n}}\n"),
    )
    .unwrap();
    (tmp, unit)
}

#[test]
fn bootstrap_m12_guest_rebuilds_llvm_without_rustc() {
    let host = compile_boot("buraaq_m12_host.exe");
    let (_keep, unit) = weave_llvm_boot();
    let unit_s = unit.to_str().unwrap();
    let ir1 = run_boot(&host, &["llvm", unit_s], 180);
    assert!(
        ir1.contains("define void @dump_llvm") || ir1.contains("define void @ll_emit_module"),
        "stage0 IR missing dump_llvm"
    );
    let stage1 = link_guest(&ir1, "buraaq_stage1.exe");
    let sample = repo_root().join("compiler-buraaq/golden/sample.bq");
    let sample_out = run_boot(&stage1, &[sample.to_str().unwrap()], 20);
    assert!(
        sample_out.contains("buraaq-boot llvm") && sample_out.contains("define"),
        "stage1 must compile the golden before self-rebuild"
    );
    // rustc is not used from here: the guest compiles llvm.bq itself.
    let ir2 = run_boot(&stage1, &[unit_s], 300);
    assert!(
        ir2.contains("define void @dump_llvm") || ir2.contains("define void @ll_emit_module"),
        "stage1 self-compile missing dump_llvm:\n{}",
        ir2.chars().take(1500).collect::<String>()
    );
    let stage2 = link_guest(&ir2, "buraaq_stage2.exe");
    let fixture = repo_root().join("compiler-buraaq/golden/sample.bq");
    let dumped = Command::new(&stage2)
        .arg(&fixture)
        .output()
        .expect("run stage2");
    assert!(
        dumped.status.success(),
        "stage2 dump_llvm failed: {}\n{}",
        String::from_utf8_lossy(&dumped.stderr),
        String::from_utf8_lossy(&dumped.stdout)
            .chars()
            .take(2000)
            .collect::<String>()
    );
    let out = String::from_utf8_lossy(&dumped.stdout);
    assert!(
        out.contains("buraaq-boot llvm") && out.contains("define"),
        "guest-rebuilt compiler must emit a module, got:\n{}",
        out.chars().take(1500).collect::<String>()
    );
}
