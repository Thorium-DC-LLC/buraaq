//! Semantic analysis entry — delegates to `buraaq_semantic`.

use buraaq_ast::Program;
use buraaq_diagnostics::DiagnosticHandler;
use buraaq_semantic::{analyze as run_semantic, SemanticResult};
use buraaq_source::SourceFile;

pub use buraaq_semantic::SemanticResult as AnalysisResult;

/// Run full semantic analysis. Returns true if additional errors were emitted.
pub fn analyze(program: &Program, file: &SourceFile, handler: &dyn DiagnosticHandler) -> bool {
    let SemanticResult { errors, .. } = run_semantic(program, file, handler);
    errors > 0
}

/// Run semantic analysis and return detailed timing stats.
pub fn analyze_with_stats(
    program: &Program,
    file: &SourceFile,
    handler: &dyn DiagnosticHandler,
) -> SemanticResult {
    run_semantic(program, file, handler)
}
