//! Insert deterministic destruction at every function exit.

use std::collections::HashSet;

use crate::{
    MirFunction, MirLocal, MirModule, MirTy, Operand, Rvalue, Statement, Terminator,
};

/// Emit destructor calls (when present) and `StorageDead` before each return.
/// Locals are destroyed in reverse declaration order, once per function-exit path.
/// Loop `break`/`continue` still run `defer` and `StorageDead` for the loop
/// scope in `lower.rs` (`close_loop_defers` + `emit_scope_deads`).
pub fn insert_drops(module: &mut MirModule) {
    let droppers: HashSet<String> = module
        .functions
        .iter()
        .filter(|f| f.name.ends_with("_drop"))
        .map(|f| f.name.clone())
        .collect();
    for func in &mut module.functions {
        insert_drops_fn(func, &droppers);
    }
}

fn insert_drops_fn(func: &mut MirFunction, droppers: &HashSet<String>) {
    if func.locals.is_empty() {
        return;
    }
    let needs_sink = func.locals.iter().any(|local| {
        matches!(&local.ty, MirTy::Struct { name } if droppers.contains(&format!("{name}_drop")))
    });
    let sink = if needs_sink {
        let id = func.locals.len() as u32;
        func.locals.push(MirLocal {
            name: "__drop_sink".into(),
            ty: MirTy::Void,
            mutable: false,
        });
        id
    } else {
        0
    };
    let cleanup = cleanup_stmts(&func.locals, droppers, sink);
    for bb in &mut func.blocks {
        if matches!(bb.terminator, Terminator::Return(_)) {
            for s in &cleanup {
                if !already_emitted(&bb.stmts, s) {
                    bb.stmts.push(s.clone());
                }
            }
        }
    }
}

fn cleanup_stmts(locals: &[MirLocal], droppers: &HashSet<String>, sink: u32) -> Vec<Statement> {
    let mut out = Vec::new();
    for (i, local) in locals.iter().enumerate().rev() {
        let id = i as u32;
        if local.name == "__drop_sink" {
            continue;
        }
        if let MirTy::Struct { name } = &local.ty {
            let dname = format!("{name}_drop");
            if droppers.contains(&dname) {
                out.push(Statement::Assign {
                    dest: sink,
                    rvalue: Rvalue::Call {
                        func: dname,
                        args: vec![Operand::Local(id)],
                        ret_ty: MirTy::Void,
                    },
                });
            }
        }
        out.push(Statement::StorageDead { local: id });
    }
    out
}

fn already_emitted(stmts: &[Statement], next: &Statement) -> bool {
    match next {
        Statement::StorageDead { local } => stmts.iter().any(|s| {
            matches!(s, Statement::StorageDead { local: l } if l == local)
        }),
        Statement::Assign {
            rvalue: Rvalue::Call { func, .. },
            ..
        } => stmts.iter().any(|s| {
            matches!(
                s,
                Statement::Assign {
                    rvalue: Rvalue::Call { func: f, .. },
                    ..
                } if f == func
            )
        }),
        _ => false,
    }
}
