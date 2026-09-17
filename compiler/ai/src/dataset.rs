use std::fs;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::{AiError, Result};

pub fn dataset_inspect(path: &Path) -> Result<()> {
    let (n, issues, roles) = scan(path)?;
    println!("Dataset: {}", path.display());
    println!("Lines:   {n}");
    println!("Issues:  {}", issues.len());
    println!("Roles seen: {:?}", roles);
    for i in issues.iter().take(20) {
        println!("  - {i}");
    }
    if issues.len() > 20 {
        println!("  … {} more", issues.len() - 20);
    }
    Ok(())
}

pub fn dataset_validate(path: &Path) -> Result<()> {
    let (n, issues, _) = scan(path)?;
    if issues.is_empty() {
        println!("ok: {n} samples, no issues");
        Ok(())
    } else {
        for i in &issues {
            eprintln!("error: {i}");
        }
        Err(AiError::msg(format!(
            "dataset validation failed: {} issue(s) in {n} lines",
            issues.len()
        )))
    }
}

pub fn dataset_split(path: &Path, train_ratio: f64) -> Result<()> {
    let raw = fs::read_to_string(path)?;
    let lines: Vec<&str> = raw.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.is_empty() {
        return Err(AiError::msg("empty dataset"));
    }
    let n_train = ((lines.len() as f64) * train_ratio.clamp(0.5, 0.99)) as usize;
    let n_train = n_train.max(1).min(lines.len() - 1);
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("data");
    let train_path = parent.join(format!("{stem}.train.jsonl"));
    let val_path = parent.join(format!("{stem}.val.jsonl"));
    fs::write(&train_path, lines[..n_train].join("\n") + "\n")?;
    fs::write(&val_path, lines[n_train..].join("\n") + "\n")?;
    println!(
        "wrote {} ({} lines), {} ({} lines)",
        train_path.display(),
        n_train,
        val_path.display(),
        lines.len() - n_train
    );
    Ok(())
}

fn scan(path: &Path) -> Result<(usize, Vec<String>, Vec<String>)> {
    let f = fs::File::open(path)?;
    let reader = BufReader::new(f);
    let mut n = 0usize;
    let mut issues = Vec::new();
    let mut roles = Vec::new();
    let mut seen_dup = std::collections::HashSet::new();
    for (lineno, line) in reader.lines().enumerate() {
        let line = line?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        n += 1;
        if !seen_dup.insert(line.to_string()) {
            issues.push(format!("line {}: duplicate sample", lineno + 1));
        }
        match serde_json::from_str::<serde_json::Value>(line) {
            Err(e) => issues.push(format!("line {}: malformed JSON ({e})", lineno + 1)),
            Ok(v) => {
                if let Some(msgs) = v.get("messages").and_then(|m| m.as_array()) {
                    if msgs.is_empty() {
                        issues.push(format!("line {}: empty messages", lineno + 1));
                    }
                    let mut last = "";
                    for m in msgs {
                        let role = m.get("role").and_then(|r| r.as_str()).unwrap_or("");
                        let content = m.get("content").and_then(|c| c.as_str()).unwrap_or("");
                        if role.is_empty() {
                            issues.push(format!("line {}: missing role", lineno + 1));
                        }
                        if content.trim().is_empty() {
                            issues.push(format!("line {}: empty content", lineno + 1));
                        }
                        if content.len() > 100_000 {
                            issues.push(format!("line {}: oversized sequence", lineno + 1));
                        }
                        if !roles.iter().any(|r| r == role) {
                            roles.push(role.to_string());
                        }
                        if last == "assistant" && role == "assistant" {
                            issues.push(format!(
                                "line {}: bad role order (assistant twice)",
                                lineno + 1
                            ));
                        }
                        last = role;
                    }
                } else if v.get("text").and_then(|t| t.as_str()).is_some() {
                    // plain text sample ok
                } else if v.get("prompt").is_some() && v.get("completion").is_some() {
                    // instruction pair ok
                } else {
                    issues.push(format!(
                        "line {}: missing messages/text/prompt+completion",
                        lineno + 1
                    ));
                }
            }
        }
    }
    if n == 0 {
        issues.push("no samples".into());
    }
    Ok((n, issues, roles))
}
