# Mind / train sample

Tiny JSONL + `buraaq.ai.toml` for Phase 2 training.

```powershell
cd examples/engineer-suite/projects/ai_train
buraaq ai dataset validate data/train.jsonl
buraaq ai train --plan-only
# Real run (needs CUDA + doctor --fix; downloads the model):
# buraaq ai doctor --fix
# buraaq ai train --yes
```

CI / engineer-suite: compile the stub binary only (`expect` path prints the hint). Docs: [docs/AI.md](../../../../docs/AI.md).
