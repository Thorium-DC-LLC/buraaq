//! Land --ai: GPU host checklist + ai-check.sh in the land kit.

use std::fs;
use std::path::Path;

use crate::doctor::probe_host;
use crate::Result;

const AI_CHECK_SH: &str = r#"#!/usr/bin/env bash
# Buraaq Land AI check — run on the GPU host after land.sh
# Does NOT install drivers.
set -euo pipefail
echo "=== Buraaq AI host check ==="
echo
echo "1. GPU"
if command -v nvidia-smi >/dev/null 2>&1; then
  nvidia-smi --query-gpu=name,memory.total --format=csv
else
  echo "nvidia-smi not found — install vendor drivers manually, then re-run."
fi
echo
echo "2. Disk / cache"
mkdir -p "${HOME}/.buraaq/models"
df -h "${HOME}/.buraaq" 2>/dev/null || df -h "${HOME}"
echo
echo "3. Buraaq AI doctor"
if command -v buraaq >/dev/null 2>&1; then
  buraaq ai doctor || true
else
  echo "buraaq not on PATH yet — install the toolchain, then: buraaq ai doctor"
fi
echo
echo "4. Next"
echo "  buraaq ai serve MODEL"
echo "  Prefer --bind 127.0.0.1 and terminate TLS at your proxy."
echo "Drivers are never auto-installed by this script."
"#;

/// Print what an AI GPU host needs. Does not install or change drivers.
pub fn print_land_ai_plan(host: Option<&str>) {
    let r = probe_host();
    println!("Buraaq Land — AI host plan");
    println!();
    if let Some(h) = host {
        println!("Target: {h}");
    } else {
        println!("Target: (local doctor below; pass user@HOST after writing the kit)");
    }
    println!();
    println!("Do not silently mutate GPU drivers from this tool.");
    println!("Review each step before applying on the host.");
    println!();
    println!("1. NVIDIA / AMD drivers");
    println!("   - Install vendor drivers matching the GPU on the host OS.");
    println!("   - Verify: nvidia-smi  (or rocm-smi)");
    if r.gpus.is_empty() {
        println!("   - This machine: no GPU via nvidia-smi (host may still have one).");
    } else {
        for (i, g) in r.gpus.iter().enumerate() {
            println!("   - Local ref GPU {i}: {} ({:.1} GB)", g.name, g.vram_gb);
        }
    }
    if let Some(c) = &r.cuda {
        println!("   - Local CUDA hint: {c}");
    }
    println!();
    println!("2. Inference backend (pick one)");
    println!("   - llama-server on PATH, or");
    println!("   - vLLM (python -m vllm …), or");
    println!("   - any OpenAI-compatible server + BURAAQ_AI_BASE_URL");
    println!();
    println!("3. Cache + disk");
    println!("   - mkdir -p ~/.buraaq/models");
    println!("   - Ensure tens of GB free for weights (this cache: {})", r.cache);
    println!("   - free disk (this machine approx): {:.1} GB", r.disk_free_gb);
    println!();
    println!("4. Service user + Dock");
    println!("   - Land Dock as usual: buraaq land user@HOST");
    println!("   - Run AI as the same service user that owns ~/.buraaq");
    println!("   - systemd unit can wrap: buraaq ai serve MODEL --bind 127.0.0.1");
    println!();
    println!("5. Firewall");
    println!("   - Prefer serve on loopback; expose via your reverse proxy / TLS.");
    println!("   - If opening the AI port: allow only trusted clients + BURAAQ_AI_KEY.");
    println!();
    println!("6. After land");
    println!("   - bash ai-check.sh       (written into the land kit)");
    println!("   - buraaq ai doctor");
    println!("   - buraaq ai pack MODEL && buraaq ship HOST");
    println!();
    println!("Never auto-installs drivers.");
}

/// Write `ai-check.sh` (+ short README) into an existing land kit directory.
pub fn write_land_ai_kit(dir: &Path) -> Result<()> {
    fs::create_dir_all(dir)?;
    let script = dir.join("ai-check.sh");
    fs::write(&script, AI_CHECK_SH)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&script)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms)?;
    }
    fs::write(
        dir.join("AI.txt"),
        "Buraaq AI host notes\n\n1. Install GPU drivers yourself (never auto).\n2. Run: bash ai-check.sh\n3. buraaq ai doctor --fix   # training env if needed\n4. buraaq ai serve MODEL\n",
    )?;
    println!("AI kit → {} (ai-check.sh, AI.txt)", dir.display());
    Ok(())
}
