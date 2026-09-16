//! M5 names + M6 MIR subset + M7 LLVM + M8 guest clang on `golden/sample.bq`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use buraaq_ast::{BinOp, Expr, Item, Literal, Stmt};
use buraaq_codegen::{clang_available, link_executable, LinkOptions};
use buraaq_codegen::{OptLevel, TargetTriple};
use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, BuildOptions};
use buraaq_parser::Parser;
use buraaq_pkg::Project;
use buraaq_source::SourceFile;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn compile_boot(out_name: &str) -> PathBuf {
    assert!(
        clang_available(),
        "clang required for bootstrap M5–M8"
    );
    let root = repo_root();
    let project = Project::discover(&root.join("compiler-buraaq")).expect("compiler-buraaq project");
    let tmp = tempfile::tempdir().unwrap();
    // Keep the tempdir alive by leaking; tests need the exe after compile_boot returns.
    let tmp = Box::leak(Box::new(tmp));
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
        .expect("spawn bootstrap");
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
        "bootstrap failed args={args:?}: {err}\n{out}"
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

fn classify_name(name: &str, params: &HashSet<String>, out: &mut Vec<String>) {
    if name == "print" || name == "println" {
        out.push("builtin".into());
        out.push(name.into());
    } else if params.contains(name) {
        out.push("local".into());
        out.push(name.into());
    } else {
        out.push("fnref".into());
        out.push(name.into());
    }
}

fn dump_names_expr(out: &mut Vec<String>, expr: &Expr, params: &HashSet<String>) {
    match expr {
        Expr::Literal(_) => {}
        Expr::Ident(name) => classify_name(&name.node, params, out),
        Expr::Binary(b) => {
            dump_names_expr(out, &b.node.left.node, params);
            dump_names_expr(out, &b.node.right.node, params);
        }
        Expr::Call(c) => {
            if let Expr::Ident(name) = c.node.callee.node.as_ref() {
                classify_name(&name.node, params, out);
            }
            for a in &c.node.args {
                dump_names_expr(out, &a.node, params);
            }
        }
        Expr::Struct(s) => {
            let name = s
                .node
                .path
                .node
                .segments
                .last()
                .map(|seg| seg.node.clone())
                .unwrap_or_default();
            classify_name(&name, params, out);
            if let buraaq_ast::StructFill::Tuple(args) = &s.node.fill {
                for a in args {
                    dump_names_expr(out, &a.node, params);
                }
            }
        }
        Expr::Paren(p) => dump_names_expr(out, &p.node, params),
        Expr::If(i) => {
            dump_names_expr(out, &i.node.cond.node, params);
            dump_names_block(out, &i.node.then_block.node, params);
        }
        _ => {}
    }
}

fn dump_names_block(out: &mut Vec<String>, block: &buraaq_ast::Block, params: &HashSet<String>) {
    for stmt in &block.stmts {
        match &stmt.node {
            Stmt::If(i) => {
                dump_names_expr(out, &i.node.cond.node, params);
                dump_names_block(out, &i.node.then_block.node, params);
            }
            Stmt::Return(r) => {
                if let Some(v) = &r.node.value {
                    dump_names_expr(out, &v.node, params);
                }
            }
            Stmt::Expr(e) => dump_names_expr(out, &e.node, params),
            _ => {}
        }
    }
    if let Some(tail) = &block.tail {
        dump_names_expr(out, &tail.node, params);
    }
}

fn rust_names(src: &SourceFile) -> Vec<String> {
    let handler = StandardHandler::new();
    let parsed = Parser::parse(src, &handler);
    assert!(!parsed.had_errors, "host parser errors: {:?}", handler.diagnostics());
    let mut out = Vec::new();
    for item in &parsed.program.items {
        let Item::Function(f) = &item.node else {
            continue;
        };
        out.push("fn".into());
        out.push(f.node.name.node.clone());
        let mut params = HashSet::new();
        for p in &f.node.params {
            out.push("param".into());
            out.push(p.node.name.node.clone());
            params.insert(p.node.name.node.clone());
        }
        dump_names_block(&mut out, &f.node.body.node, &params);
    }
    out
}

fn dump_mir_operand(out: &mut Vec<String>, expr: &Expr) {
    match expr {
        Expr::Literal(l) => {
            if let Literal::Int(n) = &l.node {
                out.push("const".into());
                out.push(n.node.to_string());
            }
        }
        Expr::Ident(name) => out.push(name.node.clone()),
        Expr::Call(c) => {
            out.push("call".into());
            if let Expr::Ident(name) = c.node.callee.node.as_ref() {
                out.push(name.node.clone());
            } else {
                out.push("_".into());
            }
            for a in &c.node.args {
                dump_mir_operand(out, &a.node);
            }
        }
        Expr::Struct(s) => {
            let name = s
                .node
                .path
                .node
                .segments
                .last()
                .map(|seg| seg.node.clone())
                .unwrap_or_default();
            out.push("call".into());
            out.push(name);
            if let buraaq_ast::StructFill::Tuple(args) = &s.node.fill {
                for a in args {
                    dump_mir_operand(out, &a.node);
                }
            }
        }
        Expr::Paren(p) => dump_mir_operand(out, &p.node),
        Expr::Binary(b) => {
            dump_mir_operand(out, &b.node.left.node);
            let op = match b.node.op.node {
                BinOp::Add => "add",
                BinOp::Sub => "sub",
                BinOp::Gt => "sgt",
                BinOp::Ge => "sge",
                BinOp::Lt => "slt",
                BinOp::Le => "sle",
                _ => "bin",
            };
            out.push(op.into());
            dump_mir_operand(out, &b.node.right.node);
        }
        _ => {}
    }
}

fn dump_mir_block(out: &mut Vec<String>, block: &buraaq_ast::Block) {
    for stmt in &block.stmts {
        match &stmt.node {
            Stmt::If(i) => {
                out.push("icmp".into());
                dump_mir_operand(out, &i.node.cond.node);
                dump_mir_block(out, &i.node.then_block.node);
            }
            Stmt::Return(r) => {
                out.push("ret".into());
                if let Some(v) = &r.node.value {
                    dump_mir_operand(out, &v.node);
                }
            }
            Stmt::Expr(e) => dump_mir_operand(out, &e.node),
            _ => {}
        }
    }
    if let Some(tail) = &block.tail {
        dump_mir_operand(out, &tail.node);
    }
}

fn rust_mir(src: &SourceFile) -> Vec<String> {
    let handler = StandardHandler::new();
    let parsed = Parser::parse(src, &handler);
    assert!(!parsed.had_errors, "host parser errors: {:?}", handler.diagnostics());
    let mut out = Vec::new();
    for item in &parsed.program.items {
        let Item::Function(f) = &item.node else {
            continue;
        };
        out.push("fn".into());
        out.push(f.node.name.node.clone());
        for p in &f.node.params {
            out.push("param".into());
            out.push(p.node.name.node.clone());
        }
        dump_mir_block(&mut out, &f.node.body.node);
        out.push("ret".into());
    }
    out
}

fn golden() -> PathBuf {
    repo_root().join("compiler-buraaq/golden/sample.bq")
}

#[test]
fn bootstrap_m5_names_match_rust() {
    let exe = compile_boot("buraaq_names.exe");
    let fixture = golden();
    let bq = lines(&run_boot(&exe, &["names", fixture.to_str().unwrap()], 8));
    let file = SourceFile::from_path(&fixture).expect("read fixture");
    let rust = rust_names(&file);
    assert_eq!(
        bq, rust,
        "M5 names must match Rust AST walk\nBuraaq={bq:?}\nRust={rust:?}"
    );
}

#[test]
fn bootstrap_m6_mir_subset_matches_rust() {
    let exe = compile_boot("buraaq_mir.exe");
    let fixture = golden();
    let bq = lines(&run_boot(&exe, &["mir", fixture.to_str().unwrap()], 8));
    let file = SourceFile::from_path(&fixture).expect("read fixture");
    let rust = rust_mir(&file);
    assert_eq!(
        bq, rust,
        "M6 MIR subset must match Rust AST walk\nBuraaq={bq:?}\nRust={rust:?}"
    );
}

#[test]
fn bootstrap_m7_llvm_links_and_prints_five() {
    let exe = compile_boot("buraaq_ll.exe");
    let fixture = golden();
    let ir = run_boot(&exe, &["llvm", fixture.to_str().unwrap()], 8);
    assert!(
        ir.contains("define i32 @add"),
        "guest LLVM missing @add:\n{ir}"
    );
    assert!(
        ir.contains("define i32 @main"),
        "guest LLVM missing @main:\n{ir}"
    );
    assert!(
        ir.contains("buraaq_print_i32"),
        "guest LLVM missing print:\n{ir}"
    );

    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("golden_m7.exe");
    let rt = repo_root().join("compiler/runtime/buraaq_rt.c");
    let std = repo_root().join("stdlib/runtime/buraaq_std.c");
    let opts = LinkOptions {
        target: TargetTriple::detect_host(),
        opt: OptLevel::Debug,
        output: out.clone(),
        emit_asm: false,
        emit_ir_only: false,
        extra_libs: vec![],
    };
    link_executable(&ir, &[rt, std], &opts).expect("clang-link guest LLVM");
    let run = Command::new(&out).output().expect("run guest-linked golden");
    assert!(
        run.status.success(),
        "guest-linked golden failed: {}",
        String::from_utf8_lossy(&run.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "5",
        "M7 golden must print 5 (print(add(2,3)))"
    );
}

#[test]
fn bootstrap_m8_guest_build_prints_five() {
    let exe = compile_boot("buraaq_build.exe");
    let fixture = golden();
    let tmp = tempfile::tempdir().unwrap();
    let out = tmp.path().join("golden_m8.exe");
    let rt = repo_root().join("compiler/runtime/buraaq_rt.c");
    let clang = "clang";
    run_boot(
        &exe,
        &[
            "build",
            fixture.to_str().unwrap(),
            out.to_str().unwrap(),
            rt.to_str().unwrap(),
            clang,
        ],
        30,
    );
    assert!(
        out.exists(),
        "M8 guest build did not produce {}",
        out.display()
    );
    let run = Command::new(&out).output().expect("run M8 golden");
    assert!(
        run.status.success(),
        "M8 golden failed: {}\n{}",
        String::from_utf8_lossy(&run.stderr),
        String::from_utf8_lossy(&run.stdout)
    );
    assert_eq!(
        String::from_utf8_lossy(&run.stdout).trim(),
        "5",
        "M8 guest-built golden must print 5"
    );
}
