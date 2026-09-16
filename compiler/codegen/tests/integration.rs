use std::path::PathBuf;
use std::process::Command;

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_to_executable, BuildOptions};

fn program(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/programs")
        .join(name)
}

fn clang_available() -> bool {
    buraaq_codegen::clang_available()
}

#[test]
fn emits_llvm_ir_for_arithmetic() {
    let path = program("arithmetic.bq");
    let handler = StandardHandler::new();
    let fe = buraaq_frontend::Frontend::compile_file(&path, &handler);
    assert!(!fe.had_errors, "frontend should succeed");
    let mut opts = BuildOptions::default();
    opts.emit_ir = true;
    let out = buraaq_driver::compile_to_ir(&fe.ast, &fe.source, &opts).expect("lower+emit");
    assert!(out.llvm_ir.contains("define i32 @main"));
    assert!(out.llvm_ir.contains("buraaq_fn_add"));
    assert!(out.llvm_ir.contains("buraaq_print_str"));
}

#[test]
fn runs_arithmetic_program() {
    if !clang_available() {
        eprintln!("skipping: clang not on PATH");
        return;
    }
    let path = program("arithmetic.bq");
    let handler = StandardHandler::new();
    let out = compile_to_executable(&path, &handler, &BuildOptions::default())
        .expect("compile");
    let exe = out.executable.expect("exe");
    let output = Command::new(&exe)
        .output()
        .expect("run exe");
    assert!(output.status.success(), "program failed");
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "5");
}

#[test]
fn release_build_enables_o2() {
    if !clang_available() {
        return;
    }
    let path = program("arithmetic.bq");
    let handler = StandardHandler::new();
    let mut opts = BuildOptions::default();
    opts.release = true;
    let out = compile_to_executable(&path, &handler, &opts).expect("release build");
    assert!(out.executable.is_some());
}
