use buraaq_mir::{insert_drops, lower_program, Rvalue, Statement, Terminator};
use buraaq_parser::Parser;
use buraaq_source::SourceFile;

#[test]
fn inserts_storage_dead_in_reverse_order_on_return() {
    let src = r#"
fn demo() {
    a = 1
    b = 2
}
"#;
    let file = SourceFile::new("drop.bq", src);
    let parse = Parser::parse(&file, &buraaq_diagnostics::StandardHandler::new());
    let mut mir = lower_program(&parse.program).expect("lower");
    insert_drops(&mut mir);
    let f = mir.functions.iter().find(|f| f.name == "demo").unwrap();
    let ret_bb = f
        .blocks
        .iter()
        .find(|b| matches!(b.terminator, Terminator::Return(_)))
        .unwrap();
    let deads: Vec<u32> = ret_bb
        .stmts
        .iter()
        .filter_map(|s| match s {
            Statement::StorageDead { local } => Some(*local),
            _ => None,
        })
        .collect();
    assert!(
        deads.len() >= 2,
        "expected StorageDead for locals, got {deads:?}"
    );
    let mut sorted = deads.clone();
    sorted.sort();
    sorted.reverse();
    assert_eq!(deads, sorted, "destruction must be reverse declaration order");
}

#[test]
fn early_return_also_gets_storage_dead() {
    let src = r#"
fn demo(c: bool) {
    a = 1
    if c {
        return
    }
}
"#;
    let file = SourceFile::new("drop_if.bq", src);
    let parse = Parser::parse(&file, &buraaq_diagnostics::StandardHandler::new());
    let mut mir = lower_program(&parse.program).expect("lower");
    insert_drops(&mut mir);
    let f = mir.functions.iter().find(|f| f.name == "demo").unwrap();
    let returns: Vec<_> = f
        .blocks
        .iter()
        .filter(|b| matches!(b.terminator, Terminator::Return(_)))
        .collect();
    assert!(returns.len() >= 2, "if-return and fallthrough must both return");
    for bb in returns {
        let has_dead = bb
            .stmts
            .iter()
            .any(|s| matches!(s, Statement::StorageDead { .. }));
        assert!(has_dead, "return path missing StorageDead");
    }
}

#[test]
fn break_runs_loop_defer_before_goto() {
    let src = r#"
fn demo() {
    mut i = 0
    while i < 4 {
        defer print(1)
        break
    }
}
"#;
    let file = SourceFile::new("defer_break.bq", src);
    let parse = Parser::parse(&file, &buraaq_diagnostics::StandardHandler::new());
    assert!(!parse.had_errors, "parse defer/break");
    let mir = lower_program(&parse.program).expect("lower");
    let f = mir.functions.iter().find(|f| f.name == "demo").unwrap();
    let mut saw_defer_then_break = false;
    for bb in &f.blocks {
        let has_print = bb.stmts.iter().any(|s| match s {
            Statement::Assign {
                rvalue: Rvalue::Call { func, .. },
                ..
            } => func.contains("print"),
            _ => false,
        });
        if has_print && matches!(bb.terminator, Terminator::Goto(_)) {
            saw_defer_then_break = true;
        }
    }
    assert!(
        saw_defer_then_break,
        "break must run loop defer before leaving the body"
    );
}
