# Buraaq Language Philosophy

Buraaq is a compiled systems programming language whose central thesis is:

> **Complexity belongs inside the compiler, not inside the programmer's source code.**

Systems programming must be powerful without being painful. Buraaq targets native machine-code performance comparable to C, C++, and Rust, while remaining substantially easier to read, learn, and maintain than any mainstream systems language today.

This document defines the non-negotiable principles that every language feature, compiler pass, and tooling decision must satisfy.

---

## 1. Design North Star

| Goal | What it means in practice |
|------|---------------------------|
| **Extremely simple syntax** | A beginner can read most programs and understand intent without knowing advanced concepts. Syntax uses familiar words, minimal punctuation, and consistent block structure. |
| **Native machine-code compilation** | Buraaq produces optimized native binaries. Interpretation and JIT are not part of the core execution model. |
| **Predictable performance** | No hidden GC pauses, no surprise allocations, no opaque runtime dispatch on hot paths unless explicitly requested. |
| **Zero-cost abstractions (where realistic)** | Generics, iterators, and structured control flow compile to the same code a skilled C programmer would write by hand. |
| **Memory safety by default** | Use-after-free, double-free, buffer overflow, and data races on safe code are compile-time or runtime-detected errors—not silent UB. |
| **Explicit escape hatches** | Low-level control exists behind one obvious gate: `unsafe`. |
| **No GC for normal systems code** | Deterministic destruction and ownership inference replace garbage collection for typical systems workloads. |
| **Minimal runtime** | The default runtime is tiny: panic handler, optional thread scheduler hooks, and platform glue. Pay only for what you use. |
| **Deterministic resource management** | Files, sockets, locks, and heap memory are released at predictable points—primarily scope exit and explicit transfer. |
| **Excellent concurrency** | Threads, structured parallelism, and async tasks are first-class without infecting every function signature. |
| **First-class async without async syntax everywhere** | Async is opt-in at task boundaries; synchronous code stays synchronous by default. |
| **Extremely clear compiler errors** | Diagnostics explain *what went wrong*, *why the compiler rejected it*, and *the smallest change that fixes it*. |
| **Near-zero boilerplate** | No build scripts for common cases, no manual memory management ceremony, no redundant type annotations. |
| **Fast compilation** | Incremental compilation, fine-grained caching, and parallel codegen are architectural requirements—not afterthoughts. |
| **Easy package management** | Dependencies are declared in one manifest; fetching and building are integrated into the compiler driver. |
| **Straightforward C interoperability** | Calling C and exposing Buraaq to C must not require a separate binding generator for common cases. |
| **Cross-platform compilation** | One toolchain targets Linux, macOS, Windows, and embedded profiles from the same source. |
| **Excellent tooling from day one** | LSP, formatter, package manager, and test runner ship with the compiler—not years later. |

---

## 2. The Buraaq Design Method

Every proposed feature must pass eight questions:

1. **Why does the programmer need to write this?**
2. **Could the compiler infer it safely?**
3. **Could this syntax disappear entirely?**
4. **Could two concepts become one?**
5. **Could an advanced feature remain invisible until needed?**
6. **Can the safe behavior be the default?**
7. **Can unsafe behavior require one obvious escape hatch?**
8. **Does this feature increase or decrease total system complexity** (source + compiler + runtime + tooling)?

If a feature fails questions 2–7, it is rejected or redesigned until it passes.

Buraaq deliberately **does not** copy Rust syntax merely because Rust solved a problem. Buraaq solves the same problems with different surface syntax and heavier compiler responsibility.

---

## 3. Core Architectural Pillars

### 3.1 Invisible Safety Analysis (ISA)

Memory and concurrency safety are enforced by the compiler's **Guarded Flow Analysis (GFA)**—a whole-program-capable static analysis combining:

- Linear/affine tracking for non-copy heap values
- Escape analysis for stack and heap references
- Borrow conflict detection without lifetime annotations
- Send/Sync capability inference for cross-thread values

Programmers write `let x = ...` and call methods. They do not write `'a`, `&mut`, or `Pin<...>`.

When the compiler cannot prove safety, it emits a structured error with a suggested fix—often adding `ref`, splitting a variable, or moving an `unsafe` block.

### 3.2 Deterministic Destruction (DD)

Every value has a deterministic destruction point:

