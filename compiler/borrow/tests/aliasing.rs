use buraaq_borrow::BorrowChecker;
use buraaq_diagnostics::{DiagnosticHandler, StandardHandler};
use buraaq_ownership::{InitState, OwnershipChecker};
use buraaq_source::{SourceFile, Span};
use buraaq_types::TypeInterner;

#[test]
fn rejects_mut_while_shared_active() {
    let file = SourceFile::new("test.bq", "");
    let handler = StandardHandler::new();
    let mut interner = TypeInterner::new();
    let mut own = OwnershipChecker::new();
    let mut borrow = BorrowChecker::new();
    borrow.enter_scope();
    let id = own.declare("x".into(), buraaq_types::Ty::I32, true);
    own.initialize(id);
    assert!(own.local(id).init == InitState::Valid);

    assert!(borrow
        .create_ref(id, false, Span::DUMMY, &file, &handler, &own, &mut interner)
        .is_some());
    assert!(borrow
        .create_ref(id, true, Span::DUMMY, &file, &handler, &own, &mut interner)
        .is_none());
    assert_eq!(handler.error_count(), 1);
}

#[test]
fn rejects_return_ref_to_local() {
    let file = SourceFile::new("test.bq", "");
    let handler = StandardHandler::new();
    let borrow = BorrowChecker::new();
    assert!(!borrow.check_return_ref_to_local(Span::DUMMY, &file, &handler));
    assert_eq!(handler.error_count(), 1);
}
