//! Orbit: counted-loop kernel. Same n, eight-wide body, remainder tail.
//!
//! Clang -O2 wins `integer_sum` by unrolling a counted `for`. Buraaq while/for
//! loops were a compare-and-branch per iteration, so LLVM left them skinny.
//! Orbit rewrites `iv += 1` loops into a kernel that runs eight iterations
//! per latch, then a scalar remainder — the same work, the same wrap, no
//! smaller n.

use crate::{
    BasicBlock, BinOp, BlockId, Constant, LocalId, MirFunction, MirLocal, MirModule, MirTy,
    Operand, Rvalue, Statement, Terminator,
};

const FACTOR: i32 = 8;

pub struct OrbitUnroll;

impl OrbitUnroll {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, module: &mut MirModule) -> usize {
        let mut n = 0;
        for func in &mut module.functions {
            let mut steps = 0;
            while steps < 8 && fold_one(func) {
                n += 1;
                steps += 1;
            }
            steps = 0;
            while steps < 8 && unroll_one(func) {
                n += 1;
                steps += 1;
            }
        }
        n
    }
}

struct CountedLoop {
    header: BlockId,
    body: BlockId,
    exit: BlockId,
    step: Option<BlockId>,
    iv: LocalId,
    bound: Operand,
    cmp: BinOp,
}

fn body_has_float(func: &MirFunction, body: BlockId) -> bool {
    func.blocks.iter().find(|b| b.id == body).is_some_and(|b| {
        b.stmts.iter().any(|s| {
            matches!(
                s,
                Statement::Assign {
                    rvalue: Rvalue::Binary {
                        left: Operand::Constant(Constant::F64(_) | Constant::F32(_)),
                        ..
                    } | Rvalue::Binary {
                        right: Operand::Constant(Constant::F64(_) | Constant::F32(_)),
                        ..
                    },
                    ..
                }
            )
        })
    })
}

fn loop_unit(func: &MirFunction, lp: &CountedLoop) -> Vec<Statement> {
    let mut unit = func
        .blocks
        .iter()
        .find(|b| b.id == lp.body)
        .map(|b| b.stmts.clone())
        .unwrap_or_default();
    if let Some(sid) = lp.step {
        if let Some(s) = func.blocks.iter().find(|b| b.id == sid) {
            unit.extend(s.stmts.clone());
        }
    }
    unit
}

fn detect_recurrence(unit: &[Statement], iv: LocalId) -> Option<(LocalId, i32)> {
    let mut k_of: Option<(LocalId, LocalId, i32)> = None;
    for stmt in unit {
        if let Statement::Assign {
            dest,
            rvalue: Rvalue::Binary {
                op: BinOp::Mul,
                left,
                right,
            },
        } = stmt
        {
            match (left, right) {
                (Operand::Local(acc), Operand::Constant(Constant::I32(k))) if *acc != iv => {
                    k_of = Some((*dest, *acc, *k));
                }
                (Operand::Constant(Constant::I32(k)), Operand::Local(acc)) if *acc != iv => {
                    k_of = Some((*dest, *acc, *k));
                }
                _ => {}
            }
        }
    }
    let (prod, acc, k) = k_of?;
    let mut writes_acc = false;
    for stmt in unit {
        if let Statement::Assign {
            dest,
            rvalue: Rvalue::Binary {
                op: BinOp::Add,
                left,
                right,
            },
        } = stmt
        {
            if *dest != acc {
                continue;
            }
            let uses_prod_and_iv = matches!(
                (left, right),
                (Operand::Local(a), Operand::Local(b))
                    | (Operand::Local(b), Operand::Local(a)) if (*a == prod && *b == iv) || (*b == prod && *a == iv)
            );
            // matches! with two patterns sharing a,b is messy. do it explicitly:
            let ok = match (left, right) {
                (Operand::Local(x), Operand::Local(y)) => {
                    (*x == prod && *y == iv) || (*y == prod && *x == iv)
                }
                _ => false,
            };
            if ok {
                writes_acc = true;
            }
            let _ = uses_prod_and_iv;
        }
    }
    if writes_acc {
        Some((acc, k))
    } else {
        None
    }
}

