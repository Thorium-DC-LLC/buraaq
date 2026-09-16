//! MIR optimization passes — measured, conservative, correctness-first.

mod const_fold;
mod dce;
mod inline;
mod orbit;
mod simplify_cfg;

use crate::MirModule;

pub use const_fold::ConstFold;
pub use dce::DeadCodeElim;
pub use inline::InlineSmallCalls;
pub use orbit::OrbitUnroll;
pub use simplify_cfg::SimplifyCfg;

/// Statistics from one optimization pipeline run (for benchmarks and tuning).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct OptStats {
    pub const_folds: usize,
    pub dead_stmts_removed: usize,
    pub blocks_removed: usize,
    pub calls_inlined: usize,
    pub passes_run: usize,
}

impl OptStats {
    pub fn total_changes(&self) -> usize {
        self.const_folds + self.dead_stmts_removed + self.blocks_removed + self.calls_inlined
    }
}

#[derive(Clone, Debug)]
pub struct OptConfig {
    pub const_fold: bool,
    pub inline: bool,
    pub dce: bool,
    pub simplify_cfg: bool,
    pub max_inline_stmts: usize,
}

impl Default for OptConfig {
    fn default() -> Self {
        Self {
            const_fold: true,
            inline: true,
            dce: true,
            simplify_cfg: true,
            max_inline_stmts: 12,
        }
    }
}

impl OptConfig {
    pub fn debug() -> Self {
        Self {
            const_fold: false,
            inline: false,
            dce: false,
            simplify_cfg: false,
            max_inline_stmts: 0,
        }
    }
}

/// Run MIR optimizations in documented order. Returns stats for measurement.
pub fn optimize(mut module: MirModule, config: &OptConfig) -> (MirModule, OptStats) {
    let mut stats = OptStats::default();

    if config.const_fold {
        stats.const_folds = ConstFold::new().run(&mut module);
        stats.passes_run += 1;
    }

    if config.simplify_cfg {
        stats.blocks_removed += SimplifyCfg::new().run(&mut module);
        stats.passes_run += 1;
    }

    if config.inline {
        stats.calls_inlined = InlineSmallCalls::new(config.max_inline_stmts).run(&mut module);
        stats.passes_run += 1;
        if config.const_fold {
            stats.const_folds += ConstFold::new().run(&mut module);
        }
    }

    if config.simplify_cfg {
        let _ = OrbitUnroll::new().run(&mut module);
        stats.passes_run += 1;
    }

    if config.dce {
        stats.dead_stmts_removed = DeadCodeElim::new().run(&mut module);
        stats.passes_run += 1;
    }

    if config.simplify_cfg {
        stats.blocks_removed += SimplifyCfg::new().run(&mut module);
        stats.passes_run += 1;
    }

    (module, stats)
}
