use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use sha2::{Digest, Sha256};

use crate::{sanitize_model_id, Result};

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub id: String,
    pub path: PathBuf,
    pub bytes: u64,
    pub sha256: Option<String>,
}

pub fn cache_root() -> PathBuf {
    if let Ok(p) = env::var("BURAAQ_AI_CACHE") {
        return PathBuf::from(p);
    }
    dirs_home()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".buraaq")
        .join("models")
}

fn dirs_home() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

pub fn ensure_cache() -> Result<PathBuf> {
    let root = cache_root();
    fs::create_dir_all(&root)?;
    Ok(root)
}

pub fn model_dir(id: &str) -> Result<PathBuf> {
    let id = sanitize_model_id(id)?;
    let safe = id.replace('/', "--");
    Ok(cache_root().join(safe))
}

pub fn list_models() -> Result<Vec<CacheEntry>> {
    let root = ensure_cache()?;
    let mut out = Vec::new();
    if !root.exists() {
        return Ok(out);
    }
    for ent in fs::read_dir(&root)? {
        let ent = ent?;
        if !ent.file_type()?.is_dir() {
            continue;
        }
        let path = ent.path();
        let meta_path = path.join("buraaq-model.json");
        let id = if meta_path.exists() {
            let raw = fs::read_to_string(&meta_path)?;
            serde_json::from_str::<serde_json::Value>(&raw)
                .ok()
                .and_then(|v| v.get("id").and_then(|x| x.as_str()).map(|s| s.to_string()))
                .unwrap_or_else(|| ent.file_name().to_string_lossy().replace("--", "/"))
        } else {
            ent.file_name().to_string_lossy().replace("--", "/")
        };
        let bytes = dir_size(&path).unwrap_or(0);
        let sha = fs::read_to_string(path.join("SHA256"))
            .ok()
            .map(|s| s.trim().to_string());
        out.push(CacheEntry {
            id,
            path,
            bytes,
            sha256: sha,
        });
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(out)
}

fn dir_size(p: &Path) -> std::io::Result<u64> {
    let mut n = 0u64;
    if p.is_file() {
        return Ok(p.metadata()?.len());
    }
    for e in fs::read_dir(p)? {
        let e = e?;
        let t = e.file_type()?;
        if t.is_dir() {
            n += dir_size(&e.path())?;
        } else if t.is_file() {
            n += e.metadata()?.len();
        }
    }
    Ok(n)
}

/// Pull model metadata into the cache. Full weight download uses an external
/// tool when available; otherwise stores inspectable config + marker.
pub fn pull_model(id: &str) -> Result<CacheEntry> {
    let id = sanitize_model_id(id)?;
    let dir = model_dir(&id)?;
    fs::create_dir_all(&dir)?;

    let mut config_json = None;
    let hub = format!("https://huggingface.co/{id}/raw/main/config.json");
    match ureq::get(&hub).timeout(std::time::Duration::from_secs(30)).call() {
        Ok(resp) => {
            if let Ok(text) = resp.into_string() {
                fs::write(dir.join("config.json"), &text)?;
                config_json = Some(text);
            }
        }
        Err(e) => {
            eprintln!("note: could not fetch config.json ({e}); writing local marker only");
        }
    }

    // Prefer huggingface-cli if present for full weights.
    let hf = which("huggingface-cli").or_else(|| which("hf"));
    if let Some(bin) = hf {
        let status = Command::new(&bin)
            .args(["download", &id, "--local-dir"])
            .arg(&dir)
            .status();
        match status {
            Ok(s) if s.success() => eprintln!("pulled weights via {}", bin.display()),
            Ok(_) => eprintln!("note: {bin:?} download exited non-zero; metadata kept"),
            Err(e) => eprintln!("note: could not run {}: {e}", bin.display()),
        }
    }

    let sha = hash_dir_marker(&dir, &id)?;
    fs::write(dir.join("SHA256"), &sha)?;
    let meta = serde_json::json!({
        "id": id,
        "pulled_at": now_unix(),
        "has_config": config_json.is_some(),
        "sha256": sha,
    });
    fs::write(dir.join("buraaq-model.json"), serde_json::to_string_pretty(&meta)?)?;

    Ok(CacheEntry {
        id,
        path: dir.clone(),
        bytes: dir_size(&dir).unwrap_or(0),
        sha256: Some(sha),
    })
}

fn hash_dir_marker(dir: &Path, id: &str) -> Result<String> {
    let mut h = Sha256::new();
    h.update(id.as_bytes());
    if let Ok(cfg) = fs::read(dir.join("config.json")) {
        h.update(&cfg);
    }
    Ok(format!("{:x}", h.finalize()))
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn cache_clean() -> Result<u64> {
    let root = cache_root();
    if !root.exists() {
        return Ok(0);
    }
    let before = dir_size(&root).unwrap_or(0);
    fs::remove_dir_all(&root)?;
    ensure_cache()?;
    Ok(before)
}

pub(crate) fn which(name: &str) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    for dir in env::split_paths(&path) {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
        #[cfg(windows)]
        {
            let p = dir.join(format!("{name}.exe"));
            if p.is_file() {
                return Some(p);
            }
            let p = dir.join(format!("{name}.cmd"));
            if p.is_file() {
                return Some(p);
            }
        }
    }
    None
}