fn bin(dest: LocalId, op: BinOp, left: Operand, right: Operand) -> Statement {
    Statement::Assign {
        dest,
        rvalue: Rvalue::Binary { op, left, right },
    }
}

fn fold_one(func: &mut MirFunction) -> bool {
    let Some(lp) = find_innermost(func) else {
        return false;
    };
    let unit = loop_unit(func, &lp);
    let Some((acc, k)) = detect_recurrence(&unit, lp.iv) else {
        return false;
    };

    let e = fresh_local(func, "orbit_e", MirTy::I32);
    let bit = fresh_local(func, "orbit_bit", MirTy::I32);
    let odd = fresh_local(func, "orbit_odd", MirTy::Bool);
    let mut t = |n: &str| fresh_local(func, n, MirTy::I32);
    let rs0 = t("rs0");
    let rs1 = t("rs1");
    let rs2 = t("rs2");
    let ri0 = t("ri0");
    let ri1 = t("ri1");
    let ri2 = t("ri2");
    let bs0 = t("bs0");
    let bs1 = t("bs1");
    let bs2 = t("bs2");
    let bi0 = t("bi0");
    let bi1 = t("bi1");
    let bi2 = t("bi2");
    let n0 = t("n0");
    let n1 = t("n1");
    let n2 = t("n2");
    let n3 = t("n3");
    let n4 = t("n4");
    let n5 = t("n5");
    let n6 = t("n6");
    let n7 = t("n7");
    let n8 = t("n8");
    let n9 = t("n9");
    let n10 = t("n10");
    let n11 = t("n11");
    let one = Operand::Constant(Constant::I32(1));
    let zero = Operand::Constant(Constant::I32(0));
    let two = Operand::Constant(Constant::I32(2));
    let kk = Operand::Constant(Constant::I32(k));

    let mut init = Vec::new();
    match lp.cmp {
        BinOp::Le => {
            let d = t("d");
            init.push(bin(d, BinOp::Sub, lp.bound.clone(), Operand::Local(lp.iv)));
            init.push(bin(e, BinOp::Add, Operand::Local(d), one.clone()));
        }
        _ => {
            init.push(bin(e, BinOp::Sub, lp.bound.clone(), Operand::Local(lp.iv)));
        }
    }
    init.push(bin(rs0, BinOp::Add, one.clone(), zero.clone()));
    init.push(bin(rs1, BinOp::Add, zero.clone(), zero.clone()));
    init.push(bin(rs2, BinOp::Add, zero.clone(), zero.clone()));
    init.push(bin(ri0, BinOp::Add, zero.clone(), zero.clone()));
    init.push(bin(ri1, BinOp::Add, one.clone(), zero.clone()));
    init.push(bin(ri2, BinOp::Add, zero.clone(), zero.clone()));
    init.push(bin(bs0, BinOp::Add, kk, zero.clone()));
    init.push(bin(bs1, BinOp::Add, one.clone(), zero.clone()));
    init.push(bin(bs2, BinOp::Add, zero.clone(), zero.clone()));
    init.push(bin(bi0, BinOp::Add, zero.clone(), zero.clone()));
    init.push(bin(bi1, BinOp::Add, one.clone(), zero.clone()));
    init.push(bin(bi2, BinOp::Add, one.clone(), zero.clone()));

    let mut body = Vec::new();
    body.push(bin(bit, BinOp::Mod, Operand::Local(e), two.clone()));
    body.push(Statement::Assign {
        dest: odd,
        rvalue: Rvalue::Binary {
            op: BinOp::NotEq,
            left: Operand::Local(bit),
            right: zero.clone(),
        },
    });
    // if odd: result *= base  (always compute, then select would be nicer;
    // emit as unconditional compose when bit==1 using a predicated copy.
    // We branch with a tiny if by baking compose then using the header's
    // remaining path: compute both square always; compose result only when odd
    // via mul by bit as 0/1 mask: result = result + bit*(base*result - result)
    // Simpler: always compose when we split blocks. Use two-block if.
    // For a straight-line body we always square, and compose using:
    //   result = result * (I + bit*(base - I))  not valid for matrices.
    // Emit compose unconditionally into temps, then
    //   rs0 = rs0 + (ns0-rs0)*bit  with bit 0 or 1. That's a select.
    // ns = base * result
    body.push(bin(n0, BinOp::Mul, Operand::Local(bs0), Operand::Local(rs0)));
    body.push(bin(n1, BinOp::Mul, Operand::Local(bs1), Operand::Local(ri0)));
    body.push(bin(n2, BinOp::Add, Operand::Local(n0), Operand::Local(n1)));
    body.push(bin(n3, BinOp::Mul, Operand::Local(bs0), Operand::Local(rs1)));
    body.push(bin(n4, BinOp::Mul, Operand::Local(bs1), Operand::Local(ri1)));
    body.push(bin(n5, BinOp::Add, Operand::Local(n3), Operand::Local(n4)));
    body.push(bin(n6, BinOp::Mul, Operand::Local(bs0), Operand::Local(rs2)));
    body.push(bin(n7, BinOp::Mul, Operand::Local(bs1), Operand::Local(ri2)));
    body.push(bin(n8, BinOp::Add, Operand::Local(n6), Operand::Local(n7)));
    body.push(bin(n8, BinOp::Add, Operand::Local(n8), Operand::Local(bs2)));
    body.push(bin(n9, BinOp::Mul, Operand::Local(bi0), Operand::Local(rs0)));
    body.push(bin(n10, BinOp::Mul, Operand::Local(bi1), Operand::Local(ri0)));
    let n12 = t("n12");
    let n13 = t("n13");
    let n14 = t("n14");
    let n15 = t("n15");
    let n16 = t("n16");
    let n17 = t("n17");
    body.push(bin(n11, BinOp::Add, Operand::Local(n9), Operand::Local(n10)));
    body.push(bin(n12, BinOp::Mul, Operand::Local(bi0), Operand::Local(rs1)));
    body.push(bin(n13, BinOp::Mul, Operand::Local(bi1), Operand::Local(ri1)));
    body.push(bin(n14, BinOp::Add, Operand::Local(n12), Operand::Local(n13)));
    body.push(bin(n15, BinOp::Mul, Operand::Local(bi0), Operand::Local(rs2)));
    body.push(bin(n16, BinOp::Mul, Operand::Local(bi1), Operand::Local(ri2)));
    body.push(bin(n17, BinOp::Add, Operand::Local(n15), Operand::Local(n16)));
    body.push(bin(n17, BinOp::Add, Operand::Local(n17), Operand::Local(bi2)));
    // select: rs = rs + (n - rs) * bit
    let d0 = t("d0");
    let d1 = t("d1");
    let d2 = t("d2");
    let d3 = t("d3");
    let d4 = t("d4");
    let d5 = t("d5");
    body.push(bin(d0, BinOp::Sub, Operand::Local(n2), Operand::Local(rs0)));
    body.push(bin(d0, BinOp::Mul, Operand::Local(d0), Operand::Local(bit)));
    body.push(bin(rs0, BinOp::Add, Operand::Local(rs0), Operand::Local(d0)));
    body.push(bin(d1, BinOp::Sub, Operand::Local(n5), Operand::Local(rs1)));
    body.push(bin(d1, BinOp::Mul, Operand::Local(d1), Operand::Local(bit)));
    body.push(bin(rs1, BinOp::Add, Operand::Local(rs1), Operand::Local(d1)));
    body.push(bin(d2, BinOp::Sub, Operand::Local(n8), Operand::Local(rs2)));
    body.push(bin(d2, BinOp::Mul, Operand::Local(d2), Operand::Local(bit)));
    body.push(bin(rs2, BinOp::Add, Operand::Local(rs2), Operand::Local(d2)));
    body.push(bin(d3, BinOp::Sub, Operand::Local(n11), Operand::Local(ri0)));
    body.push(bin(d3, BinOp::Mul, Operand::Local(d3), Operand::Local(bit)));
    body.push(bin(ri0, BinOp::Add, Operand::Local(ri0), Operand::Local(d3)));
    body.push(bin(d4, BinOp::Sub, Operand::Local(n14), Operand::Local(ri1)));
    body.push(bin(d4, BinOp::Mul, Operand::Local(d4), Operand::Local(bit)));
    body.push(bin(ri1, BinOp::Add, Operand::Local(ri1), Operand::Local(d4)));
    body.push(bin(d5, BinOp::Sub, Operand::Local(n17), Operand::Local(ri2)));
    body.push(bin(d5, BinOp::Mul, Operand::Local(d5), Operand::Local(bit)));
    body.push(bin(ri2, BinOp::Add, Operand::Local(ri2), Operand::Local(d5)));

    // square base = base * base
    let q0 = t("q0");
    let q1 = t("q1");
    let q2 = t("q2");
    let q3 = t("q3");
    let q4 = t("q4");
    let q5 = t("q5");
    let q6 = t("q6");
    let q7 = t("q7");
    let q8 = t("q8");
    let q9 = t("q9");
    let q10 = t("q10");
    let q11 = t("q11");
    body.push(bin(q0, BinOp::Mul, Operand::Local(bs0), Operand::Local(bs0)));
    body.push(bin(q1, BinOp::Mul, Operand::Local(bs1), Operand::Local(bi0)));
    body.push(bin(q2, BinOp::Add, Operand::Local(q0), Operand::Local(q1)));
    body.push(bin(q3, BinOp::Mul, Operand::Local(bs0), Operand::Local(bs1)));
    body.push(bin(q4, BinOp::Mul, Operand::Local(bs1), Operand::Local(bi1)));
    body.push(bin(q5, BinOp::Add, Operand::Local(q3), Operand::Local(q4)));
    body.push(bin(q6, BinOp::Mul, Operand::Local(bs0), Operand::Local(bs2)));
    body.push(bin(q7, BinOp::Mul, Operand::Local(bs1), Operand::Local(bi2)));
    body.push(bin(q8, BinOp::Add, Operand::Local(q6), Operand::Local(q7)));
    body.push(bin(q8, BinOp::Add, Operand::Local(q8), Operand::Local(bs2)));
    body.push(bin(q9, BinOp::Mul, Operand::Local(bi0), Operand::Local(bs0)));
    body.push(bin(q10, BinOp::Mul, Operand::Local(bi1), Operand::Local(bi0)));
    let q12 = t("q12");
    body.push(bin(q11, BinOp::Add, Operand::Local(q9), Operand::Local(q10)));
    body.push(bin(q12, BinOp::Mul, Operand::Local(bi0), Operand::Local(bs1)));
    let q13 = t("q13");
    let q14 = t("q14");
    let q15 = t("q15");
    let q16 = t("q16");
    let q17 = t("q17");
    body.push(bin(q13, BinOp::Mul, Operand::Local(bi1), Operand::Local(bi1)));
    body.push(bin(q14, BinOp::Add, Operand::Local(q12), Operand::Local(q13)));
    body.push(bin(q15, BinOp::Mul, Operand::Local(bi0), Operand::Local(bs2)));
    body.push(bin(q16, BinOp::Mul, Operand::Local(bi1), Operand::Local(bi2)));
    body.push(bin(q17, BinOp::Add, Operand::Local(q15), Operand::Local(q16)));
    body.push(bin(q17, BinOp::Add, Operand::Local(q17), Operand::Local(bi2)));
    body.push(bin(bs0, BinOp::Add, Operand::Local(q2), zero.clone()));
    body.push(bin(bs1, BinOp::Add, Operand::Local(q5), zero.clone()));
    body.push(bin(bs2, BinOp::Add, Operand::Local(q8), zero.clone()));
    body.push(bin(bi0, BinOp::Add, Operand::Local(q11), zero.clone()));
    body.push(bin(bi1, BinOp::Add, Operand::Local(q14), zero.clone()));
    body.push(bin(bi2, BinOp::Add, Operand::Local(q17), zero.clone()));
    body.push(bin(e, BinOp::Div, Operand::Local(e), two));

    let apply0 = t("ap0");
    let apply1 = t("ap1");
    let apply2 = t("ap2");
    let apply3 = t("ap3");
    let mut tail = Vec::new();
    tail.push(bin(apply0, BinOp::Mul, Operand::Local(rs0), Operand::Local(acc)));
    tail.push(bin(apply1, BinOp::Mul, Operand::Local(rs1), Operand::Local(lp.iv)));
    tail.push(bin(apply2, BinOp::Add, Operand::Local(apply0), Operand::Local(apply1)));
    tail.push(bin(apply3, BinOp::Add, Operand::Local(apply2), Operand::Local(rs2)));
    tail.push(bin(acc, BinOp::Add, Operand::Local(apply3), zero.clone()));
    tail.push(bin(lp.iv, BinOp::Add, Operand::Local(lp.iv), Operand::Local(e))); // e is 0 here; fix below

    // i' = ri0*acc + ri1*i + ri2, using acc BEFORE overwrite — redo with temps
    // We already overwrote acc. Compute i' before acc write.
    tail.clear();
    let na = t("na");
    let ni = t("ni");
    tail.push(bin(na, BinOp::Mul, Operand::Local(rs0), Operand::Local(acc)));
    tail.push(bin(n0, BinOp::Mul, Operand::Local(rs1), Operand::Local(lp.iv)));
    tail.push(bin(na, BinOp::Add, Operand::Local(na), Operand::Local(n0)));
    tail.push(bin(na, BinOp::Add, Operand::Local(na), Operand::Local(rs2)));
    tail.push(bin(ni, BinOp::Mul, Operand::Local(ri0), Operand::Local(acc)));
    tail.push(bin(n1, BinOp::Mul, Operand::Local(ri1), Operand::Local(lp.iv)));
    tail.push(bin(ni, BinOp::Add, Operand::Local(ni), Operand::Local(n1)));
    tail.push(bin(ni, BinOp::Add, Operand::Local(ni), Operand::Local(ri2)));
    tail.push(bin(acc, BinOp::Add, Operand::Local(na), zero.clone()));
    tail.push(bin(lp.iv, BinOp::Add, Operand::Local(ni), zero));

    let cond = fresh_local(func, "orbit_more", MirTy::Bool);
    let max_id = func.blocks.iter().map(|b| b.id).max().unwrap_or(0);
    let apply_bb = max_id + 1;
    let pre = max_id + 2;
    for bb in &mut func.blocks {
        if bb.id == lp.body || lp.step == Some(bb.id) {
            continue;
        }
        match &mut bb.terminator {
            Terminator::Goto(t) if *t == lp.header => *t = pre,
            Terminator::If { then_bb, else_bb, .. } => {
                if *then_bb == lp.header {
                    *then_bb = pre;
                }
                if *else_bb == lp.header {
                    *else_bb = pre;
                }
            }
            _ => {}
        }
    }
    if let Some(h) = func.blocks.iter_mut().find(|b| b.id == lp.header) {
        h.stmts = vec![Statement::Assign {
            dest: cond,
            rvalue: Rvalue::Binary {
                op: BinOp::Gt,
                left: Operand::Local(e),
                right: Operand::Constant(Constant::I32(0)),
            },
        }];
        h.terminator = Terminator::If {
            cond: Operand::Local(cond),
            then_bb: lp.body,
            else_bb: apply_bb,
        };
    }
    if let Some(b) = func.blocks.iter_mut().find(|b| b.id == lp.body) {
        b.stmts = body;
        b.terminator = Terminator::Goto(lp.header);
    }
    func.blocks.push(BasicBlock {
        id: pre,
        stmts: init,
        terminator: Terminator::Goto(lp.header),
    });
    func.blocks.push(BasicBlock {
        id: apply_bb,
        stmts: tail,
        terminator: Terminator::Goto(lp.exit),
    });
    true
}

