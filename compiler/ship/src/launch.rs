use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use thiserror::Error;

use crate::bundle::{extract_to, unpack, Bundle, BundleError};
use crate::paths::{dock_root, live_dir, run_dir};

#[derive(Debug, Error)]
pub enum LaunchError {
    #[error("{0}")]
    Bundle(#[from] BundleError),
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Msg(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LaunchMode {
    /// Wait in the foreground (developer `buraaq ship` / `buraaq launch`).
    Foreground,
    /// Spawn and return pid (dock).
    Background,
}

pub fn launch_bundle(bytes: &[u8], mode: LaunchMode) -> Result<u32, LaunchError> {
    let bundle = unpack(bytes)?;
    let dir = run_dir(&bundle.name, &bundle.digest);
    extract_to(&bundle, &dir)?;
    launch_extracted(&bundle, &dir, mode)
}

pub fn launch_extracted(bundle: &Bundle, dir: &Path, mode: LaunchMode) -> Result<u32, LaunchError> {
    let exe = dir.join("bin").join(&bundle.exe);
    if !exe.is_file() {
        return Err(LaunchError::Msg(format!(
            "extracted binary missing: {}",
            exe.display()
        )));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&exe)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&exe, perms)?;
    }

    stage_host_env(dir);

    match mode {
        LaunchMode::Foreground => {
            let status = command_for_app(&exe, dir).status()?;
            if status.success() {
                Ok(0)
            } else {
                Err(LaunchError::Msg(format!(
                    "app exited {}",
                    status.code().unwrap_or(1)
                )))
            }
        }
        LaunchMode::Background => {
            fs::create_dir_all(dir)?;
            let log_path = dir.join("app.log");
            let log = File::create(&log_path)?;
            let err = log.try_clone()?;
            let child = command_for_app(&exe, dir)
                .stdin(Stdio::null())
                .stdout(Stdio::from(log))
                .stderr(Stdio::from(err))
                .spawn()?;
            let pid = child.id();
            fs::write(dir.join("app.pid"), pid.to_string())?;
            // Detach: forget the Child so Drop does not wait/kill.
            std::mem::forget(child);
            Ok(pid)
        }
    }
}

pub(crate) fn parse_env_file(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let Some((k, v)) = line.split_once('=') else {
            continue;
        };
        let k = k.trim();
        if k.is_empty() {
            continue;
        }
        let mut v = v.trim();
        if v.len() >= 2
            && ((v.starts_with('"') && v.ends_with('"'))
                || (v.starts_with('\'') && v.ends_with('\'')))
        {
            v = &v[1..v.len() - 1];
        }
        if v.is_empty() {
            continue;
        }
        out.push((k.to_string(), v.to_string()));
    }
    out
}

fn stage_host_env(dir: &Path) {
    let src = dock_root().join("env");
    if !src.is_file() {
        return;
    }
    let dst = dir.join("forge.env");
    if fs::copy(&src, &dst).is_ok() {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(meta) = fs::metadata(&dst) {
                let mut perms = meta.permissions();
                perms.set_mode(0o600);
                let _ = fs::set_permissions(&dst, perms);
            }
        }
    }
}

fn apply_host_env(cmd: &mut Command) {
    let p = dock_root().join("env");
    let Ok(text) = fs::read_to_string(&p) else {
        return;
    };
    for (k, v) in parse_env_file(&text) {
        cmd.env(k, v);
    }
}

fn command_for_app(exe: &Path, dir: &Path) -> Command {
    let mut cmd = Command::new(exe);
    cmd.current_dir(dir);
    apply_host_env(&mut cmd);
    cmd
}

pub fn replace_live(bundle: &Bundle, bytes: &[u8]) -> Result<u32, LaunchError> {
    let live = live_dir(&bundle.name);
    if live.join("app.pid").is_file() {
        if let Ok(s) = fs::read_to_string(live.join("app.pid")) {
            if let Ok(pid) = s.trim().parse::<u32>() {
                let _ = stop_pid(pid);
            }
        }
    }
    if live.exists() {
        let _ = fs::remove_dir_all(&live);
    }
    extract_to(bundle, &live)?;
    fs::write(live.join("ship.bur"), bytes)?;
    fs::write(live.join("digest"), bundle.digest.as_bytes())?;
    launch_extracted(bundle, &live, LaunchMode::Background)
}

pub fn stop_named(name: &str) -> Result<(), LaunchError> {
    let pid_path = live_dir(name).join("app.pid");
    if !pid_path.is_file() {
        return Ok(());
    }
    let s = fs::read_to_string(&pid_path)?;
    if let Ok(pid) = s.trim().parse::<u32>() {
        stop_pid(pid)?;
    }
    let _ = fs::remove_file(pid_path);
    Ok(())
}

pub fn stop_pid(pid: u32) -> Result<(), LaunchError> {
    if pid == 0 {
        return Ok(());
    }
    #[cfg(windows)]
    {
        let status = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/T", "/F"])
            .status()?;
        if !status.success() {
            // Process may already be gone.
            return Ok(());
        }
        Ok(())
    }
    #[cfg(unix)]
    {
        let status = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .status()?;
        let _ = status;
        Ok(())
    }
}

pub fn list_live() -> io::Result<Vec<(String, String, Option<u32>)>> {
    let root = crate::paths::dock_root().join("live");
    let mut out = Vec::new();
    if !root.is_dir() {
        return Ok(out);
    }
    for ent in fs::read_dir(root)? {
        let ent = ent?;
        if !ent.file_type()?.is_dir() {
            continue;
        }
        let name = ent.file_name().to_string_lossy().into_owned();
        let digest = fs::read_to_string(ent.path().join("digest"))
            .unwrap_or_default()
            .trim()
            .to_string();
        let pid = fs::read_to_string(ent.path().join("app.pid"))
            .ok()
            .and_then(|s| s.trim().parse().ok());
        out.push((name, digest, pid));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::parse_env_file;

    #[test]
    fn parse_env_file_skips_comments_and_empty() {
        let pairs = parse_env_file(
            "# secret\nBURAAQ_DATABASE_URL=postgresql://x\nBURAAQ_API_KEY=\nFOO=\"bar\"\n",
        );
        assert_eq!(
            pairs,
            vec![
                (
                    "BURAAQ_DATABASE_URL".to_string(),
                    "postgresql://x".to_string()
                ),
                ("FOO".to_string(), "bar".to_string())
            ]
        );
        assert!(!pairs.iter().any(|(k, _)| k == "BURAAQ_API_KEY"));
    }
}
