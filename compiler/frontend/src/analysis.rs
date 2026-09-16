//! IDE-oriented single-document analysis (fast incremental path).

use buraaq_ast::Program;
use buraaq_diagnostics::{Diagnostic, DiagnosticHandler, StandardHandler};
use buraaq_lexer::{Lexer, Token};
use buraaq_parser::{ParseResult, Parser};
use buraaq_semantic::{analyze, DefMap};
use buraaq_source::SourceFile;

#[derive(Clone)]
pub struct DocumentAnalysis {
    pub source: SourceFile,
    pub tokens: Vec<Token>,
    pub ast: Program,
    pub defs: DefMap,
    pub diagnostics: Vec<Diagnostic>,
    pub had_errors: bool,
    pub semantic_us: u128,
}

pub fn analyze_text(path: impl Into<std::path::PathBuf>, text: impl Into<std::sync::Arc<str>>) -> DocumentAnalysis {
    let source = SourceFile::new(path, text);
    let handler = StandardHandler::new();
    let mut analysis = analyze_source(&source, &handler);
    analysis.diagnostics = handler.diagnostics();
    analysis
}

pub fn analyze_source(source: &SourceFile, handler: &dyn DiagnosticHandler) -> DocumentAnalysis {
    let tokens = Lexer::new(source).with_diagnostics(handler).tokenize();
    let ParseResult { program, had_errors } = Parser::parse(source, handler);
    let defs = DefMap::collect(&program);
    let sem = analyze(&program, source, handler);
    DocumentAnalysis {
        source: source.clone(),
        tokens,
        ast: program,
        defs,
        diagnostics: Vec::new(),
        had_errors: had_errors || sem.errors > 0,
        semantic_us: sem.check_duration_us,
    }
}
