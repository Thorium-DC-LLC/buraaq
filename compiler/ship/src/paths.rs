use std::env;
use std::path::PathBuf;

pub fn home_dir() -> PathBuf {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn dock_root() -> PathBuf {
    home_dir().join(".buraaq").join("dock")
}

pub fn token_path() -> PathBuf {
    dock_root().join("token")
}

pub fn live_dir(name: &str) -> PathBuf {
    dock_root().join("live").join(name)
}

pub fn run_dir(name: &str, digest: &str) -> PathBuf {
    let short = digest.chars().take(12).collect::<String>();
    home_dir().join(".buraaq").join("run").join(name).join(short)
}
