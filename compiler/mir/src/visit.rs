use crate::{
    BasicBlock, MirFunction, MirModule, Operand, Rvalue, Statement, Terminator,
};

/// Traversal hook for MIR optimization and analysis passes.
pub trait MirVisitor {
    fn visit_module(&mut self, module: &MirModule) {
        for func in &module.functions {
            self.visit_function(func);
        }
    }

    fn visit_function(&mut self, func: &MirFunction) {
        for bb in &func.blocks {
            self.visit_block(bb);
        }
    }

    fn visit_block(&mut self, bb: &BasicBlock) {
        for stmt in &bb.stmts {
            self.visit_statement(stmt);
        }
        self.visit_terminator(&bb.terminator);
    }

    fn visit_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Assign { rvalue, .. } => self.visit_rvalue(rvalue),
            Statement::Store { ptr, value, .. } => {
                self.visit_operand(ptr);
                self.visit_operand(value);
            }
            Statement::StorageLive { .. } | Statement::StorageDead { .. } => {}
        }
    }

    fn visit_rvalue(&mut self, rv: &Rvalue) {
        match rv {
            Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } => self.visit_operand(op),
            Rvalue::Binary { left, right, .. } => {
                self.visit_operand(left);
                self.visit_operand(right);
            }
            Rvalue::Call { args, .. } => {
                for a in args {
                    self.visit_operand(a);
                }
            }
            Rvalue::Aggregate { fields, .. } => {
                for f in fields {
                    self.visit_operand(f);
                }
            }
            Rvalue::Field { base, .. } => self.visit_operand(base),
            Rvalue::Index { base, index, .. } => {
                self.visit_operand(base);
                self.visit_operand(index);
            }
            Rvalue::Cast { operand, .. } => self.visit_operand(operand),
            Rvalue::HeapAlloc { count, .. } => self.visit_operand(count),
            Rvalue::Load { ptr, .. } => self.visit_operand(ptr),
            Rvalue::AddrOf { .. } | Rvalue::Literal(_) => {}
        }
    }

    fn visit_operand(&mut self, op: &Operand) {
        let _ = op;
    }

    fn visit_terminator(&mut self, term: &Terminator) {
        match term {
            Terminator::Return(v) => {
                if let Some(op) = v {
                    self.visit_operand(op);
                }
            }
            Terminator::If { cond, .. } => self.visit_operand(cond),
            Terminator::Switch { discr, .. } => self.visit_operand(discr),
            Terminator::Goto(_) | Terminator::Unreachable => {}
        }
    }
}
