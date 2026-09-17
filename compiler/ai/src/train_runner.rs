//! Managed ~/.buraaq/ai-venv + PEFT/TRL QLoRA job runner.

use std::env;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use sha2::{Digest, Sha256};

use crate::train::compute_train_plan;
use crate::{AiError, Result};

pub fn venv_root() -> PathBuf {
    if let Ok(p) = env::var("BURAAQ_AI_VENV") {
        return PathBuf::from(p);
    }
    crate::cache::cache_root()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("ai-venv")
}

pub fn train_ready() -> bool {
    python_bin().map(|p| p.is_file()).unwrap_or(false)
        && marker_ok()
}

fn marker_ok() -> bool {
    venv_root().join(".buraaq-train-ready").is_file()
}

fn python_bin() -> Option<PathBuf> {
    let root = venv_root();
    #[cfg(windows)]
    {
        let p = root.join("Scripts").join("python.exe");
        if p.is_file() {
            return Some(p);
        }
    }
    #[cfg(not(windows))]
    {
        let p = root.join("bin").join("python");
        if p.is_file() {
            return Some(p);
        }
        let p = root.join("bin").join("python3");
        if p.is_file() {
            return Some(p);
        }
    }
    None
}

fn find_system_python() -> Result<PathBuf> {
    for name in ["python", "python3"] {
        if let Some(p) = crate::cache::which(name) {
            return Ok(p);
        }
    }
    Err(AiError::msg(
        "python not found on PATH.\nInstall Python 3.10+ then: buraaq ai doctor --fix",
    ))
}

/// Create venv and install pinned training deps (Transformers / PEFT / TRL / bitsandbytes).
pub fn ensure_train_env() -> Result<()> {
    let root = venv_root();
    fs::create_dir_all(&root)?;
    if !python_bin().map(|p| p.is_file()).unwrap_or(false) {
        let py = find_system_python()?;
        println!("Creating managed venv at {}", root.display());
        let st = Command::new(&py)
            .args(["-m", "venv"])
            .arg(&root)
            .status()
            .map_err(|e| AiError::msg(format!("venv create failed: {e}")))?;
        if !st.success() {
            return Err(AiError::msg("python -m venv failed"));
        }
    }
    let py = python_bin().ok_or_else(|| AiError::msg("venv python missing after create"))?;
    println!("Installing training stack into {}", root.display());
    println!("  (torch, transformers, peft, trl, bitsandbytes, datasets, accelerate)");
    // Upgrade pip first.
    let _ = Command::new(&py)
        .args(["-m", "pip", "install", "--upgrade", "pip"])
        .status();
    let pkgs = [
        "torch",
        "transformers>=4.40",
        "peft>=0.11",
        "trl>=0.9",
        "bitsandbytes>=0.43",
        "datasets>=2.18",
        "accelerate>=0.30",
        "sentencepiece",
        "protobuf",
    ];
    let mut cmd = Command::new(&py);
    cmd.args(["-m", "pip", "install"]);
    cmd.args(pkgs);
    let st = cmd
        .status()
        .map_err(|e| AiError::msg(format!("pip install failed: {e}")))?;
    if !st.success() {
        return Err(AiError::msg(
            "pip install of training deps failed.\nOn CUDA hosts, ensure a matching torch build; then retry: buraaq ai doctor --fix",
        ));
    }
    // Sanity import
    let check = Command::new(&py)
        .args([
            "-c",
            "import transformers, peft, trl; print('ok')",
        ])
        .output()
        .map_err(|e| AiError::msg(e.to_string()))?;
    if !check.status.success() {
        return Err(AiError::msg(format!(
            "imports failed: {}",
            String::from_utf8_lossy(&check.stderr)
        )));
    }
    fs::write(
        root.join(".buraaq-train-ready"),
        format!(
            "ready\n{}\n",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        ),
    )?;
    println!("train: ready ({})", root.display());
    Ok(())
}

