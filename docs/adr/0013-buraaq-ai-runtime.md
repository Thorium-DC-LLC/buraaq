# ADR 0013: Buraaq AI runtime (Mind)

- Status: Accepted
- Date: 2026-09-17

## Problem

AI serving, planning, and fine-tuning today force engineers into a zoo of Python tools, YAML, and CUDA folklore. Buraaq’s philosophy is that complexity belongs in the toolchain. Users need local inference, OpenAI-compatible serving, and a path to LoRA without making every app a Jupyter project.

## Alternatives

1. Wrap a single engine (vLLM-only) in the CLI
2. Reimplement kernels in native Buraaq immediately
3. Backend abstraction + auto planner + OpenAI-compatible facade; adapters to mature engines

## Selected design

Option 3. Product surface is `buraaq ai` and `std.ai`. A planner inspects host + model and prints an explicit deployment plan. Backends implement a small trait (`ensure_ready`, `generate`, OpenAI base URL). Serve exposes OpenAI-compatible HTTP. Training adapters (PEFT/TRL) are Phase 2 behind the same plan printer.

Stack name: **Mind**. CLI remains `buraaq ai` for grepability.

## Advantages

- One obvious way for doctor / inspect / serve / chat
- Honest quality tradeoffs (quantization stated)
- Engines replaceable without rewriting apps
- Fits Ship/Dock later via AI manifests without embedding 100GB weights

## Disadvantages

- Depends on external binaries/modules for real inference
- Phase 1 training does not execute yet
- Method sugar on `model.chat` waits on language polish

## Performance implications

Planner heuristics are estimates; benchmarks must record backend/GPU/quant. Facade adds negligible proxy overhead vs engine decode.

## Implementation implications

New crate `buraaq_ai`, CLI module `ai.rs`, runtime `buraaq_ai.c`, docs `AI.md`.

## Phase 2 (delivered)

- Managed `~/.buraaq/ai-venv` via `buraaq ai doctor --fix`
- `buraaq ai train` executes LoRA/QLoRA through PEFT/TRL (still peer Python — not native kernels)
- Serve embeddings proxy, live status scrape, `--timeout`
- Manifest `min_vram_gb` / adapter path; `ai ship` GPU preflight
- Land kit `ai-check.sh` (no driver auto-install)

## Future compatibility (Phase 3+)

Reserved: cluster scheduling, autoscaling on queue/KV metrics, signed artifacts, SGLang/MLX production adapters, DeepSpeed/full fine-tune launcher, native Buraaq CUDA kernels, `model.chat` method sugar with language polish.
