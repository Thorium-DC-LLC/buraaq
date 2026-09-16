use std::collections::{HashSet, VecDeque};

use crate::{BasicBlock, MirModule, Terminator};

pub struct SimplifyCfg;

impl SimplifyCfg {
    pub fn new() -> Self {
        Self
    }

    pub fn run(&mut self, module: &mut MirModule) -> usize {
        let mut removed = 0;
        for func in &mut module.functions {
            if func.blocks.is_empty() {
                continue;
            }
            let reachable = reachable_blocks(func);
            let before = func.blocks.len();
            func.blocks.retain(|bb| reachable.contains(&bb.id));
            removed += before.saturating_sub(func.blocks.len());
        }
        removed
    }
}

fn reachable_blocks(func: &crate::MirFunction) -> HashSet<u32> {
    let mut seen = HashSet::new();
    if func.blocks.is_empty() {
        return seen;
    }
    let entry = func.blocks[0].id;
    let mut queue = VecDeque::from([entry]);
    while let Some(id) = queue.pop_front() {
        if !seen.insert(id) {
            continue;
        }
        if let Some(bb) = func.blocks.iter().find(|b| b.id == id) {
            for succ in successors(bb) {
                if !seen.contains(&succ) {
                    queue.push_back(succ);
                }
            }
        }
    }
    seen
}

fn successors(bb: &BasicBlock) -> Vec<u32> {
    match &bb.terminator {
        Terminator::Goto(t) => vec![*t],
        Terminator::If { then_bb, else_bb, .. } => vec![*then_bb, *else_bb],
        Terminator::Switch { arms, otherwise, .. } => {
            let mut v: Vec<u32> = arms.iter().map(|(_, bb)| *bb).collect();
            v.push(*otherwise);
            v
        }
        Terminator::Return(_) | Terminator::Unreachable => vec![],
    }
}
