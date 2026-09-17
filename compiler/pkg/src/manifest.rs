use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ManifestError {
    #[error("failed to read manifest: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse buraaq.pkg: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to write buraaq.pkg: {0}")]
    Serialize(#[from] toml::ser::Error),
    #[error("no buraaq.pkg found")]
    NotFound,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct Manifest {
    pub package: PackageMeta,
    #[serde(default)]
    pub dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub dev_dependencies: BTreeMap<String, DependencySpec>,
    #[serde(default)]
    pub build: BuildSection,
    #[serde(default)]
    pub features: BTreeMap<String, Vec<String>>,
    #[serde(default)]
    pub lints: BTreeMap<String, String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    #[serde(default = "default_entry")]
    pub entry: String,
    #[serde(default)]
    pub authors: Vec<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    /// Target board for `buraaq flash` (e.g. `pico_w`).
    #[serde(default)]
    pub board: Option<String>,
    #[serde(default)]
    pub r#type: PackageType,
}

fn default_entry() -> String {
    "main".into()
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PackageType {
    #[default]
    Bin,
    Lib,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum DependencySpec {
    Version(String),
    Detailed(DetailedDep),
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct DetailedDep {
    pub version: Option<String>,
    pub path: Option<PathBuf>,
    pub git: Option<String>,
    pub rev: Option<String>,
    pub registry: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct BuildSection {
    #[serde(default)]
    pub opt: Option<String>,
    #[serde(default)]
    pub targets: Vec<String>,
}

impl Manifest {
    pub fn load(path: &Path) -> Result<Self, ManifestError> {
        let text = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&text)?)
    }

    pub fn save(&self, path: &Path) -> Result<(), ManifestError> {
        let text = toml::to_string_pretty(self)?;
        std::fs::write(path, text)?;
        Ok(())
    }

    pub fn default_app(name: &str) -> Self {
        Self {
            package: PackageMeta {
                name: name.to_string(),
                version: "0.1.0".into(),
                entry: "main".into(),
                authors: vec![],
                license: Some("MIT".into()),
                description: None,
                board: None,
                r#type: PackageType::Bin,
            },
            dependencies: BTreeMap::new(),
            dev_dependencies: BTreeMap::new(),
            build: BuildSection::default(),
            features: BTreeMap::new(),
            lints: BTreeMap::new(),
        }
    }
}
