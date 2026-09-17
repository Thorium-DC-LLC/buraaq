use std::io::{self, BufRead, Write};

use crate::backend::{select_backend, GenerateRequest};
use crate::plan::{build_plan, print_plan, PlanRequest};
use crate::Result;

pub fn run_once(
    model: &str,
    prompt: &str,
    backend: &str,
    quantize: Option<String>,
    gpus: &str,
) -> Result<()> {
    let plan = build_plan(&PlanRequest {
        model: model.to_string(),
        backend: backend.to_string(),
        quantize,
        context: None,
        gpus: gpus.to_string(),
        tensor_parallel: None,
        require_backend: true,
    })?;
    print_plan(&plan);
    println!();
    let mut b = select_backend(&plan)?;
    let resp = b.as_mut().generate(&GenerateRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        system: None,
        max_tokens: 1024,
    })?;
    println!("{}", resp.text);
    let _ = b.as_mut().shutdown();
    Ok(())
}

pub fn run_chat(
    model: &str,
    backend: &str,
    quantize: Option<String>,
    gpus: &str,
) -> Result<()> {
    let plan = build_plan(&PlanRequest {
        model: model.to_string(),
        backend: backend.to_string(),
        quantize,
        context: None,
        gpus: gpus.to_string(),
        tensor_parallel: None,
        require_backend: true,
    })?;
    print_plan(&plan);
    println!();
    println!("Buraaq AI chat — model {model}");
    println!("Type :quit to exit.");
    let mut b = select_backend(&plan)?;
    let stdin = io::stdin();
    loop {
        print!("you> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            break;
        }
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t == ":quit" || t == ":exit" || t == ":q" {
            break;
        }
        match b.as_mut().generate(&GenerateRequest {
            model: model.to_string(),
            prompt: t.to_string(),
            system: Some("You are a helpful assistant running inside Buraaq AI.".into()),
            max_tokens: 1024,
        }) {
            Ok(r) => println!("ai> {}\n", r.text),
            Err(e) => eprintln!("error: {e}"),
        }
    }
    let _ = b.as_mut().shutdown();
    Ok(())
}
