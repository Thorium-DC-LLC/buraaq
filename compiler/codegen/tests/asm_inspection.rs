use std::path::PathBuf;

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_to_executable, BuildOptions};

fn clang_available() -> bool {
    buraaq_codegen::clang_available()
}

#[test]
fn release_asm_contains_add_instruction() {
    if !clang_available() {
        eprintln!("skipping: clang not on PATH");
        return;
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/programs/arithmetic.bq");
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.release = true;
    opts.emit_asm = true;
    let out = compile_to_executable(&path, &handler, &opts).expect("emit asm");
    let asm_path = out.executable.expect("asm path");
    let asm = std::fs::read_to_string(&asm_path).expect("read asm");
    let lowered = asm.to_lowercase();
    assert!(
        lowered.contains("add") || lowered.contains("lea"),
        "expected arithmetic in assembly, got:\n{asm}"
    );
}
