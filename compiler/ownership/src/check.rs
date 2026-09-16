use buraaq_diagnostics::{Diagnostic, DiagnosticHandler, Label};
use buraaq_source::{SourceFile, Span};
use buraaq_types::{TypeInterner, Ty};

use crate::state::{InitState, LocalId, LocalState, Place};

/// Tracks ownership state within a lexical scope.
#[derive(Debug, Default)]
pub struct OwnershipChecker {
    locals: Vec<LocalState>,
    drop_order: Vec<LocalId>,
}

impl OwnershipChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn declare(&mut self, name: String, ty: Ty, mutable: bool) -> LocalId {
        let id = LocalId(self.locals.len() as u32);
        self.locals.push(LocalState::new(name, ty, mutable));
        self.drop_order.push(id);
        id
    }

    pub fn local(&self, id: LocalId) -> &LocalState {
        &self.locals[id.0 as usize]
    }

    pub fn local_mut(&mut self, id: LocalId) -> &mut LocalState {
        &mut self.locals[id.0 as usize]
    }

    pub fn initialize(&mut self, id: LocalId) {
        self.locals[id.0 as usize].init = InitState::Valid;
    }

    pub fn check_use(
        &self,
        id: LocalId,
        span: Span,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
        interner: &TypeInterner,
    ) -> bool {
        let local = &self.locals[id.0 as usize];
        match local.init {
            InitState::Uninitialized => {
                handler.emit(
                    file,
                    Diagnostic::error(format!("`{}` used before initialization", local.name))
                        .with_code("E0301")
                        .with_label(Label::primary(span, "binding is not initialized yet"))
                        .with_suggestion("assign a value before use", format!("{} = ...", local.name)),
                );
                false
            }
            InitState::Moved => {
                let mut diag = Diagnostic::error(format!(
                    "`{}` was transferred and cannot be used again",
                    local.name
                ))
                .with_code("E0302")
                .with_label(Label::primary(
                    span,
                    format!("you attempted to use `{}` again here", local.name),
                ));
                if let Some(moved_at) = local.moved_at {
                    let note = local
                        .move_note
                        .clone()
                        .unwrap_or_else(|| format!("`{}` was transferred here", local.name));
                    diag = diag.with_label(Label::secondary(moved_at, note));
                }
                diag = diag.with_help(buraaq_diagnostics::Help {
                    message: format!(
                        "If the callee only needs temporary access to `{}`, pass a borrow (`ref {}`) instead of transferring ownership.",
                        local.name, local.name
                    ),
                    suggestion: Some(buraaq_diagnostics::Suggestion {
                        replacement: format!("ref {}", local.name),
                        span: Some(span),
                    }),
                });
                handler.emit(file, diag);
                false
            }
            InitState::Valid | InitState::PartiallyMoved => {
                if interner.is_copy(local.ty) {
                    true
                } else {
                    true
                }
            }
        }
    }

    pub fn move_place(
        &mut self,
        place: Place,
        span: Span,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
        interner: &TypeInterner,
    ) -> bool {
        if !self.check_use(place.local, span, file, handler, interner) {
            return false;
        }
        let local = &mut self.locals[place.local.0 as usize];
        if place.projection == 0 {
            local.init = InitState::Moved;
        } else {
            local.init = InitState::PartiallyMoved;
        }
        true
    }

    pub fn check_move_out(
        &mut self,
        id: LocalId,
        span: Span,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
        interner: &TypeInterner,
    ) -> bool {
        self.check_move_out_with_note(id, span, None, file, handler, interner)
    }

    pub fn check_move_out_with_note(
        &mut self,
        id: LocalId,
        span: Span,
        note: Option<String>,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
        interner: &TypeInterner,
    ) -> bool {
        if !self.check_use(id, span, file, handler, interner) {
            return false;
        }
        if interner.is_copy(self.locals[id.0 as usize].ty) {
            return true;
        }
        let local = &mut self.locals[id.0 as usize];
        local.init = InitState::Moved;
        local.moved_at = Some(span);
        local.move_note = note;
        true
    }

    pub fn reinit(&mut self, id: LocalId) {
        self.locals[id.0 as usize].init = InitState::Valid;
    }

    /// Drop order: reverse of declaration (stack unwind order).
    pub fn drop_order(&self) -> impl Iterator<Item = LocalId> + '_ {
        self.drop_order.iter().rev().copied()
    }

    pub fn check_definite_init_at_scope_end(
        &self,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
    ) -> usize {
        let mut errors = 0;
        for local in &self.locals {
            if matches!(local.init, InitState::Uninitialized) {
                handler.emit(
                    file,
                    Diagnostic::error(format!("`{}` may be used uninitialized", local.name))
                        .with_code("E0303")
                        .with_reason("all paths must initialize bindings before scope exit"),
                );
                errors += 1;
            }
        }
        errors
    }
}
