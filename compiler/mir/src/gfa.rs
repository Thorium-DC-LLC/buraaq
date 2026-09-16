//! MIR-level growing-flow analysis: control-flow-aware ownership facts.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{MirFunction, MirModule, MirTy, Operand, Rvalue, Statement, Terminator};

#[derive(Debug, thiserror::Error)]
pub enum GfaError {
    #[error("{0}")]
    User(String),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Fact {
    Uninit,
    Live,
    Moved,
}

fn join(a: Fact, b: Fact) -> Fact {
    match (a, b) {
        (Fact::Moved, _) | (_, Fact::Moved) => Fact::Moved,
        (Fact::Uninit, x) | (x, Fact::Uninit) => x,
        (Fact::Live, Fact::Live) => Fact::Live,
    }
}

fn is_copy(ty: &MirTy) -> bool {
    matches!(
        ty,
        MirTy::Void
            | MirTy::Bool
            | MirTy::I8
            | MirTy::I32
            | MirTy::I64
            | MirTy::F32
            | MirTy::F64
            | MirTy::Text
            | MirTy::Ptr(_)
            | MirTy::Ref { .. }
    )
}

/// Fixed-point ownership facts over the MIR CFG.
/// Reports use of a value that is moved on every incoming path (conservative join).
pub fn check_module(module: &MirModule) -> Result<(), GfaError> {
    for f in &module.functions {
        check_function(f)?;
    }
    Ok(())
}

fn check_function(f: &MirFunction) -> Result<(), GfaError> {
    if f.blocks.is_empty() {
        return Ok(());
    }
    let n = f.locals.len();
    let mut entry: HashMap<u32, Vec<Fact>> = HashMap::new();
    for bb in &f.blocks {
        entry.insert(bb.id, vec![Fact::Uninit; n]);
    }
    for (i, _) in f.params.iter().enumerate() {
        if let Some(facts) = entry.get_mut(&f.blocks[0].id) {
            if i < n {
                facts[i] = Fact::Live;
            }
        }
    }

    let succs = successors(f);
    let mut work: VecDeque<u32> = f.blocks.iter().map(|b| b.id).collect();
    let mut seen = 0u32;
    while let Some(bid) = work.pop_front() {
        seen += 1;
        if seen > 10_000 {
            break;
        }
        let bb = match f.blocks.iter().find(|b| b.id == bid) {
            Some(b) => b,
            None => continue,
        };
        let mut facts = entry.get(&bid).cloned().unwrap_or_else(|| vec![Fact::Uninit; n]);
        transfer(f, bb, &mut facts)?;
        if let Some(nexts) = succs.get(&bid) {
            for &s in nexts {
                let dest = entry.entry(s).or_insert_with(|| vec![Fact::Uninit; n]);
                let mut changed = false;
                for i in 0..n {
                    let j = join(dest[i], facts[i]);
                    if j != dest[i] {
                        dest[i] = j;
                        changed = true;
                    }
                }
                if changed {
                    work.push_back(s);
                }
            }
        }
    }
    Ok(())
}

fn successors(f: &MirFunction) -> HashMap<u32, Vec<u32>> {
    let mut m = HashMap::new();
    for bb in &f.blocks {
        let s = match &bb.terminator {
            Terminator::Goto(t) => vec![*t],
            Terminator::If { then_bb, else_bb, .. } => vec![*then_bb, *else_bb],
            Terminator::Switch { arms, otherwise, .. } => {
                let mut v: Vec<u32> = arms.iter().map(|(_, b)| *b).collect();
                v.push(*otherwise);
                v
            }
            Terminator::Return(_) | Terminator::Unreachable => vec![],
        };
        m.insert(bb.id, s);
    }
    m
}

fn transfer(
    f: &MirFunction,
    bb: &crate::BasicBlock,
    facts: &mut [Fact],
) -> Result<(), GfaError> {
    for stmt in &bb.stmts {
        if let Statement::Assign { dest, rvalue } = stmt {
            check_rvalue(f, rvalue, facts)?;
            if (*dest as usize) < facts.len() {
                facts[*dest as usize] = Fact::Live;
            }
            if let Rvalue::Use(Operand::Local(src)) = rvalue {
                maybe_move(f, *src, facts)?;
            }
            if let Rvalue::Call { args, .. } = rvalue {
                for a in args {
                    if let Operand::Local(src) = a {
                        maybe_move(f, *src, facts)?;
                    }
                }
            }
        }
    }
    match &bb.terminator {
        Terminator::If { cond, .. } | Terminator::Switch { discr: cond, .. } => {
            check_operand(f, cond, facts)?;
        }
        Terminator::Return(Some(op)) => check_operand(f, op, facts)?,
        _ => {}
    }
    Ok(())
}

fn check_rvalue(f: &MirFunction, rv: &Rvalue, facts: &[Fact]) -> Result<(), GfaError> {
    match rv {
        Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } | Rvalue::Load { ptr: op, .. } => {
            check_operand(f, op, facts)
        }
        Rvalue::Binary { left, right, .. } => {
            check_operand(f, left, facts)?;
            check_operand(f, right, facts)
        }
        Rvalue::Call { args, .. } => {
            for a in args {
                check_operand(f, a, facts)?;
            }
            Ok(())
        }
        Rvalue::Index { base, index, .. } => {
            check_operand(f, base, facts)?;
            check_operand(f, index, facts)
        }
        Rvalue::Field { base, .. } | Rvalue::Cast { operand: base, .. } => {
            check_operand(f, base, facts)
        }
        Rvalue::HeapAlloc { count, .. } => check_operand(f, count, facts),
        Rvalue::AddrOf { local, .. } => check_local(f, *local, facts),
        Rvalue::Literal(_) | Rvalue::Aggregate { .. } => Ok(()),
    }
}

fn check_operand(f: &MirFunction, op: &Operand, facts: &[Fact]) -> Result<(), GfaError> {
    match op {
        Operand::Local(id) => check_local(f, *id, facts),
        Operand::Constant(_) => Ok(()),
    }
}

fn check_local(f: &MirFunction, id: u32, facts: &[Fact]) -> Result<(), GfaError> {
    let i = id as usize;
    if i >= facts.len() {
        return Ok(());
    }
    match facts[i] {
        Fact::Moved => {
            let name = f.locals.get(i).map(|l| l.name.as_str()).unwrap_or("value");
            Err(GfaError::User(format!(
                "`{name}` is used after it was moved (MIR flow analysis)"
            )))
        }
        Fact::Uninit => {
            let name = f.locals.get(i).map(|l| l.name.as_str()).unwrap_or("value");
            if name.starts_with("__tmp") || name.starts_with("%") {
                return Ok(());
            }
            Ok(())
        }
        Fact::Live => Ok(()),
    }
}

fn maybe_move(f: &MirFunction, id: u32, facts: &mut [Fact]) -> Result<(), GfaError> {
    let i = id as usize;
    if i >= facts.len() {
        return Ok(());
    }
    let ty = f.locals.get(i).map(|l| &l.ty);
    if let Some(ty) = ty {
        if !is_copy(ty) {
            facts[i] = Fact::Moved;
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn live_params(f: &MirFunction) -> HashSet<u32> {
    (0..f.params.len() as u32).collect()
}