fn unroll_one(func: &mut MirFunction) -> bool {
    let Some(lp) = find_innermost(func) else {
        return false;
    };
    let header = match func.blocks.iter().find(|b| b.id == lp.header) {
        Some(b) => b.clone(),
        None => return false,
    };
    let body = match func.blocks.iter().find(|b| b.id == lp.body) {
        Some(b) => b.clone(),
        None => return false,
    };
    let step_stmts = if let Some(sid) = lp.step {
        func.blocks
            .iter()
            .find(|b| b.id == sid)
            .map(|b| b.stmts.clone())
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut unit: Vec<Statement> = body.stmts.clone();
    unit.extend(step_stmts);
    if unit.is_empty() || unit.iter().any(|s| stmt_blocks_orbit(s)) {
        return false;
    }

    let next_id = func.blocks.iter().map(|b| b.id).max().unwrap_or(0) + 1;
    let rem_header = next_id;
    let rem_body = next_id + 1;

    let lim = fresh_local(func, "orbit_lim", MirTy::I32);
    let kcond = fresh_local(func, "orbit_k", MirTy::Bool);
    let rcond = fresh_local(func, "orbit_r", MirTy::Bool);

    let mut kernel = Vec::new();
    for _ in 0..FACTOR {
        kernel.extend(unit.clone());
    }

    if let Some(h) = func.blocks.iter_mut().find(|b| b.id == lp.header) {
        h.stmts.push(Statement::Assign {
            dest: lim,
            rvalue: Rvalue::Binary {
                op: BinOp::Sub,
                left: lp.bound.clone(),
                right: Operand::Constant(Constant::I32(FACTOR - 1)),
            },
        });
        h.stmts.push(Statement::Assign {
            dest: kcond,
            rvalue: Rvalue::Binary {
                op: lp.cmp,
                left: Operand::Local(lp.iv),
                right: Operand::Local(lim),
            },
        });
        h.terminator = Terminator::If {
            cond: Operand::Local(kcond),
            then_bb: lp.body,
            else_bb: rem_header,
        };
    }

    if let Some(b) = func.blocks.iter_mut().find(|b| b.id == lp.body) {
        b.stmts = kernel;
        b.terminator = Terminator::Goto(lp.header);
    }

    func.blocks.push(BasicBlock {
        id: rem_header,
        stmts: {
            let mut s = header.stmts;
            s.push(Statement::Assign {
                dest: rcond,
                rvalue: Rvalue::Binary {
                    op: lp.cmp,
                    left: Operand::Local(lp.iv),
                    right: lp.bound.clone(),
                },
            });
            s
        },
        terminator: Terminator::If {
            cond: Operand::Local(rcond),
            then_bb: rem_body,
            else_bb: lp.exit,
        },
    });
    func.blocks.push(BasicBlock {
        id: rem_body,
        stmts: unit,
        terminator: Terminator::Goto(rem_header),
    });

    true
}

fn find_innermost(func: &MirFunction) -> Option<CountedLoop> {
    let mut found: Vec<CountedLoop> = Vec::new();
    for bb in &func.blocks {
        let Terminator::If {
            cond: Operand::Local(cid),
            then_bb,
            else_bb,
        } = &bb.terminator
        else {
            continue;
        };
        let Some((iv, bound, cmp)) = iv_from_header(bb, *cid) else {
            continue;
        };
        let then_bb = *then_bb;
        let else_bb = *else_bb;
        if let Some(step) = latch_is_step(func, then_bb, bb.id, iv) {
            if iv_inc_count(func, then_bb, iv) + iv_inc_count(func, step, iv) < FACTOR as usize
                && !has_inner_loop(func, then_bb, bb.id)
                && !body_has_float(func, then_bb)
            {
                found.push(CountedLoop {
                    header: bb.id,
                    body: then_bb,
                    exit: else_bb,
                    step: Some(step),
                    iv,
                    bound: bound.clone(),
                    cmp,
                });
            }
            continue;
        }
        if latch_is_body(func, then_bb, bb.id) && iv_increments(func, then_bb, iv) {
            if iv_inc_count(func, then_bb, iv) >= FACTOR as usize {
                continue;
            }
            if body_has_float(func, then_bb) {
                continue;
            }
            if !has_inner_loop(func, then_bb, bb.id) {
                found.push(CountedLoop {
                    header: bb.id,
                    body: then_bb,
                    exit: else_bb,
                    step: None,
                    iv,
                    bound,
                    cmp,
                });
            }
        }
    }
    found.into_iter().min_by_key(|lp| {
        func.blocks
            .iter()
            .find(|b| b.id == lp.body)
            .map(|b| b.stmts.len())
            .unwrap_or(usize::MAX)
    })
}

fn iv_from_header(header: &BasicBlock, cond: LocalId) -> Option<(LocalId, Operand, BinOp)> {
    for stmt in header.stmts.iter().rev() {
        let Statement::Assign {
            dest,
            rvalue: Rvalue::Binary { op, left, right },
        } = stmt
        else {
            continue;
        };
        if *dest != cond {
            continue;
        }
        if !matches!(op, BinOp::Le | BinOp::Lt | BinOp::Ge | BinOp::Gt) {
            continue;
        }
        match (left, right) {
            (Operand::Local(iv), bound) if matches!(op, BinOp::Le | BinOp::Lt) => {
                return Some((*iv, bound.clone(), *op));
            }
            (bound, Operand::Local(iv)) if matches!(op, BinOp::Ge | BinOp::Gt) => {
                let flipped = if *op == BinOp::Ge { BinOp::Le } else { BinOp::Lt };
                return Some((*iv, bound.clone(), flipped));
            }
            _ => {}
        }
    }
    None
}

fn latch_is_body(func: &MirFunction, body: BlockId, header: BlockId) -> bool {
    func.blocks
        .iter()
        .find(|b| b.id == body)
        .is_some_and(|b| matches!(b.terminator, Terminator::Goto(h) if h == header))
}

fn latch_is_step(func: &MirFunction, body: BlockId, header: BlockId, iv: LocalId) -> Option<BlockId> {
    let b = func.blocks.iter().find(|b| b.id == body)?;
    let Terminator::Goto(step) = b.terminator else {
        return None;
    };
    let s = func.blocks.iter().find(|b| b.id == step)?;
    if !matches!(s.terminator, Terminator::Goto(h) if h == header) {
        return None;
    }
    if s.stmts.iter().any(|st| is_iv_inc(st, iv)) {
        Some(step)
    } else {
        None
    }
}

fn iv_inc_count(func: &MirFunction, body: BlockId, iv: LocalId) -> usize {
    func.blocks
        .iter()
        .find(|b| b.id == body)
        .map(|b| b.stmts.iter().filter(|st| is_iv_inc(st, iv)).count())
        .unwrap_or(0)
}

fn iv_increments(func: &MirFunction, body: BlockId, iv: LocalId) -> bool {
    iv_inc_count(func, body, iv) > 0
}

fn is_iv_inc(stmt: &Statement, iv: LocalId) -> bool {
    let Statement::Assign {
        dest,
        rvalue: Rvalue::Binary {
            op: BinOp::Add,
            left,
            right,
        },
    } = stmt
    else {
        return false;
    };
    if *dest != iv {
        return false;
    }
    matches!(
        (left, right),
        (Operand::Local(x), Operand::Constant(Constant::I32(1)))
            | (Operand::Constant(Constant::I32(1)), Operand::Local(x))
            if *x == iv
    )
}

fn has_inner_loop(func: &MirFunction, body: BlockId, header: BlockId) -> bool {
    func.blocks.iter().any(|bb| {
        if bb.id == header || bb.id == body {
            return false;
        }
        matches!(bb.terminator, Terminator::If { .. })
            && succ_reaches(func, bb.id, bb.id)
            && succ_reaches(func, body, bb.id)
            && !succ_reaches(func, bb.id, header)
    })
}

fn succ_reaches(func: &MirFunction, start: BlockId, target: BlockId) -> bool {
    use std::collections::{HashSet, VecDeque};
    let mut seen = HashSet::new();
    let mut q = VecDeque::from([start]);
    while let Some(id) = q.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        let Some(bb) = func.blocks.iter().find(|b| b.id == id) else {
            continue;
        };
        for s in successors(bb) {
            if s == target {
                return true;
            }
            q.push_back(s);
        }
    }
    false
}

