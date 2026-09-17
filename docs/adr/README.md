# Architecture Decision Records (ADRs)

Buraaq major design decisions are documented as ADRs following the [Michael Nygard format](https://cognitect.com/blog/2011/11/15/documenting-architecture-decisions).

## Index

| ADR | Title | Status |
|-----|-------|--------|
| [0001](./0001-indentation-based-syntax.md) | Indentation-Based Block Syntax | Accepted |
| [0002](./0002-guarded-flow-analysis-memory-model.md) | Guarded Flow Analysis for Memory Safety | Accepted |
| [0003](./0003-result-based-error-handling.md) | Result-Based Error Handling with `?` Propagation | Accepted |
| [0004](./0004-async-task-model.md) | Opt-In Async Task Model | Accepted |
| [0005](./0005-llvm-backend.md) | LLVM as Primary Code Generation Backend | Accepted |
| [0006](./0006-module-and-package-system.md) | File-System-Aligned Module System | Accepted |
| [0007](./0007-c-ffi-strategy.md) | C FFI Strategy — `extern c` Blocks | Accepted |
| [0008](./0008-generics-and-type-inference.md) | Parametric Generics with Local Type Inference | Accepted |
| [0009](./0009-concurrency-model.md) | Threads + Send/Sync Inference | Accepted |
| [0010](./0010-deterministic-destruction.md) | Deterministic Destruction via Drop Protocol | Accepted |
| [0011](./0011-brace-block-syntax.md) | Brace-Delimited Blocks (supersedes 0001) | Accepted |
| [0012](./0012-unified-concurrency.md) | Unified Concurrency | Accepted |
| [0013](./0013-buraaq-ai-runtime.md) | Buraaq AI runtime (Mind) | Accepted |

## Creating a New ADR

1. Copy the next number: `0011-short-title.md`
2. Include all sections: Problem, Alternatives, Selected Design, Advantages, Disadvantages, Performance Implications, Implementation Implications, Future Compatibility
3. Set status: Proposed → Accepted → Superseded
4. Link from this index

Superseded ADRs remain in place for history; new ADR references the old one.
