use buraaq_mir::{
    verify, BasicBlock, Constant, MirFunction, MirLocal, MirModule, MirTy, Operand, Statement,
    Terminator, VerifyError,
};

#[test]
fn rejects_bad_block_ref() {
    let module = MirModule {
        name: "t".into(),
        functions: vec![MirFunction {
            name: "f".into(),
            params: vec![],
            return_ty: MirTy::Void,
            locals: vec![],
            blocks: vec![BasicBlock {
                id: 0,
                stmts: vec![],
                terminator: Terminator::Goto(99),
            }],
            is_entry: false,
        }],
        ..Default::default()
    };
    assert!(matches!(verify(&module), Err(VerifyError::Ice(_))));
}

#[test]
fn accepts_simple_return() {
    let module = MirModule {
        name: "t".into(),
        functions: vec![MirFunction {
            name: "f".into(),
            params: vec![],
            return_ty: MirTy::I32,
            locals: vec![MirLocal {
                name: "x".into(),
                ty: MirTy::I32,
                mutable: false,
            }],
            blocks: vec![BasicBlock {
                id: 0,
                stmts: vec![Statement::Assign {
                    dest: 0,
                    rvalue: buraaq_mir::Rvalue::Literal(Constant::I32(1)),
                }],
                terminator: Terminator::Return(Some(Operand::Local(0))),
            }],
            is_entry: false,
        }],
        ..Default::default()
    };
    verify(&module).unwrap();
}
