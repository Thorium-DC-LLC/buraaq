# Scripting mode (optional)

Buraaq’s **core** model is native AOT (`buraaq build` / `buraaq run`). No GC.

For a terminal edit loop, use **opt-in** scripting (same language, MIR interpreter):

## Check install

```bash
buraaq --version    # version, host, scripting line, exe path
buraaq doctor       # sysroot + clang + scripting probe
```

## Interactive REPL

```bash
buraaq              # or: buraaq repl
bq> println("hi")
bq> 1 + 2 * 3
6
bq> :quit
```

## One-liner / file

```bash
buraaq -e "println(\"hi\")"
buraaq script path.bq
buraaq script -e "print_int(40+2)"
```

```buraaq
#!/usr/bin/env -S buraaq script

fn main() {
    println("hello from buraaq script")
}
```

| Command | Role |
|---------|------|
| `buraaq` / `repl` | Interactive terminal |
| `buraaq -e` | Eval snippet |
| `buraaq script` | Run a `.bq` file without linking |
| `buraaq run` | Native AOT binary (product path) |

Clang is required for `run`/`build`. Scripting works with only the `buraaq` binary + sysroot.
