use std::env;
use std::fs;
use std::process::Command;

use crate::backend::detect_backends;
use crate::cache::cache_root;

#[derive(Debug, Clone)]
pub struct GpuInfo {
    pub name: String,
    pub vram_gb: f64,
}

#[derive(Debug, Clone)]
pub struct DoctorReport {
    pub cpu: String,
    pub ram_gb: f64,
    pub gpus: Vec<GpuInfo>,
    pub cuda: Option<String>,
    pub rocm: Option<String>,
    pub metal: bool,
    pub disk_free_gb: f64,
    pub cache: String,
    pub backends: Vec<(String, bool, String)>,
}

pub fn probe_host() -> DoctorReport {
    let cpu = detect_cpu();
    let ram_gb = detect_ram_gb();
    let gpus = detect_gpus();
    let cuda = detect_cuda();
    let rocm = env::var("ROCM_PATH").ok().map(|_| "ROCm env present".into());
    let metal = cfg!(target_os = "macos");
    let cache = cache_root();
    let _ = fs::create_dir_all(&cache);
    let disk_free_gb = free_space_gb(&cache);
    let backends = detect_backends()
        .into_iter()
        .map(|b| (b.name, b.available, b.detail))
        .collect();
    DoctorReport {
        cpu,
        ram_gb,
        gpus,
        cuda,
        rocm,
        metal,
        disk_free_gb,
        cache: cache.display().to_string(),
        backends,
    }
}

pub fn print_doctor() {
    let r = probe_host();
    println!("Buraaq AI Doctor");
    println!();
    println!("Host");
    println!("  CPU: {}", r.cpu);
    println!("  RAM: {:.0} GB", r.ram_gb);
    println!();
    println!("GPU");
    if r.gpus.is_empty() {
        println!("  (none detected via nvidia-smi)");
        println!("  hint: install NVIDIA drivers + CUDA, or use --backend openai with BURAAQ_AI_BASE_URL");
    } else {
        for (i, g) in r.gpus.iter().enumerate() {
            println!("  GPU {i}: {}", g.name);
            println!("  VRAM: {:.1} GB", g.vram_gb);
        }
    }
    if let Some(c) = &r.cuda {
        println!("  CUDA: {c}");
    }
    if let Some(c) = &r.rocm {
        println!("  ROCm: {c}");
    }
    if r.metal {
        println!("  Metal: supported (host is macOS)");
    }
    println!();
    println!("Backends");
    for (name, ok, detail) in &r.backends {
        let flag = if *ok { "available" } else { "unavailable" };
        println!("  {name}: {flag} ({detail})");
    }
    println!();
    println!("Training");
    if crate::train_runner::train_ready() {
        println!(
            "  train: ready ({})",
            crate::train_runner::venv_root().display()
        );
    } else {
        println!("  train: missing (run buraaq ai doctor --fix)");
    }
    println!();
    println!("Cache");
    println!("  path: {}", r.cache);
    println!("  free disk (approx): {:.1} GB", r.disk_free_gb);
    println!();
    let ready = r.backends.iter().any(|(_, ok, _)| *ok);
    if ready {
        println!("AI runtime: ready");
    } else {
        println!("AI runtime: not ready");
        println!("  Install llama-server, vLLM, or set BURAAQ_AI_BASE_URL to an OpenAI-compatible server.");
        println!("  Then: buraaq ai serve MODEL");
    }
}

/// `buraaq ai doctor --fix` — create managed venv + training deps.
pub fn doctor_fix() -> crate::Result<()> {
    print_doctor();
    println!();
    println!("--- fix: training environment ---");
    crate::train_runner::ensure_train_env()?;
    Ok(())
}

