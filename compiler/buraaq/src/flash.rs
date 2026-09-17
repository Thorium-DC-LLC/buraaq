//! `buraaq flash` — build a board UF2 and copy it to BOOTSEL (RPI-RP2).

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use buraaq_pkg::Project;

pub fn flash_project(project: &Project, board: &str) -> Result<(), String> {
    let board = normalize_board(board)?;
    let pack = board_pack_dir(&board)?;
    let sdk = ensure_pico_sdk()?;
    let gcc_bin = find_arm_gcc_bin()?;
    let build_dir = project.root.join("target").join(&board);

    fs::create_dir_all(&build_dir).map_err(|e| e.to_string())?;

    let mut path = env::var("PATH").unwrap_or_default();
    let sep = if cfg!(windows) { ";" } else { ":" };
    path = format!("{}{}{}", gcc_bin.display(), sep, path);

    eprintln!("board: {board}");
    eprintln!("sdk:   {}", sdk.display());
    eprintln!("pack:  {}", pack.display());

    let status = Command::new("cmake")
        .current_dir(&build_dir)
        .env("PATH", &path)
        .env("PICO_SDK_PATH", &sdk)
        .args([
            "-G",
            "Ninja",
            &format!("-DPICO_BOARD={board}"),
            pack.to_str().ok_or("board pack path")?,
        ])
        .status()
        .map_err(|e| format!("cmake not found: {e}"))?;
    if !status.success() {
        let status = Command::new("cmake")
            .current_dir(&build_dir)
            .env("PATH", &path)
            .env("PICO_SDK_PATH", &sdk)
            .args([
                "-G",
                "Unix Makefiles",
                &format!("-DPICO_BOARD={board}"),
                pack.to_str().ok_or("board pack path")?,
            ])
            .status()
            .map_err(|e| format!("cmake configure failed: {e}"))?;
        if !status.success() {
            return Err("cmake configure failed".into());
        }
    }

    let status = Command::new("cmake")
        .current_dir(&build_dir)
        .env("PATH", &path)
        .env("PICO_SDK_PATH", &sdk)
        .args(["--build", ".", "--config", "Release"])
        .status()
        .map_err(|e| e.to_string())?;
    if !status.success() {
        return Err("cmake build failed".into());
    }

    let uf2 = find_uf2(&build_dir).ok_or_else(|| {
        format!("no .uf2 under {}", build_dir.display())
    })?;
    eprintln!("built {}", uf2.display());

    match wait_for_bootsel(Duration::from_secs(90)) {
        Some(drive) => {
            let dest = drive.join(uf2.file_name().unwrap());
            eprintln!("flashing → {}", dest.display());
            fs::copy(&uf2, &dest).map_err(|e| e.to_string())?;
            eprintln!("done — board should reboot");
            Ok(())
        }
        None => {
            eprintln!("BOOTSEL drive not found (hold BOOTSEL while plugging USB).");
            eprintln!("UF2 ready: {}", uf2.display());
            Ok(())
        }
    }
}

fn normalize_board(raw: &str) -> Result<String, String> {
    let b = raw.trim().to_ascii_lowercase().replace('-', "_");
    match b.as_str() {
        "pico_w" | "picow" | "raspberry_pi_pico_w" => Ok("pico_w".into()),
        other => Err(format!(
            "unknown board `{other}` (supported: pico_w)"
        )),
    }
}

fn board_pack_dir(board: &str) -> Result<PathBuf, String> {
    let here = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        here.join("../boards").join(board),
        here.join("../../compiler/boards").join(board),
    ];
    for c in candidates {
        if c.join("CMakeLists.txt").exists() {
            return Ok(c.canonicalize().unwrap_or(c));
        }
    }
    Err(format!(
        "board pack `{board}` not found next to the compiler (compiler/boards/{board})"
    ))
}

