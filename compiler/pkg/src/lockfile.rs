use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum LockError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("serialize error: {0}")]
    Serialize(#[from] toml::ser::Error),
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Lockfile {
    pub version: u32,
    #[serde(default)]
    pub packages: Vec<LockedPackage>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct LockedPackage {
    pub name: String,
    pub version: String,
    pub source: String,
    pub checksum: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl Lockfile {
    pub fn load(path: &Path) -> Result<Self, LockError> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), LockError> {
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }

    pub fn find(&self, name: &str) -> Option<&LockedPackage> {
        self.packages.iter().find(|p| p.name == name)
    }
}