fn detect_cpu() -> String {
    #[cfg(windows)]
    {
        if let Ok(out) = Command::new("wmic")
            .args(["cpu", "get", "Name", "/value"])
            .output()
        {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if let Some(v) = line.strip_prefix("Name=") {
                    let t = v.trim();
                    if !t.is_empty() {
                        return t.to_string();
                    }
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(raw) = fs::read_to_string("/proc/cpuinfo") {
            for line in raw.lines() {
                if let Some(v) = line.strip_prefix("model name") {
                    return v.trim().trim_start_matches(':').trim().to_string();
                }
            }
        }
    }
    format!("{} cores (approx)", num_cpus_approx())
}

fn num_cpus_approx() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

fn detect_ram_gb() -> f64 {
    #[cfg(windows)]
    {
        #[repr(C)]
        struct MemoryStatusEx {
            dw_length: u32,
            dw_memory_load: u32,
            ull_total_phys: u64,
            ull_avail_phys: u64,
            ull_total_page_file: u64,
            ull_avail_page_file: u64,
            ull_total_virtual: u64,
            ull_avail_virtual: u64,
            ull_avail_extended_virtual: u64,
        }
        #[link(name = "kernel32")]
        extern "system" {
            fn GlobalMemoryStatusEx(stat: *mut MemoryStatusEx) -> i32;
        }
        let mut st = MemoryStatusEx {
            dw_length: std::mem::size_of::<MemoryStatusEx>() as u32,
            dw_memory_load: 0,
            ull_total_phys: 0,
            ull_avail_phys: 0,
            ull_total_page_file: 0,
            ull_avail_page_file: 0,
            ull_total_virtual: 0,
            ull_avail_virtual: 0,
            ull_avail_extended_virtual: 0,
        };
        let ok = unsafe { GlobalMemoryStatusEx(&mut st) };
        if ok != 0 && st.ull_total_phys > 0 {
            return st.ull_total_phys as f64 / (1024.0 * 1024.0 * 1024.0);
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(raw) = fs::read_to_string("/proc/meminfo") {
            for line in raw.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    let kb: f64 = rest
                        .split_whitespace()
                        .next()
                        .and_then(|x| x.parse().ok())
                        .unwrap_or(0.0);
                    return kb / (1024.0 * 1024.0);
                }
            }
        }
    }
    0.0
}

fn detect_gpus() -> Vec<GpuInfo> {
    let out = Command::new("nvidia-smi")
        .args([
            "--query-gpu=name,memory.total",
            "--format=csv,noheader,nounits",
        ])
        .output();
    let Ok(out) = out else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    let s = String::from_utf8_lossy(&out.stdout);
    let mut gpus = Vec::new();
    for line in s.lines() {
        let parts: Vec<_> = line.split(',').map(|p| p.trim()).collect();
        if parts.len() >= 2 {
            let name = parts[0].to_string();
            let mb: f64 = parts[1].parse().unwrap_or(0.0);
            gpus.push(GpuInfo {
                name,
                vram_gb: mb / 1024.0,
            });
        }
    }
    gpus
}

fn detect_cuda() -> Option<String> {
    if let Ok(out) = Command::new("nvcc").args(["--version"]).output() {
        let s = String::from_utf8_lossy(&out.stdout);
        for line in s.lines() {
            if line.contains("release") {
                return Some(line.trim().to_string());
            }
        }
    }
    env::var("CUDA_PATH").ok().map(|p| format!("CUDA_PATH={p}"))
}

fn free_space_gb(path: &std::path::Path) -> f64 {
    // Best-effort; 0 if unknown.
    let _ = path;
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use std::ffi::OsStr;
        #[link(name = "kernel32")]
        extern "system" {
            fn GetDiskFreeSpaceExW(
                path: *const u16,
                free_bytes: *mut u64,
                total: *mut u64,
                total_free: *mut u64,
            ) -> i32;
        }
        let wide: Vec<u16> = OsStr::new(path)
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let mut free = 0u64;
        let mut total = 0u64;
        let mut total_free = 0u64;
        let ok = unsafe {
            GetDiskFreeSpaceExW(wide.as_ptr(), &mut free, &mut total, &mut total_free)
        };
        if ok != 0 {
            return free as f64 / (1024.0 * 1024.0 * 1024.0);
        }
    }
    0.0
}