fn ensure_pico_sdk() -> Result<PathBuf, String> {
    if let Ok(p) = env::var("PICO_SDK_PATH") {
        let path = PathBuf::from(p);
        if path.join("pico_sdk_init.cmake").exists() {
            return Ok(path);
        }
    }
    let home = dirs_next_home().ok_or("cannot resolve home directory")?;
    let sdk = home.join("pico-sdk");
    if sdk.join("pico_sdk_init.cmake").exists() {
        return Ok(sdk);
    }
    eprintln!("cloning pico-sdk → {}", sdk.display());
    let st = Command::new("git")
        .args([
            "clone",
            "--depth",
            "1",
            "https://github.com/raspberrypi/pico-sdk",
            sdk.to_str().unwrap(),
        ])
        .status()
        .map_err(|e| format!("git clone pico-sdk: {e}"))?;
    if !st.success() {
        return Err("git clone pico-sdk failed".into());
    }
    let st = Command::new("git")
        .current_dir(&sdk)
        .args(["submodule", "update", "--init", "--depth", "1"])
        .status()
        .map_err(|e| e.to_string())?;
    if !st.success() {
        return Err("pico-sdk submodule update failed".into());
    }
    Ok(sdk)
}

fn dirs_next_home() -> Option<PathBuf> {
    env::var_os("USERPROFILE")
        .or_else(|| env::var_os("HOME"))
        .map(PathBuf::from)
}

fn find_arm_gcc_bin() -> Result<PathBuf, String> {
    if let Ok(p) = which("arm-none-eabi-gcc") {
        if let Some(parent) = p.parent() {
            return Ok(parent.to_path_buf());
        }
    }
    let roots = [
        PathBuf::from(r"C:\Program Files (x86)"),
        PathBuf::from(r"C:\Program Files"),
    ];
    for root in roots {
        let Ok(rd) = fs::read_dir(&root) else {
            continue;
        };
        for e in rd.flatten() {
            let p = e.path();
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if p.is_dir() && n.starts_with("Arm GNU Toolchain") {
                if let Some(gcc) = find_file_named(&p, "arm-none-eabi-gcc.exe", 4) {
                    if let Some(parent) = gcc.parent() {
                        return Ok(parent.to_path_buf());
                    }
                }
            }
        }
    }
    Err(
        "arm-none-eabi-gcc not found. Install: winget install Arm.ArmGnuToolchain"
            .into(),
    )
}

fn find_file_named(dir: &Path, name: &str, depth: u32) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    let rd = fs::read_dir(dir).ok()?;
    for e in rd.flatten() {
        let p = e.path();
        if p.is_file() && p.file_name().and_then(|n| n.to_str()) == Some(name) {
            return Some(p);
        }
        if p.is_dir() {
            if let Some(hit) = find_file_named(&p, name, depth - 1) {
                return Some(hit);
            }
        }
    }
    None
}

fn which(cmd: &str) -> Result<PathBuf, ()> {
    let out = Command::new(if cfg!(windows) { "where" } else { "which" })
        .arg(cmd)
        .output()
        .map_err(|_| ())?;
    if !out.status.success() {
        return Err(());
    }
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next().ok_or(())?;
    Ok(PathBuf::from(line.trim()))
}

fn find_uf2(dir: &Path) -> Option<PathBuf> {
    fn walk(dir: &Path, out: &mut Option<PathBuf>) {
        if out.is_some() {
            return;
        }
        let Ok(rd) = fs::read_dir(dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|e| e.to_str()) == Some("uf2") {
                *out = Some(p);
                return;
            }
        }
    }
    let mut found = None;
    walk(dir, &mut found);
    found
}

fn wait_for_bootsel(timeout: Duration) -> Option<PathBuf> {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Some(d) = find_rpi_rp2() {
            return Some(d);
        }
        thread::sleep(Duration::from_millis(500));
    }
    None
}

fn find_rpi_rp2() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        // PowerShell: volume with label RPI-RP2
        let out = Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-Volume | Where-Object { $_.FileSystemLabel -eq 'RPI-RP2' } | Select-Object -First 1).DriveLetter",
            ])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        let letter = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if letter.is_empty() {
            return None;
        }
        Some(PathBuf::from(format!("{letter}:\\")))
    }
    #[cfg(not(windows))]
    {
        for candidate in ["/Volumes/RPI-RP2", "/media/RPI-RP2", "/run/media/RPI-RP2"] {
            let p = PathBuf::from(candidate);
            if p.exists() {
                return Some(p);
            }
        }
        // Common Linux mount: /media/$USER/RPI-RP2
        if let Ok(user) = env::var("USER") {
            let p = PathBuf::from(format!("/media/{user}/RPI-RP2"));
            if p.exists() {
                return Some(p);
            }
        }
        None
    }
}
