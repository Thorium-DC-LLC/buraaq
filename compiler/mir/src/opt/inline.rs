use crate::{MirModule, Operand, Rvalue, Statement};

/// Inline trivial call sites (single-use literal-return helpers).
pub struct InlineSmallCalls {
    max_stmts: usize,
}

impl InlineSmallCalls {
    pub fn new(max_stmts: usize) -> Self {
        Self { max_stmts }
    }

    pub fn run(&mut self, module: &mut MirModule) -> usize {
        let mut inlined = 0;
        let candidates: Vec<(String, Vec<Statement>)> = module
            .functions
            .iter()
            .filter(|f| !f.is_entry && f.blocks.len() == 1)
            .filter_map(|f| {
                let bb = f.blocks.first()?;
                if bb.stmts.len() > self.max_stmts {
                    return None;
                }
                if !matches!(bb.terminator, crate::Terminator::Return(_)) {
                    return None;
                }
                Some((f.name.clone(), bb.stmts.clone()))
            })
            .collect();

        for func in &mut module.functions {
            for bb in &mut func.blocks {
                for stmt in &mut bb.stmts {
                    if let Statement::Assign { rvalue, .. } = stmt {
                        if let Rvalue::Call { func: name, args, .. } = rvalue {
                            if let Some((_, body)) = candidates.iter().find(|(n, _)| n == name) {
                                if args.is_empty() && body.len() == 1 {
                                    if let Statement::Assign { rvalue: inner, .. } = &body[0] {
                                        *rvalue = clone_rvalue(inner);
                                        inlined += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        inlined
    }
}

fn clone_rvalue(rv: &Rvalue) -> Rvalue {
    match rv {
        Rvalue::Use(op) => Rvalue::Use(clone_operand(op)),
        Rvalue::Literal(c) => Rvalue::Literal(c.clone()),
        Rvalue::Binary { op, left, right } => Rvalue::Binary {
            op: *op,
            left: clone_operand(left),
            right: clone_operand(right),
        },
        Rvalue::Unary { op, operand } => Rvalue::Unary {
            op: *op,
            operand: clone_operand(operand),
        },
        other => other.clone(),
    }
}

fn clone_operand(op: &Operand) -> Operand {
    match op {
        Operand::Local(id) => Operand::Local(*id),
        Operand::Constant(c) => Operand::Constant(c.clone()),
    }
}
