//! Compile-pass / compile-fail harness for `tests/ui/**/*.bq`.

use std::fs;
use std::path::PathBuf;

use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_frontend::Frontend;

fn ui_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/ui")
}

#[test]
fn ui_corpus() {
    let root = ui_root();
    if !root.exists() {
        return;
    }
    let mut cases = 0;
    for entry in walkdir::WalkDir::new(&root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("bq") {
            continue;
        }
        cases += 1;
        let expect_fail = path
            .file_stem()
            .and_then(|s| s.to_str())
            .is_some_and(|s| s.ends_with("_fail"));
        let handler = StandardHandler::new();
        let fe = Frontend::compile_file(path, &handler);
        let stderr_path = path.with_extension("stderr");
        if stderr_path.exists() || expect_fail {
            assert!(
                fe.had_errors || handler.has_errors(),
                "{} should fail",
                path.display()
            );
            if stderr_path.exists() {
                let expected = fs::read_to_string(&stderr_path).unwrap();
                let codes: Vec<String> = handler
                    .diagnostics()
                    .iter()
                    .filter_map(|d| d.code.clone())
                    .collect();
                for code in expected.lines().filter(|l| !l.is_empty() && !l.starts_with('#')) {
                    assert!(
                        codes.iter().any(|c| c == code),
                        "{}: expected {code}, got {codes:?}",
                        path.display()
                    );
                }
            }
        } else {
            assert!(
                !fe.had_errors && !handler.has_errors(),
                "{} should pass: {:?}",
                path.display(),
                handler.diagnostics()
            );
        }
    }
    assert!(cases > 0, "expected UI fixtures under tests/ui");
}
