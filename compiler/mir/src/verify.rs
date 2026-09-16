//! MIR verifier — compiler bugs become ICE diagnostics, not UB.

use std::collections::HashSet;

use crate::{MirFunction, MirModule, Operand, Rvalue, Statement, Terminator};

#[derive(Debug, thiserror::Error)]
pub enum VerifyError {
    #[error("internal compiler error (MIR): {0}")]
    Ice(String),
}

pub fn verify(module: &MirModule) -> Result<(), VerifyError> {
    let mut names = HashSet::new();
    for f in &module.functions {
        if !names.insert(&f.name) {
            return Err(VerifyError::Ice(format!("duplicate function `{}`", f.name)));
        }
        verify_function(f)?;
    }
    Ok(())
}

fn verify_function(f: &MirFunction) -> Result<(), VerifyError> {
    if f.blocks.is_empty() {
        return Err(VerifyError::Ice(format!("function `{}` has no blocks", f.name)));
    }
    let ids: HashSet<u32> = f.blocks.iter().map(|b| b.id).collect();
    if ids.len() != f.blocks.len() {
        return Err(VerifyError::Ice(format!(
            "function `{}` has duplicate block ids",
            f.name
        )));
    }
    let nlocals = f.locals.len() as u32;
    for bb in &f.blocks {
        for stmt in &bb.stmts {
            if let Statement::Assign { dest, rvalue } = stmt {
                if *dest >= nlocals {
                    return Err(VerifyError::Ice(format!(
                        "function `{}`: dest %{dest} out of range",
                        f.name
                    )));
                }
                check_rvalue(f, rvalue)?;
            }
        }
        check_term(f, &bb.terminator, &ids)?;
    }
    Ok(())
}

fn check_rvalue(f: &MirFunction, rv: &Rvalue) -> Result<(), VerifyError> {
    match rv {
        Rvalue::Use(op) | Rvalue::Unary { operand: op, .. } => check_operand(f, op),
        Rvalue::Binary { left, right, .. } => {
            check_operand(f, left)?;
            check_operand(f, right)
        }
        Rvalue::Call { args, .. } => {
            for a in args {
                check_operand(f, a)?;
            }
            Ok(())
        }
        Rvalue::Index { base, index, .. } => {
            check_operand(f, base)?;
            check_operand(f, index)
        }
        Rvalue::Field { base, .. } | Rvalue::Cast { operand: base, .. } | Rvalue::Load { ptr: base, .. } => {
            check_operand(f, base)
        }
        Rvalue::HeapAlloc { count, .. } => check_operand(f, count),
        Rvalue::AddrOf { local, .. } => {
            if *local >= f.locals.len() as u32 {
                Err(VerifyError::Ice(format!(
                    "function `{}`: AddrOf local out of range",
                    f.name
                )))
            } else {
                Ok(())
            }
        }
        Rvalue::Literal(_) | Rvalue::Aggregate { .. } => Ok(()),
    }
}

fn check_operand(f: &MirFunction, op: &Operand) -> Result<(), VerifyError> {
    match op {
        Operand::Local(id) if *id >= f.locals.len() as u32 => Err(VerifyError::Ice(format!(
            "function `{}`: local %{id} out of range",
            f.name
        ))),
        _ => Ok(()),
    }
}

fn check_term(f: &MirFunction, term: &Terminator, ids: &HashSet<u32>) -> Result<(), VerifyError> {
    let exists = |bb: u32| {
        if ids.contains(&bb) {
            Ok(())
        } else {
            Err(VerifyError::Ice(format!(
                "function `{}`: terminator refers to missing block %{bb}",
                f.name
            )))
        }
    };
    match term {
        Terminator::Goto(bb) => exists(*bb),
        Terminator::If { cond, then_bb, else_bb } => {
            check_operand(f, cond)?;
            exists(*then_bb)?;
            exists(*else_bb)
        }
        Terminator::Switch { discr, arms, otherwise } => {
            check_operand(f, discr)?;
            for (_, bb) in arms {
                exists(*bb)?;
            }
            exists(*otherwise)
        }
        Terminator::Return(Some(op)) => check_operand(f, op),
        Terminator::Return(None) | Terminator::Unreachable => Ok(()),
    }
}
