use std::fs;

use serde::Deserialize;

use crate::cache::model_dir;
use crate::doctor::probe_host;
use crate::{sanitize_model_id, Result};

#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub architecture: String,
    pub params_b: f64,
    pub context: u32,
    pub layers: u32,
    pub hidden: u32,
    pub local: bool,
    pub path: Option<std::path::PathBuf>,
    pub license: Option<String>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct HfConfig {
    #[serde(default)]
    architectures: Vec<String>,
    #[serde(default)]
    model_type: String,
    #[serde(default)]
    num_hidden_layers: Option<u32>,
    #[serde(default)]
    n_layer: Option<u32>,
    #[serde(default)]
    hidden_size: Option<u32>,
    #[serde(default)]
    n_embd: Option<u32>,
    #[serde(default)]
    max_position_embeddings: Option<u32>,
    #[serde(default)]
    n_positions: Option<u32>,
    #[serde(default)]
    vocab_size: Option<u32>,
    #[serde(default)]
    num_attention_heads: Option<u32>,
    #[serde(default)]
    num_key_value_heads: Option<u32>,
}

pub fn inspect_model(id: &str) -> Result<ModelInfo> {
    let id = sanitize_model_id(id)?;
    let dir = model_dir(&id)?;
    let local = dir.join("config.json").exists();
    let mut info = heuristic_from_name(&id);
    info.local = local;
    if local {
        info.path = Some(dir.clone());
        if let Ok(raw) = fs::read_to_string(dir.join("config.json")) {
            if let Ok(cfg) = serde_json::from_str::<HfConfig>(&raw) {
                apply_hf(&mut info, &cfg);
            }
        }
        if let Ok(raw) = fs::read_to_string(dir.join("buraaq-model.json")) {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
                if let Some(lic) = v.get("license").and_then(|x| x.as_str()) {
                    info.license = Some(lic.to_string());
                }
            }
        }
    }
    Ok(info)
}

fn apply_hf(info: &mut ModelInfo, cfg: &HfConfig) {
    if let Some(a) = cfg.architectures.first() {
        info.architecture = a.clone();
    } else if !cfg.model_type.is_empty() {
        info.architecture = cfg.model_type.clone();
    }
    if let Some(l) = cfg.num_hidden_layers.or(cfg.n_layer) {
        info.layers = l;
    }
    if let Some(h) = cfg.hidden_size.or(cfg.n_embd) {
        info.hidden = h;
    }
    if let Some(c) = cfg.max_position_embeddings.or(cfg.n_positions) {
        info.context = c;
    }
    // Rough param estimate when not labeled: 12 * L * H^2 for transformers (very rough).
    if info.params_b <= 0.0 && info.layers > 0 && info.hidden > 0 {
        let p = 12.0 * info.layers as f64 * (info.hidden as f64).powi(2);
        info.params_b = p / 1e9;
    }
}

fn heuristic_from_name(id: &str) -> ModelInfo {
    let lower = id.to_ascii_lowercase();
    let params_b = parse_size_token(&lower).unwrap_or(7.0);
    ModelInfo {
        id: id.to_string(),
        architecture: "unknown".into(),
        params_b,
        context: 8192,
        layers: estimate_layers(params_b),
        hidden: estimate_hidden(params_b),
        local: false,
        path: None,
        license: None,
    }
}

fn parse_size_token(s: &str) -> Option<f64> {
    // Match 7B, 8b, 32B, 1.5B, 70B, etc.
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            while i < bytes.len() && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                i += 1;
            }
            if i < bytes.len() && (bytes[i] == b'b' || bytes[i] == b'B') {
                let num: f64 = s[start..i].parse().ok()?;
                if num > 0.0 && num < 2000.0 {
                    return Some(num);
                }
            }
        } else {
            i += 1;
        }
    }
    None
}

fn estimate_layers(params_b: f64) -> u32 {
    if params_b < 2.0 {
        24
    } else if params_b < 8.0 {
        32
    } else if params_b < 14.0 {
        40
    } else if params_b < 40.0 {
        64
    } else {
        80
    }
}

fn estimate_hidden(params_b: f64) -> u32 {
    if params_b < 2.0 {
        2048
    } else if params_b < 8.0 {
        4096
    } else if params_b < 14.0 {
        5120
    } else if params_b < 40.0 {
        8192
    } else {
        8192
    }
}

impl ModelInfo {
    pub fn vram_gb(&self, bytes_per_param: f64) -> f64 {
        self.params_b * bytes_per_param * 1.2
    }

    pub fn kv_gb(&self, context: u32) -> f64 {
        // Very rough: 2 * layers * hidden * context * 2 bytes / GPU-agnostic total
        let bytes = 2.0 * self.layers as f64 * self.hidden as f64 * context as f64 * 2.0;
        bytes / (1024.0 * 1024.0 * 1024.0)
    }

    pub fn print(&self) {
        println!("{}", self.id);
        println!();
        println!("Architecture: {}", self.architecture);
        println!("Parameters:   {:.1}B (estimate)", self.params_b);
        println!("Context:      {}", self.context);
        println!("Layers:       {}", self.layers);
        println!("Hidden:       {}", self.hidden);
        println!(
            "Local cache:  {}",
            if self.local {
                self.path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| "yes".into())
            } else {
                "not cached — run: buraaq ai pull MODEL".into()
            }
        );
        if let Some(lic) = &self.license {
            println!("License:      {lic}");
        }
        println!();
        println!("Estimated VRAM (weights + 20% overhead, excl. KV):");
        println!("  FP16:  {:.1} GB", self.vram_gb(2.0));
        println!("  BF16:  {:.1} GB", self.vram_gb(2.0));
        println!("  INT8:  {:.1} GB", self.vram_gb(1.0));
        println!("  4-bit: {:.1} GB", self.vram_gb(0.5));
        let host = probe_host();
        println!();
        println!("Host GPUs:");
        if host.gpus.is_empty() {
            println!("  (none detected)");
        } else {
            for (i, g) in host.gpus.iter().enumerate() {
                println!("  GPU {i}: {} — {:.1} GB", g.name, g.vram_gb);
            }
        }
    }
}
