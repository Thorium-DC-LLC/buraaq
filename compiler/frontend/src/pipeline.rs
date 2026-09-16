use std::path::Path;

use buraaq_ast::Program;
use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_lexer::{Lexer, Token};
use buraaq_parser::{ParseResult, Parser};
use buraaq_source::{SourceFile, SourceMap};

use crate::semantic;

pub struct CompileResult {
    pub source: SourceFile,
    pub tokens: Vec<Token>,
    pub ast: Program,
    pub had_errors: bool,
    pub semantic_us: u128,
}

pub struct Frontend;

impl Frontend {
    pub fn compile_file(path: &Path, handler: &dyn DiagnosticHandler) -> CompileResult {
        let source = SourceFile::from_path(path).unwrap_or_else(|e| {
            panic!("failed to read {}: {e}", path.display());
        });
        Self::compile_source(source, handler)
    }

    pub fn compile_source(source: SourceFile, handler: &dyn DiagnosticHandler) -> CompileResult {
        let tokens = Lexer::new(&source).with_diagnostics(handler).tokenize();
        let ParseResult { program, had_errors } = Parser::parse(&source, handler);

        let sem = semantic::analyze_with_stats(&program, &source, handler);
        let had_errors = had_errors || sem.errors > 0;

        CompileResult {
            source,
            tokens,
            ast: program,
            had_errors,
            semantic_us: sem.check_duration_us,
        }
    }

    pub fn compile_file_with_default_handler(path: &Path) -> (CompileResult, StandardHandler) {
        let handler = StandardHandler::new();
        let result = Self::compile_file(path, &handler);
        (result, handler)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiles_hello() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../examples/language-tour/01_hello.bq");
        if !path.exists() {
            return;
        }
        let handler = StandardHandler::new();
        let result = Frontend::compile_file(&path, &handler);
        assert!(!result.ast.items.is_empty() || !result.had_errors);
    }
}
