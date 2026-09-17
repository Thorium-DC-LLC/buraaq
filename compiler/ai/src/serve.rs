use std::env;
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::json;

use crate::backend::{proxy_chat, proxy_chat_sse, proxy_completions, proxy_embeddings};
use crate::backend::select_backend;
use crate::plan::{build_plan, print_plan, PlanRequest};
use crate::Result;

pub struct ServeOptions {
    pub model: String,
    pub port: u16,
    pub bind: String,
    pub backend: String,
    pub quantize: Option<String>,
    pub context: Option<u32>,
    pub gpus: String,
    pub tensor_parallel: Option<u32>,
    pub api_key: Option<String>,
    /// Request read timeout seconds (default 120).
    pub timeout_secs: u64,
}

struct Metrics {
    requests: AtomicU64,
    errors: AtomicU64,
    tokens_approx: AtomicU64,
    started: Instant,
}

pub fn serve_model(opts: ServeOptions) -> Result<()> {
    let plan = build_plan(&PlanRequest {
        model: opts.model.clone(),
        backend: opts.backend.clone(),
        quantize: opts.quantize.clone(),
        context: opts.context,
        gpus: opts.gpus.clone(),
        tensor_parallel: opts.tensor_parallel,
        require_backend: true,
    })?;
    print_plan(&plan);
    println!();
    println!("Starting...");

    let mut backend = select_backend(&plan)?;
    let upstream = backend
        .as_mut()
        .openai_base_url()
        .unwrap_or_else(|| "http://127.0.0.1:8000".into());

    let api_key = opts
        .api_key
        .or_else(|| env::var("BURAAQ_AI_KEY").ok())
        .filter(|s| !s.is_empty());

    write_status_file(&StatusSnap {
        model: opts.model.clone(),
        backend: plan.backend.clone(),
        bind: format!("{}:{}", opts.bind, opts.port),
        upstream: upstream.clone(),
        healthy: true,
    })?;

    let metrics = Arc::new(Metrics {
        requests: AtomicU64::new(0),
        errors: AtomicU64::new(0),
        tokens_approx: AtomicU64::new(0),
        started: Instant::now(),
    });
    let upstream = Arc::new(upstream);
    let model = Arc::new(opts.model.clone());
    let key = Arc::new(api_key);
    let backend_hold = Arc::new(Mutex::new(backend));

    let addr = format!("{}:{}", opts.bind, opts.port);
    let listener = TcpListener::bind(&addr).map_err(|e| {
        crate::AiError::msg(format!("cannot bind AI serve on {addr}: {e}"))
    })?;
    eprintln!("buraaq ai serve");
    eprintln!("  OpenAI-compatible: http://{addr}");
    eprintln!("  upstream backend:  {}", &*upstream);
    eprintln!("  /v1/chat/completions  /v1/models  /health  /metrics");
    if key.as_ref().is_some() {
        eprintln!("  auth: API key required (BURAAQ_AI_KEY)");
    }

    let timeout = Duration::from_secs(if opts.timeout_secs == 0 {
        120
    } else {
        opts.timeout_secs
    });
    let timeout = Arc::new(timeout);

    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let upstream = Arc::clone(&upstream);
                let model = Arc::clone(&model);
                let key = Arc::clone(&key);
                let metrics = Arc::clone(&metrics);
                let timeout = Arc::clone(&timeout);
                let _hold = Arc::clone(&backend_hold);
                std::thread::spawn(move || {
                    if let Err(e) = handle_conn(
                        stream,
                        &upstream,
                        &model,
                        key.as_ref().as_deref(),
                        &metrics,
                        *timeout,
                    ) {
                        eprintln!("ai serve: {e}");
                    }
                });
            }
            Err(e) => eprintln!("ai accept: {e}"),
        }
    }
    Ok(())
}

struct StatusSnap {
    model: String,
    backend: String,
    bind: String,
    upstream: String,
    healthy: bool,
}

