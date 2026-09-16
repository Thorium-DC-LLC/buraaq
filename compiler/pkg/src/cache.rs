use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BuildCache {
    pub files: BTreeMap<String, String>,
    pub last_binary_hash: Option<String>,
}

impl BuildCache {
    pub fn load(path: &Path) -> Self {
        if path.exists() {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|t| serde_json::from_str(&t).ok())
                .unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, serde_json::to_string_pretty(self).unwrap())
    }

    pub fn needs_rebuild(&self, sources: &[PathBuf]) -> bool {
        for src in sources {
            let key = src.display().to_string();
            let hash = hash_file(src).unwrap_or_default();
            match self.files.get(&key) {
                Some(old) if old == &hash => continue,
                _ => return true,
            }
        }
        false
    }

    pub fn update(&mut self, sources: &[PathBuf]) {
        for src in sources {
            let key = src.display().to_string();
            if let Ok(hash) = hash_file(src) {
                self.files.insert(key, hash);
            }
        }
    }
}

pub fn hash_file(path: &Path) -> std::io::Result<String> {
    let bytes = std::fs::read(path)?;
    let mut h = Sha256::new();
    h.update(&bytes);
    Ok(hex::encode(h.finalize()))
}

pub use hex;
