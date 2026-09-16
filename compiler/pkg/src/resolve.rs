use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use semver::{Version, VersionReq};
use sha2::{Digest, Sha256};
use thiserror::Error;

use crate::lockfile::{Lockfile, LockedPackage};
use crate::manifest::{DependencySpec, Manifest};

#[derive(Debug, Error)]
pub enum ResolveError {
    #[error("dependency `{0}` not found")]
    NotFound(String),
    #[error("version requirement unsatisfied for `{0}`: want {1}")]
    Unsatisfied(String, String),
    #[error("lock error: {0}")]
    Lock(#[from] crate::lockfile::LockError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

const REGISTRY: &str = "https://packages.buraaq.dev";

pub struct Resolver {
    pub cache_dir: PathBuf,
    pub registry: String,
}

impl Resolver {
    pub fn new(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            registry: REGISTRY.to_string(),
        }
    }

    pub fn resolve(&self, manifest: &Manifest, lock_path: &Path) -> Result<Lockfile, ResolveError> {
        let mut lock = if lock_path.exists() {
            Lockfile::load(lock_path)?
        } else {
            Lockfile {
                version: 1,
                packages: vec![],
            }
        };

        let root = lock_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));

        for (name, spec) in manifest.dependencies.iter().chain(manifest.dev_dependencies.iter()) {
            if lock.find(name).is_some() {
                continue;
            }
            let locked = self.resolve_one(name, spec, &root)?;
            lock.packages.push(locked);
        }

        lock.packages.sort_by(|a, b| a.name.cmp(&b.name));
        lock.save(lock_path)?;
        Ok(lock)
    }

    fn resolve_one(
        &self,
        name: &str,
        spec: &DependencySpec,
        root: &Path,
    ) -> Result<LockedPackage, ResolveError> {
        match spec {
            DependencySpec::Version(v) => self.resolve_registry(name, v),
            DependencySpec::Detailed(d) => {
                if let Some(path) = &d.path {
                    return self.resolve_path(name, path, root);
                }
                if let Some(git) = &d.git {
                    return self.resolve_git(name, git, d.rev.as_deref());
                }
                if let Some(v) = &d.version {
                    return self.resolve_registry(name, v);
                }
                Err(ResolveError::NotFound(name.to_string()))
            }
        }
    }

    fn resolve_path(
        &self,
        name: &str,
        rel: &Path,
        root: &Path,
    ) -> Result<LockedPackage, ResolveError> {
        let abs = root.join(rel);
        let checksum = hash_dir(&abs)?;
        Ok(LockedPackage {
            name: name.to_string(),
            version: read_pkg_version(&abs).unwrap_or_else(|| "0.0.0".into()),
            source: format!("path+{}", abs.display()),
            checksum,
            dependencies: vec![],
        })
    }

    fn resolve_git(
        &self,
        name: &str,
        url: &str,
        rev: Option<&str>,
    ) -> Result<LockedPackage, ResolveError> {
        let rev = rev.unwrap_or("HEAD");
        let checksum = hash_str(&format!("{url}@{rev}"));
        Ok(LockedPackage {
            name: name.to_string(),
            version: "0.0.0-git".into(),
            source: format!("git+{url}#{rev}"),
            checksum,
            dependencies: vec![],
        })
    }

    fn resolve_registry(&self, name: &str, req: &str) -> Result<LockedPackage, ResolveError> {
        let version_req = VersionReq::parse(req)
            .map_err(|_| ResolveError::Unsatisfied(name.to_string(), req.to_string()))?;

        // Offline cache: registry index stub — pin to highest matching semver in cache metadata
        let cached = self.registry_cache_lookup(name, &version_req)?;
        let checksum = hash_str(&format!("{name}@{}:{}", cached, self.registry));

        Ok(LockedPackage {
            name: name.to_string(),
            version: cached,
            source: format!("registry+{}", self.registry),
            checksum,
            dependencies: vec!["buraaq-std@1.0.0".into()],
        })
    }

    fn registry_cache_lookup(
        &self,
        name: &str,
        req: &VersionReq,
    ) -> Result<String, ResolveError> {
        std::fs::create_dir_all(&self.cache_dir)?;
        let index_path = self.cache_dir.join("registry-index.json");
        let mut index: BTreeMap<String, Vec<String>> = if index_path.exists() {
            serde_json::from_str(&std::fs::read_to_string(&index_path)?).unwrap_or_default()
        } else {
            default_registry_index()
        };

        if !index.contains_key(name) {
            // Stub: new registry packages resolve to 0.1.0
            index.insert(name.to_string(), vec!["0.1.0".into()]);
            std::fs::write(&index_path, serde_json::to_string_pretty(&index).unwrap())?;
        }

        let versions = index.get(name).ok_or_else(|| ResolveError::NotFound(name.to_string()))?;
        let mut best: Option<Version> = None;
        for v in versions {
            if let Ok(ver) = Version::parse(v) {
                if req.matches(&ver) {
                    if best.as_ref().is_none_or(|b| ver > *b) {
                        best = Some(ver);
                    }
                }
            }
        }
        best.map(|v| v.to_string())
            .ok_or_else(|| ResolveError::Unsatisfied(name.to_string(), req.to_string()))
    }
}

fn default_registry_index() -> BTreeMap<String, Vec<String>> {
    let mut m = BTreeMap::new();
    m.insert("buraaq-std".into(), vec!["1.0.0".into()]);
    m.insert("postgres".into(), vec!["0.1.0".into(), "0.2.0".into()]);
    m.insert("json".into(), vec!["0.4.2".into()]);
    m
}

fn hash_str(s: &str) -> String {
    let mut h = Sha256::new();
    h.update(s.as_bytes());
    format!("sha256:{}", hex::encode(h.finalize()))
}

fn hash_dir(path: &Path) -> Result<String, ResolveError> {
    if !path.exists() {
        return Ok(hash_str(&format!("missing:{}", path.display())));
    }
    let mut h = Sha256::new();
    h.update(path.display().to_string().as_bytes());
    Ok(format!("sha256:{}", hex::encode(h.finalize())))
}

fn read_pkg_version(path: &Path) -> Option<String> {
    let pkg = path.join("buraaq.pkg");
    Manifest::load(&pkg).ok().map(|m| m.package.version)
}

pub fn add_dependency(manifest: &mut Manifest, name: &str, version: &str) -> bool {
    if manifest.dependencies.contains_key(name) {
        return false;
    }
    manifest
        .dependencies
        .insert(name.to_string(), DependencySpec::Version(version.to_string()));
    true
}

pub fn remove_dependency(manifest: &mut Manifest, name: &str) -> bool {
    manifest.dependencies.remove(name).is_some()
        || manifest.dev_dependencies.remove(name).is_some()
}

pub fn prune_lock(lock: &mut Lockfile, manifest: &Manifest) {
    let needed: HashSet<String> = manifest
        .dependencies
        .keys()
        .chain(manifest.dev_dependencies.keys())
        .cloned()
        .collect();
    lock.packages.retain(|p| needed.contains(&p.name));
}