fn status_path() -> PathBuf {
    crate::cache::cache_root()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."))
        .join("ai")
        .join("status.json")
}

fn write_status_file(s: &StatusSnap) -> Result<()> {
    let p = status_path();
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    let v = json!({
        "model": s.model,
        "backend": s.backend,
        "bind": s.bind,
        "upstream": s.upstream,
        "healthy": s.healthy,
        "updated": SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0),
    });
    fs::write(p, serde_json::to_string_pretty(&v)?)?;
    Ok(())
}

fn handle_conn(
    mut s: TcpStream,
    upstream: &str,
    model: &str,
    api_key: Option<&str>,
    metrics: &Metrics,
    timeout: Duration,
) -> std::result::Result<(), String> {
    let _ = s.set_read_timeout(Some(timeout));
    let _ = s.set_write_timeout(Some(timeout));
    let (method, path, headers, body) = read_http(&mut s)?;
    metrics.requests.fetch_add(1, Ordering::Relaxed);

    if let Some(expected) = api_key {
        let auth = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        let bearer = auth.strip_prefix("Bearer ").unwrap_or("");
        let alt = headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case("x-api-key"))
            .map(|(_, v)| v.as_str())
            .unwrap_or("");
        if bearer != expected && alt != expected {
            metrics.errors.fetch_add(1, Ordering::Relaxed);
            return reply(&mut s, 401, "application/json", r#"{"error":"unauthorized"}"#);
        }
    }

    const MAX_BODY: usize = 8 * 1024 * 1024;
    if body.len() > MAX_BODY {
        metrics.errors.fetch_add(1, Ordering::Relaxed);
        return reply(&mut s, 413, "text/plain", "request too large");
    }

    match (method.as_str(), path.as_str()) {
        ("GET", "/health") => reply(&mut s, 200, "application/json", r#"{"status":"ok"}"#),
        ("GET", "/metrics") => {
            let up = metrics.started.elapsed().as_secs();
            let body = format!(
                "# TYPE buraaq_ai_requests counter\nburaaq_ai_requests {}\n# TYPE buraaq_ai_errors counter\nburaaq_ai_errors {}\n# TYPE buraaq_ai_tokens_approx counter\nburaaq_ai_tokens_approx {}\n# TYPE buraaq_ai_uptime_seconds gauge\nburaaq_ai_uptime_seconds {}\n",
                metrics.requests.load(Ordering::Relaxed),
                metrics.errors.load(Ordering::Relaxed),
                metrics.tokens_approx.load(Ordering::Relaxed),
                up
            );
            reply(&mut s, 200, "text/plain; version=0.0.4", &body)
        }
        ("GET", "/v1/models") => {
            let body = json!({
                "object": "list",
                "data": [{
                    "id": model,
                    "object": "model",
                    "owned_by": "buraaq"
                }]
            });
            reply(&mut s, 200, "application/json", &body.to_string())
        }
        ("POST", "/v1/chat/completions") => {
            let v: serde_json::Value =
                serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
            let want_stream = v
                .get("stream")
                .and_then(|x| x.as_bool())
                .unwrap_or(false);
            if want_stream {
                let hdr = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: close\r\n\r\n";
                s.write_all(hdr.as_bytes()).map_err(|e| e.to_string())?;
                match proxy_chat_sse(upstream, None, v, &mut s) {
                    Ok(_) => {
                        metrics.tokens_approx.fetch_add(1, Ordering::Relaxed);
                        Ok(())
                    }
                    Err(e) => {
                        metrics.errors.fetch_add(1, Ordering::Relaxed);
                        let _ = s.write_all(
                            format!("data: {{\"error\":{}}}\n\n", serde_json::to_string(&e.to_string()).unwrap_or_default())
                                .as_bytes(),
                        );
                        Ok(())
                    }
                }
            } else {
                match proxy_chat(upstream, None, v) {
                    Ok((code, text)) => {
                        metrics
                            .tokens_approx
                            .fetch_add((text.len() / 4) as u64, Ordering::Relaxed);
                        reply(&mut s, code, "application/json", &text)
                    }
                    Err(e) => {
                        metrics.errors.fetch_add(1, Ordering::Relaxed);
                        reply(
                            &mut s,
                            502,
                            "application/json",
                            &json!({"error": e.to_string()}).to_string(),
                        )
                    }
                }
            }
        }
        ("POST", "/v1/completions") => {
            let v: serde_json::Value =
                serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
            match proxy_completions(upstream, None, v) {
                Ok((code, text)) => reply(&mut s, code, "application/json", &text),
                Err(e) => {
                    metrics.errors.fetch_add(1, Ordering::Relaxed);
                    reply(
                        &mut s,
                        502,
                        "application/json",
                        &json!({"error": e.to_string()}).to_string(),
                    )
                }
            }
        }
        ("POST", "/v1/embeddings") => {
            let v: serde_json::Value =
                serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
            match proxy_embeddings(upstream, None, v) {
                Ok((code, text)) => {
                    if code == 404 || code == 501 {
                        reply(
                            &mut s,
                            501,
                            "application/json",
                            r#"{"error":"embeddings not available for this backend"}"#,
                        )
                    } else {
                        reply(&mut s, code, "application/json", &text)
                    }
                }
                Err(_) => reply(
                    &mut s,
                    501,
                    "application/json",
                    r#"{"error":"embeddings not available for this backend"}"#,
                ),
            }
        }
        _ => {
            metrics.errors.fetch_add(1, Ordering::Relaxed);
            reply(&mut s, 404, "text/plain", "not found")
        }
    }
}

fn read_http(
    s: &mut TcpStream,
) -> std::result::Result<(String, String, Vec<(String, String)>, Vec<u8>), String> {
    let mut head = Vec::new();
    let mut tmp = [0u8; 4096];
    while head.len() < 64 * 1024 {
        let n = s.read(&mut tmp).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        head.extend_from_slice(&tmp[..n]);
        if head.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }
    }
    let header_end = head
        .windows(4)
        .position(|w| w == b"\r\n\r\n")
        .ok_or_else(|| "bad request".to_string())?;
    let header = std::str::from_utf8(&head[..header_end]).unwrap_or("");
    let mut method = String::new();
    let mut path = String::new();
    let mut first = true;
    let mut content_len: usize = 0;
    let mut headers = Vec::new();
    for line in header.lines() {
        if first {
            let mut it = line.split_whitespace();
            method = it.next().unwrap_or("").to_string();
            path = it.next().unwrap_or("").to_string();
            // strip query
            if let Some((p, _)) = path.split_once('?') {
                path = p.to_string();
            }
            first = false;
            continue;
        }
        if let Some((k, v)) = line.split_once(':') {
            let kl = k.trim().to_string();
            let vl = v.trim().to_string();
            if kl.eq_ignore_ascii_case("content-length") {
                content_len = vl.parse().unwrap_or(0);
            }
            headers.push((kl, vl));
        }
    }
    let extra = head.len().saturating_sub(header_end + 4);
    let mut body = Vec::new();
    if extra > 0 {
        body.extend_from_slice(&head[header_end + 4..]);
    }
    while body.len() < content_len {
        let mut buf = vec![0u8; (content_len - body.len()).min(1 << 20)];
        let n = s.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
    }
    Ok((method, path, headers, body))
}

fn reply(
    s: &mut TcpStream,
    code: u16,
    ctype: &str,
    body: &str,
) -> std::result::Result<(), String> {
    let reason = match code {
        200 => "OK",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        _ => "Error",
    };
    let resp = format!(
        "HTTP/1.1 {code} {reason}\r\nContent-Type: {ctype}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    s.write_all(resp.as_bytes()).map_err(|e| e.to_string())
}
