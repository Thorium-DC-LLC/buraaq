//! Gate C — Buraaq lexer golden token-kind parity vs Rust `buraaq_lexer`.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, BuildOptions};
use buraaq_lexer::{Lexer, TokenKind};
use buraaq_pkg::Project;
use buraaq_source::SourceFile;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

#[test]
fn bootstrap_lexer_matches_rust_token_kinds() {
    assert!(
        buraaq_codegen::clang_available(),
        "clang required for bootstrap M3 native lexer"
    );
    let root = repo_root();
    let project = Project::discover(&root.join("compiler-buraaq")).expect("compiler-buraaq project");
    let fixture = root.join("compiler-buraaq/golden/sample.bq");
    assert!(fixture.exists(), "missing {}", fixture.display());

    let tmp = tempfile::tempdir().unwrap();
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.release = false;
    opts.mir_opt = false;
    opts.output = Some(tmp.path().join("buraaq_lex.exe"));
    let exe = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("bootstrap lexer compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe");

    eprintln!("compiled bootstrap lexer → {}", exe.display());
    let mut child = Command::new(&exe)
        .arg(&fixture)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bootstrap lexer");
    let started = Instant::now();
    let run = loop {
        match child.try_wait() {
            Ok(Some(_)) => break child.wait_with_output().expect("collect lexer output"),
            Ok(None) if started.elapsed() > Duration::from_secs(8) => {
                let _ = child.kill();
                panic!("bootstrap lexer exceeded 8s — likely an infinite loop");
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("wait lexer: {e}"),
        }
    };
    assert!(
        run.status.success(),
        "bootstrap lexer failed: {}\n{}",
        String::from_utf8_lossy(&run.stderr),
        String::from_utf8_lossy(&run.stdout)
    );
    let bq: Vec<String> = String::from_utf8_lossy(&run.stdout)
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect();

    let file = SourceFile::from_path(&fixture).expect("read fixture");
    let rust: Vec<String> = Lexer::new(&file)
        .tokenize()
        .into_iter()
        .filter(|t| !matches!(t.kind, TokenKind::Eof))
        .map(|t| t.kind.golden_name().to_string())
        .collect();

    assert_eq!(
        bq, rust,
        "Buraaq lexer kinds must match Rust buraaq_lexer\nBuraaq={bq:?}\nRust={rust:?}"
    );
}
