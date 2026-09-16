use std::io::{self, Write};
use std::sync::{Arc, Mutex};

use buraaq_source::SourceFile;

use crate::{Diagnostic, Emitter, Level};

/// Collects diagnostics during compilation.
pub trait DiagnosticHandler: Send + Sync {
    fn emit(&self, file: &SourceFile, diagnostic: Diagnostic);
    fn error_count(&self) -> usize;
    fn warning_count(&self) -> usize;
    fn has_errors(&self) -> bool {
        self.error_count() > 0
    }
}

#[derive(Default)]
pub struct StandardHandler {
    inner: Mutex<HandlerState>,
}

#[derive(Default)]
struct HandlerState {
    diagnostics: Vec<(usize, Diagnostic)>,
    errors: usize,
    warnings: usize,
}

impl StandardHandler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.inner
            .lock()
            .unwrap()
            .diagnostics
            .iter()
            .map(|(_, d)| d.clone())
            .collect()
    }

    pub fn print_all(&self, file: &SourceFile, writer: &mut dyn Write) -> io::Result<()> {
        let diags = self.diagnostics();
        for diag in &diags {
            Emitter::new(&mut *writer, file).emit(diag)?;
        }
        Ok(())
    }
}

impl DiagnosticHandler for StandardHandler {
    fn emit(&self, _file: &SourceFile, diagnostic: Diagnostic) {
        let mut state = self.inner.lock().unwrap();
        match diagnostic.level {
            Level::Error => state.errors += 1,
            Level::Warning => state.warnings += 1,
            Level::Note => {}
        }
        state.diagnostics.push((0, diagnostic));
    }

    fn error_count(&self) -> usize {
        self.inner.lock().unwrap().errors
    }

    fn warning_count(&self) -> usize {
        self.inner.lock().unwrap().warnings
    }
}

/// Shared handler for parser/lexer stages.
pub type SharedHandler = Arc<dyn DiagnosticHandler>;

pub fn shared_handler() -> SharedHandler {
    Arc::new(StandardHandler::new())
}
