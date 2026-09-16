use crate::{BinOp, Constant, MirModule, Operand, Rvalue, Statement, UnOp};

pub struct ConstFold;

impl ConstFold {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, module: &mut MirModule) -> usize {
        let mut count = 0;
        for func in &mut module.functions {
            for bb in &mut func.blocks {
                for stmt in &mut bb.stmts {
                    if let Statement::Assign { rvalue, .. } = stmt {
                        if let Some(c) = fold_rvalue(rvalue) {
                            *rvalue = Rvalue::Literal(c);
                            count += 1;
                        }
                    }
                }
                if let Some(c) = fold_terminator_cond(&mut bb.terminator) {
                    apply_const_branch(&mut bb.terminator, c);
                    count += 1;
                }
            }
        }
        count
    }
}

fn fold_terminator_cond(term: &mut crate::Terminator) -> Option<bool> {
    use crate::Terminator;
    match term {
        Terminator::If { cond, .. } => match cond {
            Operand::Constant(Constant::Bool(b)) => Some(*b),
            Operand::Constant(Constant::I32(v)) => Some(*v != 0),
            Operand::Constant(Constant::I64(v)) => Some(*v != 0),
            _ => None,
        },
        _ => None,
    }
}

fn apply_const_branch(term: &mut crate::Terminator, taken: bool) {
    use crate::Terminator;
    if let Terminator::If { then_bb, else_bb, .. } = term {
        let target = if taken { *then_bb } else { *else_bb };
        *term = Terminator::Goto(target);
    }
}

fn fold_rvalue(rv: &Rvalue) -> Option<Constant> {
    match rv {
        Rvalue::Binary { op, left, right } => {
            let l = operand_const(left)?;
            let r = operand_const(right)?;
            fold_binary(*op, l, r)
        }
        Rvalue::Unary { op, operand } => {
            let v = operand_const(operand)?;
            fold_unary(*op, v)
        }
        _ => None,
    }
}

fn operand_const(op: &Operand) -> Option<Constant> {
    match op {
        Operand::Constant(c) => Some(c.clone()),
        _ => None,
    }
}

fn fold_binary(op: BinOp, l: Constant, r: Constant) -> Option<Constant> {
    match (op, l, r) {
        (BinOp::Add, Constant::I32(a), Constant::I32(b)) => Some(Constant::I32(a.wrapping_add(b))),
        (BinOp::Sub, Constant::I32(a), Constant::I32(b)) => Some(Constant::I32(a.wrapping_sub(b))),
        (BinOp::Mul, Constant::I32(a), Constant::I32(b)) => Some(Constant::I32(a.wrapping_mul(b))),
        (BinOp::Div, Constant::I32(a), Constant::I32(b)) if b != 0 => Some(Constant::I32(a / b)),
        (BinOp::Mod, Constant::I32(a), Constant::I32(b)) if b != 0 => Some(Constant::I32(a % b)),
        (BinOp::Eq, Constant::I32(a), Constant::I32(b)) => Some(Constant::Bool(a == b)),
        (BinOp::NotEq, Constant::I32(a), Constant::I32(b)) => Some(Constant::Bool(a != b)),
        (BinOp::Lt, Constant::I32(a), Constant::I32(b)) => Some(Constant::Bool(a < b)),
        (BinOp::Le, Constant::I32(a), Constant::I32(b)) => Some(Constant::Bool(a <= b)),
        (BinOp::Gt, Constant::I32(a), Constant::I32(b)) => Some(Constant::Bool(a > b)),
        (BinOp::Ge, Constant::I32(a), Constant::I32(b)) => Some(Constant::Bool(a >= b)),
        (BinOp::And, Constant::Bool(a), Constant::Bool(b)) => Some(Constant::Bool(a && b)),
        (BinOp::Or, Constant::Bool(a), Constant::Bool(b)) => Some(Constant::Bool(a || b)),
        (BinOp::Add, Constant::I64(a), Constant::I64(b)) => Some(Constant::I64(a.wrapping_add(b))),
        (BinOp::Sub, Constant::I64(a), Constant::I64(b)) => Some(Constant::I64(a.wrapping_sub(b))),
        (BinOp::Mul, Constant::I64(a), Constant::I64(b)) => Some(Constant::I64(a.wrapping_mul(b))),
        _ => None,
    }
}

fn fold_unary(op: UnOp, v: Constant) -> Option<Constant> {
    match (op, v) {
        (UnOp::Neg, Constant::I32(n)) => Some(Constant::I32(-n)),
        (UnOp::Neg, Constant::I64(n)) => Some(Constant::I64(-n)),
        (UnOp::Not, Constant::Bool(b)) => Some(Constant::Bool(!b)),
        (UnOp::Not, Constant::I32(n)) => Some(Constant::Bool(n == 0)),
        _ => None,
    }
}
