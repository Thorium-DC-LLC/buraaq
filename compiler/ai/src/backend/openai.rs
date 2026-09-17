use std::env;
use std::time::Duration;

use serde_json::json;

use crate::backend::{GenerateRequest, GenerateResponse, InferenceBackend};
use crate::plan::DeployPlan;
use crate::{AiError, Result};

pub struct OpenAiCompatBackend {
    pub base_url: String,
    pub api_key: Option<String>,
}

impl OpenAiCompatBackend {
    pub fn from_env() -> Self {
        let base = env::var("BURAAQ_AI_BASE_URL").unwrap_or_default();
        let api_key = env::var("BURAAQ_AI_KEY").ok().filter(|s| !s.is_empty());
        Self {
            base_url: base.trim_end_matches('/').to_string(),
            api_key,
        }
    }

    pub fn with_base(base: String) -> Self {
        Self {
            base_url: base.trim_end_matches('/').to_string(),
            api_key: env::var("BURAAQ_AI_KEY").ok().filter(|s| !s.is_empty()),
        }
    }
}

impl InferenceBackend for OpenAiCompatBackend {
    fn name(&self) -> &str {
        "openai"
    }

    fn ensure_ready(&mut self, _plan: &DeployPlan) -> Result<()> {
        if self.base_url.is_empty() {
            return Err(AiError::msg(
                "BURAAQ_AI_BASE_URL is unset.\nPoint it at an OpenAI-compatible server, or use --backend llamacpp|vllm.",
            ));
        }
        let url = format!("{}/v1/models", self.base_url);
        let mut req = ureq::get(&url).timeout(Duration::from_secs(5));
        if let Some(k) = &self.api_key {
            req = req.set("Authorization", &format!("Bearer {k}"));
        }
        match req.call() {
            Ok(_) => Ok(()),
            Err(e) => Err(AiError::msg(format!(
                "OpenAI-compatible server not reachable at {}\n  {e}\n  Set BURAAQ_AI_BASE_URL to a running upstream (not the buraaq ai serve port).",
                self.base_url
            ))),
        }
    }

    fn openai_base_url(&self) -> Option<String> {
        Some(self.base_url.clone())
    }

    fn generate(&mut self, req: &GenerateRequest) -> Result<GenerateResponse> {
        let url = format!("{}/v1/chat/completions", self.base_url);
        let mut messages = Vec::new();
        if let Some(sys) = &req.system {
            messages.push(json!({"role": "system", "content": sys}));
        }
        messages.push(json!({"role": "user", "content": req.prompt}));
        let body = json!({
            "model": req.model,
            "messages": messages,
            "max_tokens": req.max_tokens,
            "stream": false,
        });
        let mut http = ureq::post(&url)
            .timeout(Duration::from_secs(300))
            .set("Content-Type", "application/json");
        if let Some(k) = &self.api_key {
            http = http.set("Authorization", &format!("Bearer {k}"));
        }
        let resp = http.send_json(body)?;
        let v: serde_json::Value = resp.into_json()?;
        let text = v["choices"][0]["message"]["content"]
            .as_str()
            .or_else(|| v["choices"][0]["text"].as_str())
            .unwrap_or("")
            .to_string();
        Ok(GenerateResponse { text })
    }

    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
}

/// POST JSON to an OpenAI-compatible chat endpoint (shared by serve proxy).
pub fn proxy_chat(
    upstream: &str,
    api_key: Option<&str>,
    body: serde_json::Value,
) -> Result<(u16, String)> {
    let url = format!("{}/v1/chat/completions", upstream.trim_end_matches('/'));
    let mut http = ureq::post(&url)
        .timeout(Duration::from_secs(300))
        .set("Content-Type", "application/json");
    if let Some(k) = api_key {
        http = http.set("Authorization", &format!("Bearer {k}"));
    }
    match http.send_json(body) {
        Ok(resp) => {
            let status = resp.status();
            let text = resp.into_string().unwrap_or_default();
            Ok((status, text))
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            Ok((code, text))
        }
        Err(e) => Err(AiError::msg(e.to_string())),
    }
}

/// Stream chat completions (SSE) from upstream into `out`. Returns HTTP status.
pub fn proxy_chat_sse(
    upstream: &str,
    api_key: Option<&str>,
    mut body: serde_json::Value,
    out: &mut dyn std::io::Write,
) -> Result<u16> {
    if let Some(obj) = body.as_object_mut() {
        obj.insert("stream".into(), serde_json::Value::Bool(true));
    }
    let url = format!("{}/v1/chat/completions", upstream.trim_end_matches('/'));
    let mut http = ureq::post(&url)
        .timeout(Duration::from_secs(600))
        .set("Content-Type", "application/json")
        .set("Accept", "text/event-stream");
    if let Some(k) = api_key {
        http = http.set("Authorization", &format!("Bearer {k}"));
    }
    match http.send_json(body) {
        Ok(resp) => {
            let status = resp.status();
            let mut reader = resp.into_reader();
            let mut buf = [0u8; 8192];
            loop {
                match std::io::Read::read(&mut reader, &mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        out.write_all(&buf[..n])
                            .map_err(|e| AiError::msg(e.to_string()))?;
                        let _ = out.flush();
                    }
                    Err(e) => return Err(AiError::msg(e.to_string())),
                }
            }
            Ok(status)
        }
        Err(ureq::Error::Status(code, resp)) => {
            let text = resp.into_string().unwrap_or_default();
            let _ = out.write_all(text.as_bytes());
            Ok(code)
        }
        Err(e) => Err(AiError::msg(e.to_string())),
    }
}

pub fn proxy_completions(
    upstream: &str,
    api_key: Option<&str>,
    body: serde_json::Value,
) -> Result<(u16, String)> {
    let url = format!("{}/v1/completions", upstream.trim_end_matches('/'));
    let mut http = ureq::post(&url)
        .timeout(Duration::from_secs(300))
        .set("Content-Type", "application/json");
    if let Some(k) = api_key {
        http = http.set("Authorization", &format!("Bearer {k}"));
    }
    match http.send_json(body) {
        Ok(resp) => Ok((resp.status(), resp.into_string().unwrap_or_default())),
        Err(ureq::Error::Status(code, resp)) => {
            Ok((code, resp.into_string().unwrap_or_default()))
        }
        Err(e) => Err(AiError::msg(e.to_string())),
    }
}

pub fn proxy_embeddings(
    upstream: &str,
    api_key: Option<&str>,
    body: serde_json::Value,
) -> Result<(u16, String)> {
    let url = format!("{}/v1/embeddings", upstream.trim_end_matches('/'));
    let mut http = ureq::post(&url)
        .timeout(Duration::from_secs(120))
        .set("Content-Type", "application/json");
    if let Some(k) = api_key {
        http = http.set("Authorization", &format!("Bearer {k}"));
    }
    match http.send_json(body) {
        Ok(resp) => Ok((resp.status(), resp.into_string().unwrap_or_default())),
        Err(ureq::Error::Status(code, resp)) => {
            Ok((code, resp.into_string().unwrap_or_default()))
        }
        Err(e) => Err(AiError::msg(e.to_string())),
    }
}
