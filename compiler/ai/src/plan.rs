use crate::doctor::{probe_host, GpuInfo};
use crate::error::AiError;
use crate::model::{inspect_model, ModelInfo};
use crate::Result;

#[derive(Debug, Clone)]
pub struct PlanRequest {
    pub model: String,
    pub backend: String, // auto | vllm | llamacpp | openai
    pub quantize: Option<String>, // none | 8bit | 4bit | awq
    pub context: Option<u32>,
    pub gpus: String, // auto | 0,1 | …
    pub tensor_parallel: Option<u32>,
    /// When false, record a recommended backend even if none are installed (for `ai pack`).
    pub require_backend: bool,
}

#[derive(Debug, Clone)]
pub struct DeployPlan {
    pub model: ModelInfo,
    pub backend: String,
    pub quantize: String,
    pub bytes_per_param: f64,
    pub context: u32,
    pub tensor_parallel: u32,
    pub gpu_indices: Vec<usize>,
    pub model_vram_per_gpu_gb: f64,
    pub kv_per_gpu_gb: f64,
    pub estimated_fit: bool,
    pub rationale: Vec<String>,
}

pub fn build_plan(req: &PlanRequest) -> Result<DeployPlan> {
    let model = inspect_model(&req.model)?;
    let host = probe_host();
    let mut rationale = Vec::new();

    let context = req.context.unwrap_or(model.context.min(8192));
    rationale.push(format!("context={context} (requested or model default, capped for safety)"));

    let (quantize, bpp) = choose_quant(&model, &host.gpus, req.quantize.as_deref(), &mut rationale);
    let gpu_indices = parse_gpus(&req.gpus, host.gpus.len(), &mut rationale);
    let tp = req
        .tensor_parallel
        .unwrap_or_else(|| gpu_indices.len().max(1) as u32);
    if tp > 1 {
        rationale.push(format!("tensor_parallel={tp} across {} GPU(s)", gpu_indices.len()));
    }

    let weight_gb = model.vram_gb(bpp);
    let kv_total = model.kv_gb(context);
    let per = (weight_gb / tp as f64) + (kv_total / tp as f64);
    let min_vram = gpu_indices
        .iter()
        .filter_map(|&i| host.gpus.get(i).map(|g| g.vram_gb))
        .fold(f64::INFINITY, f64::min);
    let estimated_fit = if host.gpus.is_empty() {
        rationale.push("no GPU: plan assumes CPU/offload backend (llama.cpp or remote)".into());
        true
    } else {
        let fit = per < min_vram * 0.92;
        if !fit {
            rationale.push(format!(
                "does not fit: ~{per:.1} GB/GPU needed, smallest selected GPU has {min_vram:.1} GB"
            ));
        } else {
            rationale.push(format!(
                "fits: ~{per:.1} GB/GPU vs {min_vram:.1} GB available (with ~8% reserve)"
            ));
        }
        fit
    };

    let backend = choose_backend(
        &req.backend,
        &host.gpus,
        &model,
        &quantize,
        req.require_backend,
        &mut rationale,
    )?;

    if !estimated_fit && host.gpus.len() > 0 && quantize == "none" {
        return Err(AiError::oom_hint(
            &model.id,
            weight_gb + kv_total,
            &host
                .gpus
                .iter()
                .map(|g| (g.name.clone(), g.vram_gb))
                .collect::<Vec<_>>(),
        ));
    }

    Ok(DeployPlan {
        model,
        backend,
        quantize,
        bytes_per_param: bpp,
        context,
        tensor_parallel: tp,
        gpu_indices,
        model_vram_per_gpu_gb: weight_gb / tp as f64,
        kv_per_gpu_gb: kv_total / tp as f64,
        estimated_fit,
        rationale,
    })
}

pub fn print_plan(plan: &DeployPlan) {
    let host = probe_host();
    println!("Model: {}", plan.model.id);
    println!();
    println!("Host:");
    if host.gpus.is_empty() {
        println!("  (no GPU)");
    } else {
        for (i, g) in host.gpus.iter().enumerate() {
            let mark = if plan.gpu_indices.contains(&i) {
                "*"
            } else {
                " "
            };
            println!("{mark} GPU {i}: {} — {:.1} GB", g.name, g.vram_gb);
        }
    }
    println!();
    println!(
        "FP16 estimate: ~{:.1} GB VRAM",
        plan.model.vram_gb(2.0) + plan.model.kv_gb(plan.context)
    );
    if plan.estimated_fit {
        println!("Fits with selected plan.");
    } else {
        println!("Does not fit without further quantization/offload.");
    }
    println!();
    println!("Selected plan:");
    println!("  Quantization: {}", plan.quantize);
    println!("  Backend: {}", plan.backend);
    println!("  Tensor parallel: {}", plan.tensor_parallel);
    println!("  Context: {}", plan.context);
    println!("  KV cache reserve: {:.1} GB/GPU", plan.kv_per_gpu_gb);
    println!();
    println!("Estimated:");
    println!("  Model VRAM: {:.1} GB/GPU", plan.model_vram_per_gpu_gb);
    println!(
        "  Available KV cache: {:.1} GB/GPU",
        plan.kv_per_gpu_gb
    );
    println!();
    println!("Why:");
    for r in &plan.rationale {
        println!("  - {r}");
    }
}

