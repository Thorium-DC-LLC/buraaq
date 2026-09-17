use std::env;
use std::fs;
use std::path::Path;

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::doctor::probe_host;
use crate::model::inspect_model;
use crate::plan::{build_plan, PlanRequest};
use crate::{AiError, Result};

/// Write `target/ai/manifest.json` — model hash + backend requirements, no weight duplication.
pub fn write_ai_manifest(model: &str, out_dir: &Path) -> Result<()> {
    let plan = build_plan(&PlanRequest {
        model: model.to_string(),
        backend: "auto".into(),
        quantize: None,
        context: None,
        gpus: "auto".into(),
        tensor_parallel: None,
        require_backend: false,
    })?;
    let info = inspect_model(model)?;
    fs::create_dir_all(out_dir)?;
    let min_vram = plan.model_vram_per_gpu_gb + plan.kv_per_gpu_gb;
    let adapter = latest_adapter_for(model);
    let require_gpu = env::var("BURAAQ_AI_REQUIRE_GPU")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    let mut hasher = Sha256::new();
    hasher.update(model.as_bytes());
    hasher.update(plan.backend.as_bytes());
    hasher.update(plan.quantize.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    let manifest = json!({
        "format": "buraaq.ai.manifest.v1",
        "model": model,
        "model_hash": info.path.as_ref().and_then(|p| {
            fs::read_to_string(p.join("SHA256")).ok().map(|s| s.trim().to_string())
        }),
        "plan_hash": hash,
        "backend": plan.backend,
        "recommended_backend": plan.backend,
        "quantize": plan.quantize,
        "context": plan.context,
        "tensor_parallel": plan.tensor_parallel,
        "min_vram_gb": (min_vram * 10.0).round() / 10.0,
        "require_gpu": require_gpu,
        "adapter_path": adapter,
        "weights": "external_cache",
        "cache_hint": crate::cache::cache_root().display().to_string(),
        "note": "Large weights stay content-addressed in the model cache; this manifest ships requirements only.",
    });
    let path = out_dir.join("manifest.json");
    fs::write(&path, serde_json::to_string_pretty(&manifest)?)?;
    fs::write(
        out_dir.join("GPU_CHECK.txt"),
        format!(
            "GPU required: {:.1} GB VRAM (approx)\nbackend: {}\nrequire_gpu: {require_gpu}\n",
            min_vram, plan.backend
        ),
    )?;
    println!("wrote {}", path.display());
    println!("  min_vram_gb: {:.1}", min_vram);
    println!("  recommended_backend: {}", plan.backend);
    if let Some(a) = &adapter {
        println!("  adapter_path: {a}");
    }
    Ok(())
}

fn latest_adapter_for(model: &str) -> Option<String> {
    let reg = crate::cache::cache_root()
        .parent()?
        .join("ai")
        .join("adapters.jsonl");
    let raw = fs::read_to_string(reg).ok()?;
    let mut last = None;
    for line in raw.lines().rev() {
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(line) {
            if v["model"].as_str() == Some(model) {
                last = v["adapter_dir"].as_str().map(|s| s.to_string());
                break;
            }
        }
    }
    last.or_else(|| {
        let dir = Path::new("target/ai/adapters");
        let rd = fs::read_dir(dir).ok()?;
        let mut dirs: Vec<_> = rd.filter_map(|e| e.ok()).map(|e| e.path()).collect();
        dirs.sort();
        dirs.pop().map(|p| p.display().to_string())
    })
}

/// Preflight + pack manifest; optionally push an existing .bur to Dock.
pub fn ai_ship(model: &str, host: &str) -> Result<()> {
    let out = Path::new("target/ai");
    write_ai_manifest(model, out)?;
    let raw = fs::read_to_string(out.join("manifest.json"))?;
    let man: serde_json::Value = serde_json::from_str(&raw)?;
    let min_vram = man["min_vram_gb"].as_f64().unwrap_or(0.0);
    let require_gpu = man["require_gpu"].as_bool().unwrap_or(false);
    let host_probe = probe_host();
    let best = host_probe
        .gpus
        .iter()
        .map(|g| g.vram_gb)
        .fold(0.0_f64, f64::max);

    println!();
    println!("AI ship preflight (local doctor — remote host must match or exceed):");
    println!("  GPU required: {min_vram:.1} GB");
    if host_probe.gpus.is_empty() {
        println!("  Local GPUs: none");
        if require_gpu
            || env::var("BURAAQ_AI_REQUIRE_GPU")
                .map(|v| v == "1")
                .unwrap_or(false)
        {
            return Err(AiError::msg(format!(
                "GPU required ({min_vram:.1} GB) but none detected locally.\nSet BURAAQ_AI_REQUIRE_GPU=0 to skip, or land a GPU host first."
            )));
        }
    } else {
        println!("  Local best VRAM: {best:.1} GB");
        if require_gpu && best + 0.1 < min_vram {
            return Err(AiError::msg(format!(
                "Local GPU ({best:.1} GB) below manifest min_vram_gb ({min_vram:.1}).\nUse a larger GPU host or lower quant via planner."
            )));
        }
    }

    // Push existing ship bundle if present.
    let ship_dir = Path::new("target/ship");
    let bur = if ship_dir.is_dir() {
        fs::read_dir(ship_dir)
            .ok()
            .and_then(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .find(|p| p.extension().and_then(|x| x.to_str()) == Some("bur"))
            })
    } else {
        None
    };

    println!();
    println!("Target Dock: {host}");
    println!("AI manifest stays external (weights not embedded).");
    if let Some(bur_path) = bur {
        println!("Found {} — push with: buraaq ship {host} --bundle {}", bur_path.display(), bur_path.display());
        println!("(ai ship does not re-pack the app binary; use buraaq pack + ship for the .bur)");
    } else {
        println!("No target/ship/*.bur yet.");
        println!("  1. buraaq pack");
        println!("  2. buraaq ship {host}");
        println!("  3. On host: buraaq ai serve {model}");
    }
    Ok(())
}
