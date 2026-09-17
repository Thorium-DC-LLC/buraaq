use std::env;
use std::path::PathBuf;
use std::process::Command;

use crate::cache::which;

#[derive(Debug, Clone)]
pub struct BackendInfo {
    pub name: String,
    pub available: bool,
    pub detail: String,
}

pub fn detect_backends() -> Vec<BackendInfo> {
    let mut out = Vec::new();

    let llama = which("llama-server")
        .or_else(|| which("llama-cli"))
        .or_else(|| which("main"));
    out.push(BackendInfo {
        name: "llamacpp".into(),
        available: llama.is_some(),
        detail: llama
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "llama-server not on PATH".into()),
    });

    let vllm = Command::new("python")
        .args(["-c", "import vllm; print(vllm.__version__)"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string());
    out.push(BackendInfo {
        name: "vllm".into(),
        available: vllm.is_some(),
        detail: vllm.unwrap_or_else(|| "python -c 'import vllm' failed".into()),
    });

    out.push(BackendInfo {
        name: "sglang".into(),
        available: false,
        detail: "Phase 1: not wired".into(),
    });
    out.push(BackendInfo {
        name: "mlx".into(),
        available: cfg!(target_os = "macos"),
        detail: if cfg!(target_os = "macos") {
            "Metal host; adapter later".into()
        } else {
            "macOS only".into()
        },
    });

    let base = env::var("BURAAQ_AI_BASE_URL").ok().filter(|s| !s.is_empty());
    let openai_ok = if let Some(ref url) = base {
        probe_health(url)
    } else {
        false
    };
    out.push(BackendInfo {
        name: "openai".into(),
        available: base.is_some() && openai_ok || base.is_some(),
        detail: base.unwrap_or_else(|| "set BURAAQ_AI_BASE_URL".into()),
    });

    out
}

fn probe_health(base: &str) -> bool {
    let url = format!("{}/health", base.trim_end_matches('/'));
    ureq::get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .call()
        .is_ok()
        || ureq::get(&format!("{}/v1/models", base.trim_end_matches('/')))
            .timeout(std::time::Duration::from_secs(2))
            .call()
            .is_ok()
}

pub fn find_gguf(dir: &PathBuf) -> Option<PathBuf> {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return None;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.extension().and_then(|x| x.to_str()) == Some("gguf") {
            return Some(p);
        }
        if p.is_dir() {
            if let Some(f) = find_gguf(&p) {
                return Some(f);
            }
        }
    }
    None
}
