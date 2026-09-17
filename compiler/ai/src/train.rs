use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::doctor::probe_host;
use crate::model::inspect_model;
use crate::{AiError, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct TrainConfig {
    pub model: String,
    pub data: String,
    #[serde(default = "default_method")]
    pub method: String,
}

fn default_method() -> String {
    "lora".into()
}

#[derive(Debug, Clone)]
pub struct TrainPlan {
    pub config: TrainConfig,
    pub method: String,
    pub batch_size: u32,
    pub grad_accum: u32,
    pub context: u32,
    pub estimated_vram_gb: f64,
    pub gpu_name: Option<String>,
}

pub fn load_config(path: Option<&Path>) -> Result<TrainConfig> {
    let candidates: Vec<std::path::PathBuf> = if let Some(p) = path {
        vec![p.to_path_buf()]
    } else {
        vec![
            std::path::PathBuf::from("buraaq.ai.toml"),
            std::path::PathBuf::from("ai.toml"),
        ]
    };
    for p in &candidates {
        if p.exists() {
            let raw = fs::read_to_string(p)?;
            let model = find_str(&raw, "model").unwrap_or_else(|| "Qwen/Qwen3-8B".into());
            let data = find_str(&raw, "data").unwrap_or_else(|| "train.jsonl".into());
            let method = find_str(&raw, "method").unwrap_or_else(|| "lora".into());
            return Ok(TrainConfig { model, data, method });
        }
    }
    if path.is_some() {
        return Err(AiError::msg("train config not found"));
    }
    Ok(TrainConfig {
        model: "Qwen/Qwen3-8B".into(),
        data: "train.jsonl".into(),
        method: "lora".into(),
    })
}

pub fn compute_train_plan(cfg: &TrainConfig) -> Result<TrainPlan> {
    let model = inspect_model(&cfg.model)?;
    let host = probe_host();
    let vram: f64 = host.gpus.iter().map(|g| g.vram_gb).sum();

    if host.gpus.is_empty() {
        return Ok(TrainPlan {
            config: cfg.clone(),
            method: "qlora".into(),
            batch_size: 1,
            grad_accum: 16,
            context: 2048,
            estimated_vram_gb: 0.0,
            gpu_name: None,
        });
    }
    let g0 = &host.gpus[0];
    let full_ok = model.vram_gb(2.0) * 3.0 < vram;
    let method = if cfg.method == "full" && full_ok {
        "full"
    } else if g0.vram_gb < 16.0 || model.params_b >= 7.0 {
        "qlora"
    } else if cfg.method == "qlora" {
        "qlora"
    } else {
        "lora"
    };
    let (batch, accum, ctx, est) = match method {
        "full" => (1, 8, 2048, model.vram_gb(2.0) * 2.5),
        "lora" => (2, 8, 4096, model.vram_gb(2.0) * 0.55 + 2.0),
        _ => (2, 8, 4096, model.vram_gb(0.5) + 4.0),
    };
    Ok(TrainPlan {
        config: cfg.clone(),
        method: method.into(),
        batch_size: batch,
        grad_accum: accum,
        context: ctx,
        estimated_vram_gb: est,
        gpu_name: Some(g0.name.clone()),
    })
}

pub fn print_train_plan(plan: &TrainPlan, cfg_path: Option<&Path>) {
    println!("Buraaq AI train");
    println!();
    println!("Model:  {}", plan.config.model);
    println!("Data:   {}", plan.config.data);
    println!("Method request: {}", plan.config.method);
    println!();
    match &plan.gpu_name {
        None => {
            println!("Detected: CPU only");
            println!("Full fine-tuning: not recommended");
        }
        Some(name) => {
            println!("Detected: {name}");
            if plan.method == "full" {
                println!("Full fine-tuning: selected");
            } else {
                println!("Full fine-tuning: not recommended");
            }
        }
    }
    println!();
    println!("Selected:");
    println!("  Method: {}", plan.method);
    if plan.method == "qlora" {
        println!("  Quantization: 4-bit NF4");
    }
    println!("  Batch size: {}", plan.batch_size);
    println!("  Gradient accumulation: {}", plan.grad_accum);
    println!("  Context: {}", plan.context);
    if plan.estimated_vram_gb > 0.0 {
        println!("  Estimated VRAM: {:.1} GB", plan.estimated_vram_gb);
    } else {
        println!("  Estimated VRAM: n/a (CPU)");
    }
    println!();
    println!(
        "Config: {}",
        cfg_path
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "buraaq.ai.toml / defaults".into())
    );
    if crate::train_runner::train_ready() {
        println!("Train env: ready ({})", crate::train_runner::venv_root().display());
    } else {
        println!("Train env: missing — run: buraaq ai doctor --fix");
    }
}

/// Phase 1 compatibility: print plan only.
pub fn plan_train(cfg_path: Option<&Path>) -> Result<()> {
    let cfg = load_config(cfg_path)?;
    let plan = compute_train_plan(&cfg)?;
    print_train_plan(&plan, cfg_path);
    println!();
    println!("Tip: buraaq ai train --yes   # execute after confirm");
    println!("     buraaq ai train --plan-only");
    Ok(())
}

fn find_str(raw: &str, key: &str) -> Option<String> {
    for line in raw.lines() {
        let line = line.trim();
        if line.starts_with('#') {
            continue;
        }
        if let Some(rest) = line.strip_prefix(key) {
            let rest = rest.trim().trim_start_matches('=').trim();
            let v = rest.trim_matches('"').trim_matches('\'').to_string();
            if !v.is_empty() {
                return Some(v);
            }
        }
    }
    None
}