fn choose_quant(
    model: &ModelInfo,
    gpus: &[GpuInfo],
    override_q: Option<&str>,
    rationale: &mut Vec<String>,
) -> (String, f64) {
    if let Some(q) = override_q {
        let (name, bpp) = match q {
            "none" | "fp16" | "bf16" => ("none".into(), 2.0),
            "8bit" | "int8" => ("8bit".into(), 1.0),
            "4bit" | "awq" | "nf4" => ("4bit".into(), 0.5),
            other => {
                rationale.push(format!("unknown --quantize {other}; using 4bit"));
                ("4bit".into(), 0.5)
            }
        };
        rationale.push(format!("quantization overridden: {name}"));
        return (name, bpp);
    }
    let total_vram: f64 = gpus.iter().map(|g| g.vram_gb).sum();
    let fp16 = model.vram_gb(2.0) + model.kv_gb(8192);
    if gpus.is_empty() {
        rationale.push("no GPU: prefer 4-bit / GGUF-friendly plan".into());
        return ("4bit".into(), 0.5);
    }
    if fp16 <= total_vram * 0.85 {
        rationale.push("FP16 fits total VRAM with margin — no quant".into());
        ("none".into(), 2.0)
    } else if model.vram_gb(1.0) + model.kv_gb(8192) <= total_vram * 0.85 {
        rationale.push("FP16 does not fit; selecting INT8".into());
        ("8bit".into(), 1.0)
    } else {
        rationale.push("FP16/INT8 do not fit; selecting 4-bit (quality reduced — stated)".into());
        ("4bit".into(), 0.5)
    }
}

fn parse_gpus(spec: &str, n: usize, rationale: &mut Vec<String>) -> Vec<usize> {
    if spec == "auto" || spec.is_empty() {
        let all: Vec<_> = (0..n).collect();
        rationale.push(format!("gpus=auto → {:?}", all));
        return all;
    }
    let mut idxs = Vec::new();
    for part in spec.split(',') {
        if let Ok(i) = part.trim().parse::<usize>() {
            if i < n {
                idxs.push(i);
            }
        }
    }
    if idxs.is_empty() && n > 0 {
        idxs.push(0);
    }
    rationale.push(format!("gpus={spec} → {:?}", idxs));
    idxs
}

fn choose_backend(
    requested: &str,
    gpus: &[GpuInfo],
    model: &ModelInfo,
    quant: &str,
    require: bool,
    rationale: &mut Vec<String>,
) -> Result<String> {
    let detected = crate::backend::detect_backends();
    let has = |name: &str| {
        detected
            .iter()
            .any(|b| b.name == name && b.available)
    };

    if requested != "auto" {
        if !has(requested) && requested != "openai" {
            rationale.push(format!(
                "requested backend `{requested}` not detected; will still try attach/spawn"
            ));
        } else {
            rationale.push(format!("backend overridden: {requested}"));
        }
        return Ok(requested.to_string());
    }

    if has("vllm") && (gpus.len() > 1 || model.params_b >= 13.0) {
        rationale.push("auto: vLLM (multi-GPU or large model)".into());
        return Ok("vllm".into());
    }
    if has("llamacpp") && (quant == "4bit" || gpus.is_empty() || model.params_b <= 14.0) {
        rationale.push("auto: llama.cpp (local / quantized friendly)".into());
        return Ok("llamacpp".into());
    }
    if has("openai") {
        rationale.push("auto: OpenAI-compatible (BURAAQ_AI_BASE_URL)".into());
        return Ok("openai".into());
    }
    if has("vllm") {
        rationale.push("auto: vLLM".into());
        return Ok("vllm".into());
    }
    if has("llamacpp") {
        rationale.push("auto: llama.cpp".into());
        return Ok("llamacpp".into());
    }
    if !require {
        let rec = if quant == "4bit" || gpus.is_empty() {
            "llamacpp"
        } else if gpus.len() > 1 || model.params_b >= 13.0 {
            "vllm"
        } else {
            "llamacpp"
        };
        rationale.push(format!(
            "no backend installed yet; manifest recommends `{rec}`"
        ));
        return Ok(rec.into());
    }
    Err(AiError::msg(
        "no inference backend available.\n\nRun: buraaq ai doctor\nSet BURAAQ_AI_BASE_URL, or install llama-server / vLLM.",
    ))
}
