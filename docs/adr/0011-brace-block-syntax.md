# ADR 0011: Brace-Delimited Blocks (Supersedes ADR 0001)

## Status

Accepted — supersedes [ADR 0001](./0001-indentation-based-syntax.md)

## Date

2026-09-13

## Problem

PROMPT 2 refined the surface syntax toward readable pseudocode familiar to C/Go/JavaScript programmers. Significant indentation (ADR 0001) conflicts with the exemplar syntax using `{ }` blocks and creates copy-paste friction for developers migrating from brace languages.

## Alternatives Considered

### A. Keep significant indentation (ADR 0001)

Rejected for PROMPT 2: contradicts explicit design direction; harder incremental migration from brace languages.

### B. Brace-delimited blocks with mandatory formatter

**Pros:** Familiar; unambiguous for parsers; matches user exemplars.  
**Cons:** Brace matching; style debates (resolved by `buraaq fmt`).

### C. Indentation with optional braces

Rejected: two valid forms; doubles parser and formatter complexity.

## Selected Design

**Alternative B** — all blocks use `{` `}`:

```buraaq
fn add(a: int, b: int) -> int {
    a + b
}

if age >= 18 {
    print("Adult")
}
```

- Semicolons are **optional**; newline terminates statements.
- `buraaq fmt` emits K&R style (opening brace same line for fn/if/for).
- Tabs forbidden; indent width 4 spaces inside blocks.

## Advantages

- Matches pseudocode readability target from PROMPT 2.
- Simpler lexer (no INDENT/DEDENT tokens).
- IDE block folding aligns with `{` `}`.

## Disadvantages

- Slightly more punctuation than indentation-only.
- Misplaced `}` errors require good diagnostics.

## Performance Implications

None at runtime. Parser is simpler/faster without indent stack.

## Implementation Implications

- Remove INDENT/DEDENT from lexer design in COMPILER_ARCHITECTURE.
- Formatter becomes mandatory for consistent style.
- Update all examples to brace form.

## Future Compatibility

Indentation-based syntax will not return. ADR 0001 retained for history only.
