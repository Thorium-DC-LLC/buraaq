use std::path::{Path, PathBuf};

use buraaq_ast::{BinOp, Expr, Item, Literal, Program, Stmt, TestDef};
use buraaq_diagnostics::StandardHandler;
use buraaq_parser::Parser;
use buraaq_source::SourceFile;
use thiserror::Error;
use walkdir::WalkDir;

#[derive(Debug, Error)]
pub enum TestError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse failed: {0}")]
    Parse(String),
    #[error("test `{0}` failed: {1}")]
    Failed(String, String),
}

#[derive(Clone, Debug)]
pub struct TestCase {
    pub name: String,
    pub file: PathBuf,
}

#[derive(Clone, Debug, Default)]
pub struct TestResults {
    pub passed: usize,
    pub failed: usize,
    pub failures: Vec<(String, String)>,
}

pub fn discover_tests(root: &Path) -> Vec<TestCase> {
    let mut tests = Vec::new();
    for dir in [root.join("src"), root.join("tests")] {
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "bq") {
                if let Ok(file) = SourceFile::from_path(path) {
                    let handler = StandardHandler::new();
                    let result = Parser::parse(&file, &handler);
                    if !result.had_errors {
                        for item in &result.program.items {
                            if let Item::Test(t) = &item.node {
                                tests.push(TestCase {
                                    name: t.node.name.node.clone(),
                                    file: path.to_path_buf(),
                                });
                            }
                        }
                    }
                }
            }
        }
    }
    tests
}

pub fn run_tests(root: &Path) -> Result<TestResults, TestError> {
    let mut results = TestResults::default();
    for dir in [root.join("src"), root.join("tests")] {
        if !dir.exists() {
            continue;
        }
        for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "bq") {
                run_file_tests(path, &mut results)?;
            }
        }
    }
    Ok(results)
}

fn run_file_tests(path: &Path, results: &mut TestResults) -> Result<(), TestError> {
    let file = SourceFile::from_path(path)?;
    let handler = StandardHandler::new();
    let result = Parser::parse(&file, &handler);
    if result.had_errors {
        return Err(TestError::Parse(path.display().to_string()));
    }
    for item in &result.program.items {
        if let Item::Test(t) = &item.node {
            match run_test(t) {
                Ok(()) => results.passed += 1,
                Err(msg) => {
                    results.failed += 1;
                    results
                        .failures
                        .push((t.node.name.node.clone(), msg.clone()));
                }
            }
        }
    }
    Ok(())
}

fn run_test(test: &buraaq_source::Spanned<TestDef>) -> Result<(), String> {
    for stmt in &test.node.body.node.stmts {
        if let Stmt::Expect(e) = &stmt.node {
            eval_expect(&e.node)?;
        }
    }
    Ok(())
}

fn eval_expect(expect: &buraaq_ast::ExpectStmt) -> Result<(), String> {
    let val = eval_const_expr(&expect.expr)?;
    if val == 0 {
        return Err("expect expression evaluated to false".into());
    }
    Ok(())
}

fn eval_const_expr(expr: &buraaq_source::Spanned<buraaq_ast::ExprNode>) -> Result<i128, String> {
    match expr.node.as_ref() {
        Expr::Literal(l) => match &l.node {
            Literal::Int(n) => Ok(n.node),
            _ => Err("expect supports integer literals only in v1".into()),
        },
        Expr::Binary(b) => {
            let l = eval_const_expr(&b.node.left)?;
            let r = eval_const_expr(&b.node.right)?;
            match b.node.op.node {
                BinOp::Add => Ok(l + r),
                BinOp::Sub => Ok(l - r),
                BinOp::Mul => Ok(l * r),
                BinOp::Div => Ok(if r == 0 { 0 } else { l / r }),
                BinOp::Eq => Ok(if l == r { 1 } else { 0 }),
                BinOp::NotEq => Ok(if l != r { 1 } else { 0 }),
                _ => Err("unsupported operator in expect".into()),
            }
        },
        _ => Err("expect supports literal arithmetic only in v1".into()),
    }
}

pub fn discover_benches(root: &Path) -> Vec<TestCase> {
    let mut benches = Vec::new();
    let dir = root.join("benches");
    if !dir.exists() {
        return benches;
    }
    for entry in WalkDir::new(&dir).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "bq") {
            if let Ok(file) = SourceFile::from_path(path) {
                let handler = StandardHandler::new();
                let result = Parser::parse(&file, &handler);
                if !result.had_errors {
                    for item in &result.program.items {
                        if let Item::Bench(b) = &item.node {
                            benches.push(TestCase {
                                name: b.node.name.node.clone(),
                                file: path.to_path_buf(),
                            });
                        }
                    }
                }
            }
        }
    }
    benches
}
