# ADR 0001: Indentation-Based Block Syntax

## Status

Superseded by [ADR 0011](./0011-brace-block-syntax.md)

## Date

2026-09-13

## Problem

Systems languages overwhelmingly use brace-delimited blocks (`{ }`). Braces add visual noise, enable style wars (K&R vs Allman), and create a gap between how beginners read code (indentation) and how parsers recognize structure (braces). Buraaq targets both beginners and systems programmers; block syntax must minimize ceremony while remaining unambiguous for tooling.

## Alternatives Considered

### A. C-style braces (required)

```buraaq
fn main() {
    if x > 0 {
        print("yes");
    }
}
```

**Pros:** Familiar to C/C++/Rust/Zig/Go programmers; easy to parse with conventional lexer.  
**Cons:** Redundant with indentation; encourages inconsistent formatting; harder for beginners.

### B. Significant indentation (Python-style)

```buraaq
fn main():
    if x > 0:
        print("yes")
```

**Pros:** Maximum readability; impossible to mis-indent without parser error; no brace matching.  
**Cons:** Whitespace sensitivity surprises some C programmers; copy-paste across editors can break blocks.

### C. Hybrid: braces optional, indentation significant

**Pros:** Migration path from C.  
**Cons:** Two valid styles; formatter must pick one anyway; doubles parser complexity.

### D. `end`-terminated blocks (Ruby/Lua style)

```buraaq
fn main()
    if x > 0
        print("yes")
    end
end
```

**Pros:** No whitespace sensitivity for block boundaries.  
**Cons:** Extra keywords; verbose; feels dated for a new language.

## Selected Design

**Alternative B: significant indentation** with these concrete rules:

- Tab width is fixed at **4 spaces** (tabs are forbidden in `.bq` source; formatter converts).
- Block-introducing tokens end with `:` — `fn`, `if`, `else`, `while`, `for`, `match`, `struct`, `enum`, `impl`, `trait`, `module`, `unsafe`, `async`, `parallel for`.
- One statement per line at the same indentation level unless using explicit `;` for multiple simple statements (discouraged; formatter splits them).
- Blank lines are insignificant.
- Continuation lines inside expressions use parenthesis wrapping; the parser does not use indentation for expression continuation.

Parentheses remain required for function calls with multiple arguments and for grouping expressions—only *blocks* are indentation-delimited.

## Advantages

- Source code reads like structured prose.
- Formatter and parser agree on structure—no "brace style" debates.
- IDE auto-indent becomes the primary navigation aid; block folding aligns with semantics.
- Beginners learn one rule: "indent inside a colon line."

## Disadvantages

- C/Rust refugees must adapt to whitespace rules.
- Generated code and naive copy-paste require formatter pass.
- Error messages for indentation mistakes must be excellent ("expected indented block after `:` on line 12").

## Performance Implications

None at runtime. Parser performs one-pass indentation stack tracking—O(n) in source length, negligible vs lexing.

## Implementation Implications

- Lexer emits `INDENT`/`DEDENT` tokens (Python model).
- Parser maintains indent stack; mismatch produces structured diagnostic with visual gutter hint.
- `buraaq fmt` is mandatory in CI templates; editor LSP formats on save by default.
- `.editorconfig` enforces `indent_size = 4`, `insert_final_newline = true`.

## Future Compatibility

If community demand for braces is overwhelming, a **non-default** `style braces` pragma may be added in v2.0—but the canonical formatter output remains indentation-based to preserve one true style. ADR can be superseded only with a major version bump.
