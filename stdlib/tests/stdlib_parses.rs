use std::path::PathBuf;

use buraaq_diagnostics::StandardHandler;
use buraaq_parser::Parser;
use buraaq_source::SourceFile;
use walkdir::WalkDir;

fn stdlib_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn all_stdlib_modules_parse() {
    let root = stdlib_root();
    let scan_dirs = [root.join("src"), root.join("examples")];
    let mut files = Vec::new();

    for dir in &scan_dirs {
        for entry in WalkDir::new(dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "bq") {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();

    assert!(
        files.len() >= 25,
        "expected stdlib modules + examples; found {} .bq files",
        files.len()
    );

    let mut failures = Vec::new();
    for path in &files {
        let handler = StandardHandler::new();
        let source = SourceFile::from_path(path).expect("read stdlib source");
        let result = Parser::parse(&source, &handler);
        if result.had_errors {
            failures.push(
                path.strip_prefix(stdlib_root())
                    .unwrap_or(path)
                    .display()
                    .to_string(),
            );
        }
    }

    assert!(
        failures.is_empty(),
        "stdlib modules failed to parse:\n  {}",
        failures.join("\n  ")
    );
}