pub struct TrainOptions {
    pub config_path: Option<PathBuf>,
    pub plan_only: bool,
    pub yes: bool,
    pub adapter_hint: Option<PathBuf>,
}

pub fn run_train(opts: TrainOptions) -> Result<()> {
    let cfg = crate::train::load_config(opts.config_path.as_deref())?;
    let plan = compute_train_plan(&cfg)?;
    crate::train::print_train_plan(&plan, opts.config_path.as_deref());

    if opts.plan_only {
        println!();
        println!("(--plan-only) not launching PEFT/TRL.");
        return Ok(());
    }

    if !opts.yes {
        print!("Proceed with training? [y/N] ");
        let _ = std::io::stdout().flush();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).ok();
        let t = line.trim().to_ascii_lowercase();
        if t != "y" && t != "yes" {
            println!("aborted");
            return Ok(());
        }
    }

    if !train_ready() {
        println!("Training env not ready — running doctor --fix…");
        ensure_train_env()?;
    }

    let data = PathBuf::from(&cfg.data);
    if !data.is_file() {
        return Err(AiError::msg(format!(
            "training data not found: {}\nCreate JSONL or set data= in buraaq.ai.toml",
            data.display()
        )));
    }
    crate::dataset::dataset_validate(&data)?;

    let run_id = format!(
        "run-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    );
    let work = PathBuf::from("target/ai/train").join(&run_id);
    fs::create_dir_all(&work)?;
    let out_dir = opts
        .adapter_hint
        .unwrap_or_else(|| PathBuf::from("target/ai/adapters").join(&run_id));
    fs::create_dir_all(&out_dir)?;

    let job = json!({
        "model": cfg.model,
        "data": data.canonicalize().unwrap_or(data.clone()).display().to_string(),
        "method": plan.method,
        "batch_size": plan.batch_size,
        "grad_accum": plan.grad_accum,
        "context": plan.context,
        "output_dir": out_dir.canonicalize().unwrap_or(out_dir.clone()).display().to_string(),
        "max_steps": 50,
        "learning_rate": 2e-4,
    });
    let job_path = work.join("train_job.json");
    fs::write(&job_path, serde_json::to_string_pretty(&job)?)?;
    let script_path = work.join("train_qlora.py");
    fs::write(&script_path, TRAIN_SCRIPT)?;

    let py = python_bin().ok_or_else(|| AiError::msg("venv python missing"))?;
    println!();
    println!("Launching training…");
    println!("  python: {}", py.display());
    println!("  job:    {}", job_path.display());
    println!("  out:    {}", out_dir.display());

    let mut child = Command::new(&py)
        .arg(&script_path)
        .arg(&job_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| AiError::msg(format!("spawn train failed: {e}")))?;

    if let Some(out) = child.stdout.take() {
        let reader = BufReader::new(out);
        for line in reader.lines().flatten() {
            println!("{line}");
        }
    }
    if let Some(err) = child.stderr.take() {
        let reader = BufReader::new(err);
        for line in reader.lines().flatten() {
            eprintln!("{line}");
        }
    }
    let status = child
        .wait()
        .map_err(|e| AiError::msg(format!("wait train: {e}")))?;
    if !status.success() {
        return Err(AiError::msg(format!(
            "training exited with {status}\nSee logs above. Fix CUDA/torch or reduce batch/context."
        )));
    }

    let sha = hash_dir_files(&out_dir)?;
    let meta = json!({
        "run_id": run_id,
        "model": cfg.model,
        "method": plan.method,
        "data": cfg.data,
        "adapter_dir": out_dir.display().to_string(),
        "sha256": sha,
    });
    fs::write(out_dir.join("adapter_config.json"), serde_json::to_string_pretty(&meta)?)?;
    fs::write(out_dir.join("SHA256"), &sha)?;

    // Register in cache side-car
    let reg = crate::cache::cache_root()
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("ai")
        .join("adapters.jsonl");
    if let Some(parent) = reg.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&reg)?;
    writeln!(f, "{}", meta)?;

    println!();
    println!("Adapter ready: {}", out_dir.display());
    println!("sha256 {sha}");
    println!("Next: buraaq ai pack {}  (manifest can reference this adapter)", cfg.model);
    Ok(())
}