- Stack values: end of enclosing scope (reverse declaration order)
- Owned heap values: scope exit or explicit ownership transfer
- Resources (`File`, `Socket`, `MutexGuard`): release on scope exit via `drop` protocol

There is no stop-the-world collector in the default model.

### 3.3 Unified Error Flow

Functions that can fail return `Result[T, E]` or use `throws Error` sugar that desugars to `Result`. Error propagation uses postfix `?`. There are no exceptions with stack unwinding in safe code.

### 3.4 Concurrency Without Contagion

- **Threads**: `spawn expr` creates an OS thread; closure captures obey Send checking.
- **Structured parallelism**: `parallel for item in collection { ... }` for fork-join workloads.
- **Async tasks**: `async { ... }` creates a lazy task; `await task` runs it. Async does not infect callers unless they choose to `await`.

### 3.5 One Obvious Unsafe Gate

All unchecked behavior—raw pointers, unchecked indexing, inline assembly, C type punning—lives inside `unsafe:` blocks or `unsafe fn` declarations. Safe code cannot accidentally perform these operations.

---

## 4. What Buraaq Optimizes For

### 4.1 Human Readability First

Code is read far more often than written. Buraaq prioritizes:

- **Significant indentation** over brace matching
- **Keyword-based declarations** (`fn`, `let`, `struct`, `enum`) over sigils
- **Postfix clarity** for null/error handling (`value?`, `list[0]!`)
- **Named types** over cryptic aliases (`text` not `&str`, `bytes` not `&[u8]` in surface API)

### 4.2 Compiler Engineering Second

The compiler is allowed to be sophisticated because:

- Users compile once; they read source forever
- Static analysis cost amortizes across millions of execution cycles
- Better analysis eliminates entire categories of runtime checks

### 4.3 Performance Third (But Non-Negotiable)

Readability never means slow code. The compiler must:

- Monomorphize generics at compile time by default
- Eliminate bounds checks on proven-safe indexing
- Lower high-level constructs to LLVM IR that optimizes equivalently to hand-written C

---

## 5. Explicit Non-Goals (v1.0)

These are intentionally out of scope for the initial language version:

| Non-goal | Rationale |
|----------|-----------|
| Gradual typing / dynamic dispatch by default | Conflicts with predictable performance |
| Built-in garbage collector | Conflicts with deterministic systems programming |
| Macro-heavy metaprogramming | Hard to tool; deferred to staged hygienic macros in v1.x |
| Full dependent types | Disproportionate complexity for systems workloads |
| Compiler-as-IDE-plugin-only | Tooling is co-developed with the language |
| Source compatibility with C/C++/Rust | Interop yes; syntax migration no |

---

## 6. Comparison Posture

Buraaq respects prior art but does not mimic it:

| Language | Buraaq learns from | Buraaq deliberately differs |
|----------|-------------------|----------------------------|
| **C** | Simple compilation model, C ABI, minimal runtime | Memory safety, modules, no preprocessor |
| **C++** | Zero-cost templates, RAII | No inheritance, SFINAE, or header files |
| **Rust** | Ownership discipline, fearless concurrency | No lifetime syntax; compiler proves lifetimes |
| **Zig** | `@`-free clarity, comptime ambition | Indentation blocks, integrated package manager |
| **Go** | Readable syntax, built-in tooling | No GC, no goroutine-only concurrency model |

---

## 7. Success Criteria

Buraaq v1.0 is successful when:

1. A programmer with no prior systems language experience can read `examples/hello.bq` and explain what it does.
2. An experienced systems programmer can stand up a production TLS API with `page`, `api`, and `run` — without writing sockets, accept loops, or SQL plumbing. The same program ships with `buraaq up` locally and `buraaq land` + `buraaq ship` on any host.
3. The same program compiles to within 10% of equivalent `-O2` C performance on LLVM benchmarks.
4. `buraaq build` requires no CMake, Makefile, or external build generator for standard projects.
5. Compiler errors for ownership violations include a fix suggestion in ≥90% of cases (measured on conformance test suite).
6. Cold incremental rebuild of a 10 KLOC project completes in under 2 seconds on a modern laptop.

---

## 8. Guiding Mantras

- **Defaults are safe. Escape hatches are loud.**
- **If the type can be inferred, don't write it.**
- **If the compiler can prove it, don't check it at runtime.**
- **If it needs a build script, the language failed.**
- **If the error message needs a wiki page, the compiler failed.**

Buraaq exists so systems programmers can focus on systems—not on the language.
