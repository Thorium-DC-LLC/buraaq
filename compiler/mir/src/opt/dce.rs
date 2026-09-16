use std::collections::HashSet;

use crate::{MirFunction, MirModule, Operand, Statement, Terminator};

pub struct DeadCodeElim;

impl DeadCodeElim {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, module: &mut MirModule) -> usize {
        let mut removed = 0;
        for func in &mut module.functions {
            removed += self.eliminate_in_function(func);
        }
        removed
    }

    fn eliminate_in_function(&self, func: &mut MirFunction) -> usize {
        let live = compute_live_locals(func);
        let mut removed = 0;
        for bb in &mut func.blocks {
            bb.stmts.retain(|stmt| {
                if let Statement::Assign { dest, rvalue } = stmt {
                    let side_effect = rvalue_has_side_effect(rvalue);
                    if !side_effect && !live.contains(dest) {
                        removed += 1;
                        return false;
                    }
                }
                true
            });
        }
        removed
    }
}

fn compute_live_locals(func: &MirFunction) -> HashSet<u32> {
    let mut live = HashSet::new();
    for bb in &func.blocks {
        for stmt in &bb.stmts {
            if let Statement::Assign { rvalue, .. } = stmt {
                collect_operand_uses(rvalue, &mut live);
            }
            if let Statement::Store { ptr, value, .. } = stmt {
                collect_operand(ptr, &mut live);
                collect_operand(value, &mut live);
            }
        }
        collect_term_uses(&bb.terminator, &mut live);
    }
    live
}

fn collect_term_uses(term: &Terminator, live: &mut HashSet<u32>) {
    match term {
        Terminator::Return(Some(op)) => collect_operand(op, live),
        Terminator::If { cond, .. } => collect_operand(cond, live),
        Terminator::Switch { discr, .. } => collect_operand(discr, live),
        _ => {}
    }
}

fn collect_operand(op: &Operand, live: &mut HashSet<u32>) {
    if let Operand::Local(id) = op {
        live.insert(*id);
    }
}

fn collect_operand_uses(rv: &crate::Rvalue, live: &mut HashSet<u32>) {
    use crate::Rvalue;
    match rv {
        Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } => collect_operand(op, live),
        Rvalue::Binary { left, right, .. } => {
            collect_operand(left, live);
            collect_operand(right, live);
        }
        Rvalue::Call { args, .. } => {
            for a in args {
                collect_operand(a, live);
            }
        }
        Rvalue::Aggregate { fields, .. } => {
            for f in fields {
                collect_operand(f, live);
            }
        }
        Rvalue::Field { base, .. } => collect_operand(base, live),
        Rvalue::Index { base, index, .. } => {
            collect_operand(base, live);
            collect_operand(index, live);
        }
        Rvalue::Cast { operand, .. } => collect_operand(operand, live),
        Rvalue::HeapAlloc { count, .. } => collect_operand(count, live),
        Rvalue::Load { ptr, .. } => collect_operand(ptr, live),
        Rvalue::AddrOf { local, .. } => {
            live.insert(*local);
        }
        Rvalue::Literal(_) => {}
    }
}

fn rvalue_has_side_effect(rv: &crate::Rvalue) -> bool {
    matches!(
        rv,
        crate::Rvalue::Call { .. }
            | crate::Rvalue::HeapAlloc { .. }
            | crate::Rvalue::Load { .. }
    )
}
