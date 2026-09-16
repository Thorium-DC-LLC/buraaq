//! M4 — Buraaq parser AST-event parity vs Rust `buraaq_parser` (golden subset).

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use buraaq_ast::{BinOp, Expr, Item, Literal, Stmt, Type};
use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_project, BuildOptions};
use buraaq_parser::Parser;
use buraaq_pkg::Project;
use buraaq_source::SourceFile;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn type_name(ty: &Type) -> String {
    match ty {
        Type::Named(n) => n
            .node
            .path
            .node
            .segments
            .last()
            .map(|s| s.node.clone())
            .unwrap_or_default(),
        _ => "unknown".into(),
    }
}

fn bin_name(op: BinOp) -> &'static str {
    match op {
        BinOp::Add => "Plus",
        BinOp::Sub => "Minus",
        BinOp::Gt => "Gt",
        BinOp::Ge => "Ge",
        BinOp::Lt => "Lt",
        BinOp::Le => "Le",
        BinOp::Eq => "EqEq",
        _ => "Bin",
    }
}

fn dump_expr(out: &mut Vec<String>, expr: &Expr) {
    match expr {
        Expr::Literal(l) => match &l.node {
            Literal::Int(n) => {
                out.push("int".into());
                out.push(n.node.to_string());
            }
            _ => out.push("lit".into()),
        },
        Expr::Ident(name) => {
            out.push("ident".into());
            out.push(name.node.clone());
        }
        Expr::Binary(b) => {
            dump_expr(out, &b.node.left.node);
            out.push("bin".into());
            out.push(bin_name(b.node.op.node).into());
            dump_expr(out, &b.node.right.node);
        }
        Expr::Call(c) => {
            if let Expr::Ident(name) = c.node.callee.node.as_ref() {
                out.push("app".into());
                out.push(name.node.clone());
            } else {
                out.push("app".into());
                out.push("_".into());
                dump_expr(out, &c.node.callee.node);
            }
            for a in &c.node.args {
                dump_expr(out, &a.node);
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
            out.push("app".into());
            out.push(name);
            if let buraaq_ast::StructFill::Tuple(args) = &s.node.fill {
                for a in args {
                    dump_expr(out, &a.node);
                }
            }
        }
        Expr::Paren(p) => dump_expr(out, &p.node),
        Expr::If(i) => {
            out.push("if".into());
            dump_expr(out, &i.node.cond.node);
            dump_block(out, &i.node.then_block.node);
        }
        _ => out.push("expr".into()),
    }
}

fn dump_block(out: &mut Vec<String>, block: &buraaq_ast::Block) {
    for stmt in &block.stmts {
        match &stmt.node {
            Stmt::If(i) => {
                out.push("if".into());
                dump_expr(out, &i.node.cond.node);
                dump_block(out, &i.node.then_block.node);
            }
            Stmt::Return(r) => {
                out.push("return".into());
                if let Some(v) = &r.node.value {
                    dump_expr(out, &v.node);
                }
            }
            Stmt::Expr(e) => dump_expr(out, &e.node),
            _ => {}
        }
    }
    if let Some(tail) = &block.tail {
        dump_expr(out, &tail.node);
    }
}

fn rust_events(src: &SourceFile) -> Vec<String> {
    let handler = StandardHandler::new();
    let parsed = Parser::parse(src, &handler);
    assert!(
        !parsed.had_errors,
        "host parser errors: {:?}",
        handler.diagnostics()
    );
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
            out.push(type_name(&p.node.ty.node));
        }
        if let Some(ret) = &f.node.ret {
            out.push("ret".into());
            out.push(type_name(&ret.node));
        }
        dump_block(&mut out, &f.node.body.node);
    }
    out
}

#[test]
fn bootstrap_parser_matches_rust_ast_events() {
    assert!(
        buraaq_codegen::clang_available(),
        "clang required for bootstrap M4 native parser"
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
    opts.output = Some(tmp.path().join("buraaq_parse.exe"));
    let exe = compile_project(&project, &handler, &opts)
        .unwrap_or_else(|e| panic!("bootstrap parser compile: {e:?} {:?}", handler.diagnostics()))
        .executable
        .expect("exe");

    let mut child = Command::new(&exe)
        .arg("parse")
        .arg(&fixture)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn bootstrap parser");
    let started = Instant::now();
    let run = loop {
        match child.try_wait() {
            Ok(Some(_)) => break child.wait_with_output().expect("collect parser output"),
            Ok(None) if started.elapsed() > Duration::from_secs(8) => {
                let _ = child.kill();
                panic!("bootstrap parser exceeded 8s");
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => panic!("wait parser: {e}"),
        }
    };
    assert!(
        run.status.success(),
        "bootstrap parser failed: {}\n{}",
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
    let rust = rust_events(&file);
    assert_eq!(
        bq, rust,
        "Buraaq parser events must match Rust AST walk\nBuraaq={bq:?}\nRust={rust:?}"
    );
}
