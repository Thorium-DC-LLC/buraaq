use buraaq_mir::{
    optimize, BasicBlock, BinOp, Constant, MirFunction, MirLocal, MirModule, MirTy, OptConfig,
    Operand, Rvalue, Statement, Terminator,
};

fn sample_module() -> MirModule {
    MirModule {
        name: "test".into(),
        functions: vec![MirFunction {
            name: "main".into(),
            params: vec![],
            return_ty: MirTy::I32,
            locals: vec![
                MirLocal {
                    name: "a".into(),
                    ty: MirTy::I32,
                    mutable: false,
                },
                MirLocal {
                    name: "dead".into(),
                    ty: MirTy::I32,
                    mutable: false,
                },
            ],
            blocks: vec![
                BasicBlock {
                    id: 0,
                    stmts: vec![
                        Statement::Assign {
                            dest: 0,
                            rvalue: Rvalue::Binary {
                                op: BinOp::Add,
                                left: Operand::Constant(Constant::I32(10)),
                                right: Operand::Constant(Constant::I32(32)),
                            },
                        },
                        Statement::Assign {
                            dest: 1,
                            rvalue: Rvalue::Binary {
                                op: BinOp::Mul,
                                left: Operand::Constant(Constant::I32(1)),
                                right: Operand::Constant(Constant::I32(2)),
                            },
                        },
                    ],
                    terminator: Terminator::If {
                        cond: Operand::Constant(Constant::Bool(false)),
                        then_bb: 1,
                        else_bb: 2,
                    },
                },
                BasicBlock {
                    id: 1,
                    stmts: vec![],
                    terminator: Terminator::Return(Some(Operand::Local(0))),
                },
                BasicBlock {
                    id: 2,
                    stmts: vec![],
                    terminator: Terminator::Return(Some(Operand::Local(0))),
                },
            ],
            is_entry: true,
        }],
        ..Default::default()
    }
}

#[test]
fn const_fold_and_dce_and_cfg() {
    let (module, stats) = optimize(sample_module(), &OptConfig::default());
    assert!(stats.const_folds >= 2, "expected folds, got {stats:?}");
    assert!(stats.dead_stmts_removed >= 1, "expected DCE, got {stats:?}");
    assert!(stats.blocks_removed >= 1, "expected CFG simplify, got {stats:?}");
    let main = module.entry_function().unwrap();
    assert!(
        main.blocks.len() <= 2,
        "unreachable block should be removed"
    );
}

#[test]
fn debug_config_skips_opts() {
    let (_, stats) = optimize(sample_module(), &OptConfig::debug());
    assert_eq!(stats.total_changes(), 0);
    assert_eq!(stats.passes_run, 0);
}
