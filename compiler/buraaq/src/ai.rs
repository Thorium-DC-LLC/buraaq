//! `buraaq ai` — Mind: local models, planning, OpenAI-compatible serve.

use std::env;
use std::path::PathBuf;

type CmdResult = Result<(), i32>;

pub fn cmd_ai(args: &[String]) -> CmdResult {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("help");
    match sub {
        "help" | "-h" | "--help" => {
            print_ai_usage();
            Ok(())
        }
        "doctor" => {
            if args.iter().any(|a| a == "--fix") {
                buraaq_ai::doctor_fix().map_err(|e| {
                    eprintln!("error: {e}");
                    1
                })
            } else {
                buraaq_ai::print_doctor();
                Ok(())
            }
        }
        "models" | "cache" => cmd_models_cache(args),
        "pull" => {
            let model = args.get(1).ok_or_else(|| {
                eprintln!("usage: buraaq ai pull MODEL");
                1
            })?;
            match buraaq_ai::pull_model(model) {
                Ok(e) => {
                    println!("cached {} ({} bytes)", e.id, e.bytes);
                    if let Some(h) = e.sha256 {
                        println!("sha256 {h}");
                    }
                    Ok(())
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    Err(1)
                }
            }
        }
        "inspect" => {
            let model = args.get(1).ok_or_else(|| {
                eprintln!("usage: buraaq ai inspect MODEL");
                1
            })?;
            match buraaq_ai::inspect_model(model) {
                Ok(info) => {
                    info.print();
                    Ok(())
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    Err(1)
                }
            }
        }
        "run" => cmd_run(args),
        "chat" => cmd_chat(args),
        "serve" => cmd_serve(args),
        "bench" => cmd_bench(args),
        "status" => buraaq_ai::print_status().map_err(|e| {
            eprintln!("error: {e}");
            1
        }),
        "train" => {
            let cfg = flag_value(args, "--config").map(PathBuf::from);
            let plan_only = args.iter().any(|a| a == "--plan-only");
            let yes = args.iter().any(|a| a == "--yes" || a == "-y");
            if plan_only && !yes {
                return buraaq_ai::plan_train(cfg.as_deref()).map_err(|e| {
                    eprintln!("error: {e}");
                    1
                });
            }
            buraaq_ai::run_train(buraaq_ai::TrainOptions {
                config_path: cfg,
                plan_only,
                yes,
                adapter_hint: None,
            })
            .map_err(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        "dataset" => cmd_dataset(&args[1..]),
        "pack" => {
            let model = args
                .get(1)
                .cloned()
                .or_else(|| env::var("BURAAQ_AI_MODEL").ok())
                .ok_or_else(|| {
                    eprintln!("usage: buraaq ai pack MODEL");
                    1
                })?;
            let out = PathBuf::from("target/ai");
            buraaq_ai::write_ai_manifest(&model, &out).map_err(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        "ship" => {
            let model = positional_model(args)
                .or_else(|| env::var("BURAAQ_AI_MODEL").ok())
                .ok_or_else(|| {
                    eprintln!("usage: buraaq ai ship MODEL HOST");
                    1
                })?;
            // host: last non-flag after model
            let host = args
                .iter()
                .skip(1)
                .filter(|a| !a.starts_with('-'))
                .nth(1)
                .cloned()
                .ok_or_else(|| {
                    eprintln!("usage: buraaq ai ship MODEL HOST");
                    1
                })?;
            buraaq_ai::ai_ship(&model, &host).map_err(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        other => {
            eprintln!("error: unknown ai command `{other}`");
            print_ai_usage();
            Err(1)
        }
    }
}

fn print_ai_usage() {
    eprintln!("Buraaq AI (Mind) — models without the ML zoo in your face");
    eprintln!();
    eprintln!("  buraaq ai doctor              Host / GPU / backend report");
    eprintln!("  buraaq ai doctor --fix         Install managed train venv (PEFT/TRL)");
    eprintln!("  buraaq ai models              List cached models");
    eprintln!("  buraaq ai pull MODEL          Fetch metadata (+ weights if hf CLI present)");
    eprintln!("  buraaq ai inspect MODEL       Params, VRAM estimates, cache status");
    eprintln!("  buraaq ai run MODEL -p TEXT   One-shot inference");
    eprintln!("  buraaq ai chat MODEL          Interactive chat");
    eprintln!("  buraaq ai serve MODEL         OpenAI-compatible HTTP API");
    eprintln!("  buraaq ai bench MODEL         Honest load + throughput sample");
    eprintln!("  buraaq ai status              Live /health + /metrics + GPUs");
    eprintln!("  buraaq ai cache clean         Wipe ~/.buraaq/models");
    eprintln!("  buraaq ai train [--yes]       Plan + run LoRA/QLoRA (PEFT/TRL)");
    eprintln!("  buraaq ai train --plan-only   Print plan only");
    eprintln!("  buraaq ai dataset inspect|validate|split FILE");
    eprintln!("  buraaq ai pack MODEL          Write target/ai/manifest.json");
    eprintln!("  buraaq ai ship MODEL HOST     Manifest + GPU preflight for Dock");
    eprintln!();
    eprintln!("Flags: --backend auto|vllm|llamacpp|openai  --quantize none|8bit|4bit");
    eprintln!("       --gpus auto|0,1  --port 8000  --bind 127.0.0.1  --api-key KEY");
    eprintln!("Env:   BURAAQ_AI_BASE_URL  BURAAQ_AI_KEY  BURAAQ_AI_CACHE  BURAAQ_AI_VENV");
    eprintln!("       BURAAQ_AI_REQUIRE_GPU");
}

fn cmd_models_cache(args: &[String]) -> CmdResult {
    if args.get(1).map(|s| s.as_str()) == Some("clean")
        || (args.first().map(|s| s.as_str()) == Some("cache")
            && args.get(1).map(|s| s.as_str()) == Some("clean"))
    {
        return match buraaq_ai::cache_clean() {
            Ok(n) => {
                println!("cleared ~{n} bytes from {}", buraaq_ai::cache_root().display());
                Ok(())
            }
            Err(e) => {
                eprintln!("error: {e}");
                Err(1)
            }
        };
    }
    match buraaq_ai::list_models() {
        Ok(list) => {
            if list.is_empty() {
                println!("(no cached models in {})", buraaq_ai::cache_root().display());
                println!("hint: buraaq ai pull MODEL");
            } else {
                println!("Cache: {}", buraaq_ai::cache_root().display());
                for e in list {
                    println!(
                        "  {}  ({:.1} MB){}",
                        e.id,
                        e.bytes as f64 / (1024.0 * 1024.0),
                        e.sha256
                            .as_ref()
                            .map(|h| format!("  sha256:{}…", &h[..h.len().min(12)]))
                            .unwrap_or_default()
                    );
                }
            }
            Ok(())
        }
        Err(e) => {
            eprintln!("error: {e}");
            Err(1)
        }
    }
}

fn common_flags(args: &[String]) -> (String, Option<String>, String, Option<u32>, Option<u32>) {
    let backend = flag_value(args, "--backend").unwrap_or_else(|| "auto".into());
    let quantize = flag_value(args, "--quantize");
    let gpus = flag_value(args, "--gpus").unwrap_or_else(|| "auto".into());
    let context = flag_value(args, "--context").and_then(|s| s.parse().ok());
    let tp = flag_value(args, "--tensor-parallel").and_then(|s| s.parse().ok());
    (backend, quantize, gpus, context, tp)
}

fn cmd_run(args: &[String]) -> CmdResult {
    let model = positional_model(args).ok_or_else(|| {
        eprintln!("usage: buraaq ai run MODEL -p \"prompt\"");
        1
    })?;
    let prompt = flag_value(args, "-p")
        .or_else(|| flag_value(args, "--prompt"))
        .ok_or_else(|| {
            eprintln!("error: pass -p \"prompt\"");
            1
        })?;
    let (backend, quantize, gpus, _, _) = common_flags(args);
    buraaq_ai::run_once(&model, &prompt, &backend, quantize, &gpus).map_err(|e| {
        eprintln!("error: {e}");
        1
    })
}

fn cmd_chat(args: &[String]) -> CmdResult {
    let model = positional_model(args).ok_or_else(|| {
        eprintln!("usage: buraaq ai chat MODEL");
        1
    })?;
    let (backend, quantize, gpus, _, _) = common_flags(args);
    buraaq_ai::run_chat(&model, &backend, quantize, &gpus).map_err(|e| {
        eprintln!("error: {e}");
        1
    })
}

fn cmd_serve(args: &[String]) -> CmdResult {
    let model = positional_model(args).ok_or_else(|| {
        eprintln!("usage: buraaq ai serve MODEL [--port 8000]");
        1
    })?;
    let (backend, quantize, gpus, context, tp) = common_flags(args);
    let port: u16 = flag_value(args, "--port")
        .and_then(|s| s.parse().ok())
        .unwrap_or(8000);
    let bind = flag_value(args, "--bind").unwrap_or_else(|| "127.0.0.1".into());
    let api_key = flag_value(args, "--api-key");
    let timeout_secs: u64 = flag_value(args, "--timeout")
        .and_then(|s| s.parse().ok())
        .unwrap_or(120);
    let opts = buraaq_ai::ServeOptions {
        model,
        port,
        bind,
        backend,
        quantize,
        context,
        gpus,
        tensor_parallel: tp,
        api_key,
        timeout_secs,
    };
    buraaq_ai::serve_model(opts).map_err(|e| {
        eprintln!("error: {e}");
        1
    })
}

fn cmd_bench(args: &[String]) -> CmdResult {
    let model = positional_model(args).ok_or_else(|| {
        eprintln!("usage: buraaq ai bench MODEL");
        1
    })?;
    let prompt = flag_value(args, "-p")
        .or_else(|| flag_value(args, "--prompt"))
        .unwrap_or_default();
    let (backend, quantize, gpus, _, _) = common_flags(args);
    buraaq_ai::run_bench(&model, &backend, &prompt, quantize, &gpus).map_err(|e| {
        eprintln!("error: {e}");
        1
    })
}

fn cmd_dataset(args: &[String]) -> CmdResult {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("help");
    let file = args.get(1).map(PathBuf::from);
    match sub {
        "inspect" => {
            let p = file.ok_or_else(|| {
                eprintln!("usage: buraaq ai dataset inspect FILE");
                1
            })?;
            buraaq_ai::dataset_inspect(&p).map_err(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        "validate" => {
            let p = file.ok_or_else(|| {
                eprintln!("usage: buraaq ai dataset validate FILE");
                1
            })?;
            buraaq_ai::dataset_validate(&p).map_err(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        "split" => {
            let p = file.ok_or_else(|| {
                eprintln!("usage: buraaq ai dataset split FILE");
                1
            })?;
            let ratio = flag_value(args, "--ratio")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.9);
            buraaq_ai::dataset_split(&p, ratio).map_err(|e| {
                eprintln!("error: {e}");
                1
            })
        }
        _ => {
            eprintln!("usage: buraaq ai dataset inspect|validate|split FILE");
            Err(1)
        }
    }
}

fn positional_model(args: &[String]) -> Option<String> {
    // args[0] is subcommand; first non-flag token after that
    let mut i = 1;
    while i < args.len() {
        let a = &args[i];
        if a.starts_with('-') {
            if matches!(
                a.as_str(),
                "--backend"
                    | "--quantize"
                    | "--gpus"
                    | "--port"
                    | "--bind"
                    | "--api-key"
                    | "--timeout"
                    | "--context"
                    | "--tensor-parallel"
                    | "--config"
                    | "--ratio"
                    | "-p"
                    | "--prompt"
            ) {
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        return Some(a.clone());
    }
    None
}

fn flag_value(args: &[String], name: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            return args.get(i + 1).cloned();
        }
        if let Some(rest) = args[i].strip_prefix(&format!("{name}=")) {
            return Some(rest.to_string());
        }
        i += 1;
    }
    None
}
