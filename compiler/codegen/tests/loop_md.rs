use buraaq_codegen::{emit_llvm_ir, LlvmEmitOptions, OptLevel};

#[test]
fn newline_string_constant_fits_llvm_array() {
    let mut opts = LlvmEmitOptions::default();
    opts.module_name = "nl".into();
    let ir = emit_llvm_ir(
        &buraaq_mir::MirModule {
            name: "nl".into(),
            functions: vec![buraaq_mir::MirFunction {
                name: "main".into(),
                params: vec![],
                return_ty: buraaq_mir::MirTy::Void,
                locals: vec![],
                blocks: vec![buraaq_mir::BasicBlock {
                    id: 0,
                    stmts: vec![buraaq_mir::Statement::Assign {
                        dest: 0,
                        rvalue: buraaq_mir::Rvalue::Use(buraaq_mir::Operand::Constant(
                            buraaq_mir::Constant::Str("\n".into()),
                        )),
                    }],
                    terminator: buraaq_mir::Terminator::Return(None),
                }],
                is_entry: true,
            }],
            ..Default::default()
        },
        &opts,
    )
    .expect("emit");
    assert!(
        ir.contains("[2 x i8] c\"\\0A\\00\""),
        "expected newline as single-byte LLVM string, got:\n{ir}"
    );
}

use buraaq_mir::{
    BasicBlock, BinOp, Constant, MirFunction, MirLocal, MirModule, MirTy, Operand, Rvalue, Statement,
    Terminator,
};

fn while_module() -> MirModule {
    MirModule {
        name: "loop".into(),
        functions: vec![MirFunction {
            name: "main".into(),
            params: vec![],
            return_ty: MirTy::Void,
            locals: vec![MirLocal {
                name: "c".into(),
                ty: MirTy::Bool,
                mutable: false,
            }],
            blocks: vec![
                BasicBlock {
                    id: 0,
                    stmts: vec![],
                    terminator: Terminator::Goto(1),
                },
                BasicBlock {
                    id: 1,
                    stmts: vec![Statement::Assign {
                        dest: 0,
                        rvalue: Rvalue::Literal(Constant::Bool(true)),
                    }],
                    terminator: Terminator::If {
                        cond: Operand::Local(0),
                        then_bb: 2,
                        else_bb: 3,
                    },
                },
                BasicBlock {
                    id: 2,
                    stmts: vec![Statement::Assign {
                        dest: 0,
                        rvalue: Rvalue::Binary {
                            op: BinOp::Add,
                            left: Operand::Constant(Constant::I32(1)),
                            right: Operand::Constant(Constant::I32(1)),
                        },
                    }],
                    terminator: Terminator::Goto(1),
                },
                BasicBlock {
                    id: 3,
                    stmts: vec![],
                    terminator: Terminator::Return(None),
                },
            ],
            is_entry: true,
        }],
        ..Default::default()
    }
}

#[test]
fn while_backedge_gets_mustprogress_and_release_attrs() {
    let mut opts = LlvmEmitOptions::default();
    opts.opt = OptLevel::Release;
    let ir = emit_llvm_ir(&while_module(), &opts).expect("emit");
    assert!(
        ir.contains("llvm.loop.mustprogress"),
        "expected loop metadata, got:\n{ir}"
    );
    let latch_mds = ir.matches("!llvm.loop").count();
    assert_eq!(latch_mds, 1, "only the latch should carry loop metadata:\n{ir}");
    assert!(
        ir.contains("mustprogress nounwind"),
        "expected function attrs, got:\n{ir}"
    );
    assert!(
        ir.contains("define internal"),
        "Buraaq fns should be internal so clang can IPO"
    );
    assert!(
        ir.contains("add nsw"),
        "signed add must be nsw like C++ -O2, got:\n{ir}"
    );
    assert!(
        ir.contains("target datalayout"),
        "SCEV needs a datalayout, got:\n{ir}"
    );
}
