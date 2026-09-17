use std::fs;
use std::time::Duration;

use crate::doctor::probe_host;
use crate::Result;

pub fn print_status() -> Result<()> {
    let p = crate::cache::cache_root()
        .parent()
        .map(|x| x.join("ai").join("status.json"))
        .unwrap_or_else(|| std::path::PathBuf::from("status.json"));
    println!("Buraaq AI status");
    println!();

    let mut bind: Option<String> = None;
    if p.exists() {
        let raw = fs::read_to_string(&p)?;
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) {
            println!("Model:    {}", v["model"].as_str().unwrap_or("?"));
            println!("Backend:  {}", v["backend"].as_str().unwrap_or("?"));
            println!("Bind:     {}", v["bind"].as_str().unwrap_or("?"));
            println!("Upstream: {}", v["upstream"].as_str().unwrap_or("?"));
            bind = v["bind"].as_str().map(|s| s.to_string());
        } else {
            println!("{}", raw);
        }
    } else {
        println!("No status file at {}", p.display());
        println!("Start with: buraaq ai serve MODEL");
    }

    // Live scrape /health + /metrics when bind known (or default).
    let base = bind
        .map(|b| {
            if b.starts_with("http") {
                b
            } else {
                format!("http://{b}")
            }
        })
        .or_else(|| std::env::var("BURAAQ_AI_BASE_URL").ok())
        .unwrap_or_else(|| "http://127.0.0.1:8000".into());
    let base = base.trim_end_matches('/');

    println!();
    println!("Live ({base}):");
    match ureq::get(&format!("{base}/health"))
        .timeout(Duration::from_secs(2))
        .call()
    {
        Ok(resp) => {
            let body = resp.into_string().unwrap_or_default();
            println!("  /health: ok  {body}");
        }
        Err(e) => println!("  /health: unreachable ({e})"),
    }
    match ureq::get(&format!("{base}/metrics"))
        .timeout(Duration::from_secs(2))
        .call()
    {
        Ok(resp) => {
            let body = resp.into_string().unwrap_or_default();
            for key in [
                "buraaq_ai_requests",
                "buraaq_ai_errors",
                "buraaq_ai_tokens_approx",
                "buraaq_ai_uptime_seconds",
            ] {
                if let Some(line) = body.lines().find(|l| l.starts_with(key)) {
                    println!("  {line}");
                }
            }
        }
        Err(_) => println!("  /metrics: unreachable"),
    }

    println!();
    let host = probe_host();
    if host.gpus.is_empty() {
        println!("GPU: (none detected)");
    } else {
        for (i, g) in host.gpus.iter().enumerate() {
            println!("GPU {i}: {} — {:.1} GB VRAM", g.name, g.vram_gb);
        }
    }
    Ok(())
}