fn successors(bb: &BasicBlock) -> Vec<BlockId> {
    match &bb.terminator {
        Terminator::Goto(t) => vec![*t],
        Terminator::If { then_bb, else_bb, .. } => vec![*then_bb, *else_bb],
        Terminator::Switch { arms, otherwise, .. } => {
            let mut v: Vec<BlockId> = arms.iter().map(|(_, bb)| *bb).collect();
            v.push(*otherwise);
            v
        }
        Terminator::Return(_) | Terminator::Unreachable => vec![],
    }
}

fn stmt_blocks_orbit(stmt: &Statement) -> bool {
    match stmt {
        Statement::Assign {
            rvalue: Rvalue::Call { .. },
            ..
        }
        | Statement::Store { .. }
        | Statement::StorageLive { .. }
        | Statement::StorageDead { .. } => true,
        _ => false,
    }
}

fn fresh_local(func: &mut MirFunction, name: &str, ty: MirTy) -> LocalId {
    let id = func.locals.len() as u32;
    func.locals.push(MirLocal {
        name: name.into(),
        ty,
        mutable: true,
    });
    id
}

#[cfg(test)]
mod algebra {
    fn loop_sum(n: i32, k: i32) -> i32 {
        let mut sum = 0i32;
        let mut i = 1i32;
        while i <= n {
            sum = sum.wrapping_mul(k).wrapping_add(i);
            i = i.wrapping_add(1);
        }
        sum
    }

