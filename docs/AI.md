# Buraaq AI (Mind)

Complexity stays in the toolchain. You type intent; the planner chooses backend, precision, and layout.

```bash
buraaq ai doctor
buraaq ai doctor --fix      # managed PEFT/TRL venv
buraaq ai pull Qwen/Qwen3-8B
buraaq ai inspect Qwen/Qwen3-8B
buraaq ai serve Qwen/Qwen3-8B
buraaq ai chat Qwen/Qwen3-8B
buraaq ai train --yes
buraaq land --ai
```

Stack name: **Mind**. CLI family: `buraaq ai`. Language: `std.ai`.

## Phase status

| Piece | Phase 1 | Phase 2 |
|-------|---------|---------|
| Host doctor / planner / cache | Yes | + train readiness |
| Backends vLLM / llama.cpp / OpenAI | Yes | |
| OpenAI-compatible serve + SSE | Yes | + embeddings proxy, `--timeout` |
| `std.ai` chat client | Yes | |
| Dataset inspect/validate/split | Yes | |
| Train | Plan only | **Executes** LoRA/QLoRA via managed venv |
| `ai pack` / `ai ship` | Manifest | + min_vram, adapter, GPU preflight |
| Land `--ai` | Checklist | + `ai-check.sh` in land kit |
| Live `ai status` | status.json | + scrape `/health` `/metrics` |

## Training

```bash
buraaq ai doctor --fix          # once: ~/.buraaq/ai-venv
# buraaq.ai.toml:
#   model = "Qwen/Qwen3-8B"
#   data = "train.jsonl"
#   method = "lora"
buraaq ai dataset validate train.jsonl
buraaq ai train --plan-only
buraaq ai train --yes
```

Adapters land in `target/ai/adapters/<run_id>/` with `adapter_config.json` + SHA-256. The toolchain hides pip/PEFT/TRL; you do not hand-write a Jupyter stack for the happy path.

## Isolation honesty

Backends and the train venv run as **peer processes**. Buraaq does not claim Docker/VM isolation. Land `--ai` **never** auto-installs GPU drivers.

## Planner / serve

```bash
buraaq ai serve MODEL --timeout 120
```

Endpoints: `/v1/models`, `/v1/chat/completions` (SSE if `"stream":true`), `/v1/completions`, `/v1/embeddings` (501 if upstream lacks them), `/health`, `/metrics`.

## Pack / ship / land

```bash
buraaq ai pack MODEL
buraaq ai ship MODEL HOST          # preflight + next steps (weights stay cached)
buraaq land --ai                   # writes target/land/ai-check.sh
```

Manifest fields include `min_vram_gb`, `recommended_backend`, `adapter_path`, `require_gpu` (set via `BURAAQ_AI_REQUIRE_GPU=1`).

## Language API

```buraaq
use std.ai

fn main() {
    m = ai.model("Qwen/Qwen3-8B")
    println(ai.chat(m, "Explain ownership"))
}
```

Method sugar (`model.chat`) remains a later language polish.

## Environment

| Variable | Role |
|----------|------|
| `BURAAQ_AI_CACHE` | Model cache |
| `BURAAQ_AI_VENV` | Train venv root (default `~/.buraaq/ai-venv`) |
| `BURAAQ_AI_BASE_URL` | Upstream OpenAI-compatible base |
| `BURAAQ_AI_KEY` | Optional API key |
| `BURAAQ_AI_REQUIRE_GPU` | Strict GPU preflight on `ai ship` |

See ADR [0013](adr/0013-buraaq-ai-runtime.md).
