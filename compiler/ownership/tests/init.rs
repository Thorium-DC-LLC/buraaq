use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_ownership::{InitState, LocalId, OwnershipChecker};
use buraaq_source::{SourceFile, Span};
use buraaq_types::{TypeInterner, Ty};

#[test]
fn rejects_use_before_init() {
    let handler = StandardHandler::new();
    let file = SourceFile::new("test.bq", "");
    let mut interner = TypeInterner::new();
    let mut own = OwnershipChecker::new();
    let id = own.declare("x".into(), Ty::I32, true);
    assert!(!own.check_use(id, Span::DUMMY, &file, &handler, &interner));
    assert_eq!(own.local(id).init, InitState::Uninitialized);
    assert_eq!(handler.error_count(), 1);
}

#[test]
fn rejects_use_after_move() {
    let handler = StandardHandler::new();
    let file = SourceFile::new("test.bq", "");
    let mut interner = TypeInterner::new();
    let mut own = OwnershipChecker::new();
    let id = own.declare("s".into(), Ty::BYTES, true);
    own.initialize(id);
    assert!(own.check_move_out(id, Span::DUMMY, &file, &handler, &interner));
    assert!(!own.check_use(id, Span::DUMMY, &file, &handler, &interner));
    assert_eq!(handler.error_count(), 1);
}
