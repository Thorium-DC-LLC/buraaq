use std::time::Instant;

use crate::backend::{select_backend, GenerateRequest};
use crate::plan::{build_plan, print_plan, PlanRequest};
use crate::Result;

pub fn run_bench(
    model: &str,
    backend: &str,
    prompt: &str,
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
    println!("Benchmark methodology: single request, wall clock, approx tokens = chars/4.");
    println!("Not a synthetic win — record backend/GPU yourself from the plan above.");
    println!();

    let t0 = Instant::now();
    let mut b = select_backend(&plan)?;
    let load_ms = t0.elapsed().as_secs_f64() * 1000.0;

    let prompt = if prompt.is_empty() {
        "Write two sentences about Buraaq native compilation."
    } else {
        prompt
    };

    let t1 = Instant::now();
    let resp = b.as_mut().generate(&GenerateRequest {
        model: model.to_string(),
        prompt: prompt.to_string(),
        system: None,
        max_tokens: 256,
    })?;
    let total_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let approx_tokens = (resp.text.chars().count() as f64 / 4.0).max(1.0);
    let toks = approx_tokens / (total_ms / 1000.0);

    println!("model:           {}", model);
    println!("backend:         {}", plan.backend);
    println!("quantize:        {}", plan.quantize);
    println!("load_ms:         {:.1}", load_ms);
    println!("total_ms:        {:.1}  (includes prefill+decode; TTFT not separated in Phase 1)", total_ms);
    println!("approx_tokens:   {:.0}", approx_tokens);
    println!("approx_tok_s:    {:.1}", toks);
    println!();
    println!("--- sample ---");
    println!("{}", resp.text.chars().take(400).collect::<String>());

    let _ = b.as_mut().shutdown();
    Ok(())
}