fn hash_dir_files(dir: &Path) -> Result<String> {
    let mut h = Sha256::new();
    let mut paths: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    paths.sort();
    for p in paths {
        h.update(p.file_name().and_then(|s| s.to_str()).unwrap_or("").as_bytes());
        if let Ok(bytes) = fs::read(&p) {
            h.update(&bytes);
        }
    }
    Ok(format!("{:x}", h.finalize()))
}

const TRAIN_SCRIPT: &str = r#"
# Generated by Buraaq AI — QLoRA / LoRA SFT via TRL + PEFT.
import json, sys, os
from pathlib import Path

job_path = Path(sys.argv[1])
job = json.loads(job_path.read_text(encoding="utf-8"))

model_id = job["model"]
data_path = job["data"]
method = job.get("method", "qlora")
out_dir = job["output_dir"]
batch = int(job.get("batch_size", 1))
accum = int(job.get("grad_accum", 8))
ctx = int(job.get("context", 2048))
max_steps = int(job.get("max_steps", 50))
lr = float(job.get("learning_rate", 2e-4))

print(f"buraaq train: model={model_id} method={method} steps={max_steps}")

from datasets import load_dataset
from transformers import AutoModelForCausalLM, AutoTokenizer, BitsAndBytesConfig
from peft import LoraConfig, prepare_model_for_kbit_training
from trl import SFTTrainer, SFTConfig
import torch

ds = load_dataset("json", data_files=data_path, split="train")

def to_text(example):
    if "messages" in example and example["messages"]:
        parts = []
        for m in example["messages"]:
            role = m.get("role", "user")
            content = m.get("content", "")
            parts.append(f"{role}: {content}")
        return {"text": "\n".join(parts)}
    if "text" in example:
        return {"text": example["text"]}
    prompt = example.get("prompt", "")
    completion = example.get("completion", "")
    return {"text": f"user: {prompt}\nassistant: {completion}"}

ds = ds.map(to_text)

tokenizer = AutoTokenizer.from_pretrained(model_id, trust_remote_code=True)
if tokenizer.pad_token is None:
    tokenizer.pad_token = tokenizer.eos_token

use_qlora = method == "qlora" and torch.cuda.is_available()
bnb = None
if use_qlora:
    bnb = BitsAndBytesConfig(
        load_in_4bit=True,
        bnb_4bit_quant_type="nf4",
        bnb_4bit_compute_dtype=torch.bfloat16 if torch.cuda.is_bf16_supported() else torch.float16,
    )

model = AutoModelForCausalLM.from_pretrained(
    model_id,
    quantization_config=bnb,
    device_map="auto" if torch.cuda.is_available() else None,
    trust_remote_code=True,
)
if use_qlora:
    model = prepare_model_for_kbit_training(model)

lora = LoraConfig(
    r=16,
    lora_alpha=32,
    lora_dropout=0.05,
    bias="none",
    task_type="CAUSAL_LM",
    target_modules=["q_proj", "v_proj", "k_proj", "o_proj"],
)

args = SFTConfig(
    output_dir=out_dir,
    per_device_train_batch_size=batch,
    gradient_accumulation_steps=accum,
    max_steps=max_steps,
    learning_rate=lr,
    logging_steps=5,
    save_steps=max_steps,
    max_seq_length=ctx,
    packing=False,
    report_to=[],
)

trainer = SFTTrainer(
    model=model,
    train_dataset=ds,
    peft_config=lora,
    args=args,
    processing_class=tokenizer,
)
trainer.train()
trainer.save_model(out_dir)
tokenizer.save_pretrained(out_dir)
print(f"buraaq train: saved adapter to {out_dir}")
"#;
