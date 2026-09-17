# Mind / std.ai — run after `buraaq ai serve MODEL`

Requires Phase 1 AI serve (OpenAI-compatible) on `http://127.0.0.1:8000` or `BURAAQ_AI_BASE_URL`.

```powershell
buraaq ai doctor
buraaq ai serve Qwen/Qwen3-8B --backend openai   # if you already have an upstream
# or: buraaq ai serve … --backend llamacpp|vllm
cd examples/engineer-suite/projects/ai_chat
buraaq run
```

Docs: [docs/AI.md](../../../../docs/AI.md).