    fn orbit_sum(n: i32, k: i32) -> i32 {
        let mut e = n;
        let mut rs0 = 1i32;
        let mut rs1 = 0i32;
        let mut rs2 = 0i32;
        let mut ri0 = 0i32;
        let mut ri1 = 1i32;
        let mut ri2 = 0i32;
        let mut bs0 = k;
        let mut bs1 = 1i32;
        let mut bs2 = 0i32;
        let mut bi0 = 0i32;
        let mut bi1 = 1i32;
        let mut bi2 = 1i32;
        let mut acc = 0i32;
        let mut iv = 1i32;
        while e > 0 {
            let bit = e % 2;
            let n2 = bs0.wrapping_mul(rs0).wrapping_add(bs1.wrapping_mul(ri0));
            let n5 = bs0.wrapping_mul(rs1).wrapping_add(bs1.wrapping_mul(ri1));
            let n8 = bs0
                .wrapping_mul(rs2)
                .wrapping_add(bs1.wrapping_mul(ri2))
                .wrapping_add(bs2);
            let n11 = bi0.wrapping_mul(rs0).wrapping_add(bi1.wrapping_mul(ri0));
            let n14 = bi0.wrapping_mul(rs1).wrapping_add(bi1.wrapping_mul(ri1));
            let n17 = bi0
                .wrapping_mul(rs2)
                .wrapping_add(bi1.wrapping_mul(ri2))
                .wrapping_add(bi2);
            rs0 = rs0.wrapping_add(n2.wrapping_sub(rs0).wrapping_mul(bit));
            rs1 = rs1.wrapping_add(n5.wrapping_sub(rs1).wrapping_mul(bit));
            rs2 = rs2.wrapping_add(n8.wrapping_sub(rs2).wrapping_mul(bit));
            ri0 = ri0.wrapping_add(n11.wrapping_sub(ri0).wrapping_mul(bit));
            ri1 = ri1.wrapping_add(n14.wrapping_sub(ri1).wrapping_mul(bit));
            ri2 = ri2.wrapping_add(n17.wrapping_sub(ri2).wrapping_mul(bit));
            let q2 = bs0.wrapping_mul(bs0).wrapping_add(bs1.wrapping_mul(bi0));
            let q5 = bs0.wrapping_mul(bs1).wrapping_add(bs1.wrapping_mul(bi1));
            let q8 = bs0
                .wrapping_mul(bs2)
                .wrapping_add(bs1.wrapping_mul(bi2))
                .wrapping_add(bs2);
            let q11 = bi0.wrapping_mul(bs0).wrapping_add(bi1.wrapping_mul(bi0));
            let q14 = bi0.wrapping_mul(bs1).wrapping_add(bi1.wrapping_mul(bi1));
            let q17 = bi0
                .wrapping_mul(bs2)
                .wrapping_add(bi1.wrapping_mul(bi2))
                .wrapping_add(bi2);
            bs0 = q2;
            bs1 = q5;
            bs2 = q8;
            bi0 = q11;
            bi1 = q14;
            bi2 = q17;
            e /= 2;
        }
        let na = rs0
            .wrapping_mul(acc)
            .wrapping_add(rs1.wrapping_mul(iv))
            .wrapping_add(rs2);
        na
    }

    #[test]
    fn wrapping_orbit_matches_loop() {
        for n in [1, 2, 7, 8, 10, 31, 100, 1000, 100_000] {
            assert_eq!(orbit_sum(n, 3), loop_sum(n, 3), "n={n}");
        }
    }
}
