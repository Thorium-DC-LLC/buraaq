//! Interactive terminal REPL for `buraaq` / `buraaq repl`.
//! Uses the same MIR interpreter as `buraaq script`.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use buraaq_diagnostics::StandardHandler;
use buraaq_driver::{compile_to_mir, BuildOptions};
use buraaq_frontend::Frontend;
use buraaq_interp::interpret_module;
use buraaq_source::SourceFile;

pub fn run_repl() -> Result<(), i32> {
    let version = env!("CARGO_PKG_VERSION");
    println!("Buraaq {version} shell");
    println!("Script mode — same language as `buraaq run`.  :help  ·  :quit");
    println!();

    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut buffer = String::new();
    let mut more = false;

    loop {
        let prompt = if more { "... " } else { "bq> " };
        print!("{prompt}");
        let _ = stdout.flush();

        let mut line = String::new();
        let n = stdin.lock().read_line(&mut line).map_err(|_| 1)?;
        if n == 0 {
            println!();
            break;
        }
        let trimmed = line.trim_end();

        if !more {
            match trimmed {
                "" => continue,
                ":q" | ":quit" | ":exit" => break,
                ":help" | ":h" | "?" => {
                    print_help();
                    continue;
                }
                ":version" | ":v" => {
                    println!("Buraaq {version}");
                    println!("mode: script (MIR interpreter)");
                    println!("native: buraaq run / buraaq build");
                    continue;
                }
                ":clear" | ":c" => {
                    buffer.clear();
                    more = false;
                    continue;
                }
                _ => {}
            }
        }

        if !buffer.is_empty() {
            buffer.push('\n');
        }
        buffer.push_str(trimmed);

        if incomplete(&buffer) {
            more = true;
            continue;
        }
        more = false;
        let src = std::mem::take(&mut buffer);
        if let Err(e) = eval_snippet(&src) {
            eprintln!("{e}");
        }
    }
    Ok(())
}

fn print_help() {
    println!(
        "Commands:
  :help / :h     this help
  :version / :v  version + mode
  :clear / :c    drop incomplete buffer
  :quit / :q     leave the REPL

Examples:
  println(\"hi\")
  print_int(1 + 2 * 3)
  mut n = 0
  while n < 3 {{
      print_int(n)
      n = n + 1
  }}

Tip: unfinished blocks continue on `...` until braces balance.
Native binary: leave REPL and run `buraaq run`."
    );
}

/// True if braces/parens/brackets are unbalanced (need more input).
fn incomplete(src: &str) -> bool {
    let mut paren = 0i32;
    let mut bracket = 0i32;
    let mut brace = 0i32;
    let mut in_str = false;
    let mut escape = false;
    for ch in src.chars() {
        if in_str {
            if escape {
                escape = false;
                continue;
            }
            if ch == '\\' {
                escape = true;
                continue;
            }
            if ch == '"' {
                in_str = false;
            }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '(' => paren += 1,
            ')' => paren -= 1,
            '[' => bracket += 1,
            ']' => bracket -= 1,
            '{' => brace += 1,
            '}' => brace -= 1,
            _ => {}
        }
    }
    paren > 0 || bracket > 0 || brace > 0
}

pub fn eval_snippet(src: &str) -> Result<(), String> {
    let wrapped = wrap_snippet(src.trim());
    let path = PathBuf::from("<repl>");
    let source = SourceFile::new(path, wrapped);
    let handler = StandardHandler::new();
    let fe = Frontend::compile_source(source, &handler);
    if fe.had_errors {
        let mut buf = Vec::new();
        let _ = handler.print_all(&fe.source, &mut buf);
        let err = String::from_utf8_lossy(&buf).to_string();
        if err.is_empty() {
            return Err("parse/type error".into());
        }
        return Err(err);
    }
    let opts = BuildOptions {
        mir_opt: false,
        ..BuildOptions::default()
    };
    let mir = compile_to_mir(&fe.ast, &opts).map_err(|e| e.to_string())?;
    interpret_module(&mir).map_err(|e| e.to_string())?;
    Ok(())
}

fn wrap_snippet(src: &str) -> String {
    let s = src.trim();
    if s.is_empty() {
        return "fn main() {}\n".into();
    }
    if s.contains("fn main") {
        return format!("{s}\n");
    }
    // Expression-ish single line → print if it's not already a call/statement form.
    if !s.contains('\n')
        && !s.ends_with('}')
        && !s.contains('=')
        && !s.starts_with("print")
        && !s.starts_with("mut ")
        && !s.starts_with("while ")
        && !s.starts_with("if ")
        && !s.starts_with("for ")
        && !s.starts_with("return ")
    {
        // Prefer print_int for numeric-looking exprs; println for strings.
        if s.contains('"') {
            return format!("fn main() {{\n    println({s})\n}}\n");
        }
        return format!("fn main() {{\n    print_int({s})\n}}\n");
    }
    format!("fn main() {{\n{s}\n}}\n")
}
