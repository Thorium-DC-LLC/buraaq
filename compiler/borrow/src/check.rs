use buraaq_diagnostics::{Diagnostic, DiagnosticHandler, Label};
use buraaq_ownership::{LocalId, OwnershipChecker, Place};
use buraaq_source::{SourceFile, Span};
use buraaq_types::{TypeInterner, Ty, TyKind};

use crate::loans::{LoanKind, LoanSet, Region};

/// Region-scoped borrow checker layered on ownership state.
#[derive(Debug, Default)]
pub struct BorrowChecker {
    loans: LoanSet,
    region_stack: Vec<Region>,
    in_unsafe: u32,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn enter_scope(&mut self) -> Region {
        let region = self.loans.fresh_region();
        self.region_stack.push(region);
        region
    }

    pub fn exit_scope(&mut self, region: Region) {
        self.loans.end_region(region);
        self.region_stack.retain(|r| *r != region);
    }

    pub fn enter_unsafe(&mut self) {
        self.in_unsafe += 1;
    }

    pub fn exit_unsafe(&mut self) {
        self.in_unsafe = self.in_unsafe.saturating_sub(1);
    }

    pub fn is_unsafe(&self) -> bool {
        self.in_unsafe > 0
    }

    pub fn create_ref(
        &mut self,
        local: LocalId,
        mut_: bool,
        span: Span,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
        ownership: &OwnershipChecker,
        interner: &mut TypeInterner,
    ) -> Option<Ty> {
        let place = Place::root(local);
        let kind = if mut_ { LoanKind::Mut } else { LoanKind::Shared };

        if !ownership.local(local).mutable && mut_ {
            handler.emit(
                file,
                Diagnostic::error("cannot borrow immutable binding as mutable")
                    .with_code("E0310")
                    .with_label(Label::primary(span, "binding is not declared `mut`"))
                    .with_reason("declare with `mut name = ...` to allow mutation"),
            );
            return None;
        }

        if let Some(conflict) = self.loans.conflicting(place, kind) {
            let name = ownership.local(local).name.clone();
            let msg = format!("`{name}` is already borrowed and cannot be borrowed again here");
            handler.emit(
                file,
                Diagnostic::error(msg)
                    .with_code("E0311")
                    .with_label(Label::primary(
                        span,
                        format!("this new borrow conflicts with an active borrow"),
                    ))
                    .with_label(Label::secondary(
                        conflict.span,
                        format!("`{name}` was borrowed here"),
                    ))
                    .with_help(buraaq_diagnostics::Help {
                        message: "Finish using the earlier borrow before starting a new one, or restructure so borrows do not overlap.".into(),
                        suggestion: None,
                    }),
            );
            return None;
        }

        let region = *self.region_stack.last().unwrap_or(&Region(0));
        self.loans.start_loan(place, kind, region, span);

        let inner = ownership.local(local).ty;
        Some(interner.intern(TyKind::Ref {
            mut_: mut_,
            inner,
            region,
        }))
    }

    pub fn check_deref(
        &self,
        ty: Ty,
        span: Span,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
        interner: &TypeInterner,
    ) -> Option<Ty> {
        match interner.kind(ty) {
            TyKind::Ref { inner, .. } => Some(*inner),
            TyKind::RawPtr { inner, .. } if self.is_unsafe() => Some(*inner),
            TyKind::RawPtr { .. } => {
                handler.emit(
                    file,
                    Diagnostic::error("dereference of raw pointer requires `unsafe`")
                        .with_code("E0312")
                        .with_label(Label::primary(span, "raw pointer dereference"))
                        .with_suggestion(
                        "wrap this code in an `unsafe` block",
                        "unsafe { ... }",
                    ),
                );
                None
            }
            _ => {
                handler.emit(
                    file,
                    Diagnostic::error("type cannot be dereferenced")
                        .with_code("E0313")
                        .with_label(Label::primary(span, "expected a reference type")),
                );
                None
            }
        }
    }

    pub fn check_return_ref_to_local(
        &self,
        span: Span,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
    ) -> bool {
        handler.emit(
            file,
            Diagnostic::error("cannot return reference to local variable")
                .with_code("E0314")
                .with_label(Label::primary(
                    span,
                    "reference would outlive the stack slot",
                ))
                .with_reason("return an owned value instead, or extend the owner's lifetime"),
        );
        false
    }

    pub fn check_escape_to_outer(
        &self,
        inner_region: Region,
        outer_region: Region,
        span: Span,
        file: &SourceFile,
        handler: &dyn DiagnosticHandler,
    ) -> bool {
        if inner_region.0 <= outer_region.0 {
            return true;
        }
        handler.emit(
            file,
            Diagnostic::error("borrow escapes its enclosing scope")
                .with_code("E0315")
                .with_label(Label::primary(span, "borrow cannot outlive owner"))
                .with_reason("store an owned value or shorten the borrow region"),
        );
        false
    }
}
