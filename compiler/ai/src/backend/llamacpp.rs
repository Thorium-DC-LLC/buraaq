use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::backend::detect::find_gguf;
use crate::backend::openai::OpenAiCompatBackend;
use crate::backend::{GenerateRequest, GenerateResponse, InferenceBackend};
use crate::cache::{model_dir, which};
use crate::plan::DeployPlan;
use crate::{AiError, Result};

pub struct LlamaCppBackend {
    child: Option<Child>,
    port: u16,
    inner: OpenAiCompatBackend,
}

impl LlamaCppBackend {
    pub fn new() -> Self {
        let port = 8010;
        Self {
            child: None,
            port,
            inner: OpenAiCompatBackend::with_base(format!("http://127.0.0.1:{port}")),
        }
    }
}

impl InferenceBackend for LlamaCppBackend {
    fn name(&self) -> &str {
        "llamacpp"
    }

    fn ensure_ready(&mut self, plan: &DeployPlan) -> Result<()> {
        // Already up?
        if self.inner.ensure_ready(plan).is_ok() {
            return Ok(());
        }
        let bin = which("llama-server").ok_or_else(|| {
            AiError::msg(
                "llama-server not found on PATH.\nInstall llama.cpp server or use --backend openai.",
            )
        })?;
        let dir = model_dir(&plan.model.id)?;
        let gguf = find_gguf(&dir).ok_or_else(|| {
            AiError::msg(format!(
                "no .gguf in {}.\nPull a GGUF into the cache or use vLLM/HF weights.\n  buraaq ai pull {}",
                dir.display(),
                plan.model.id
            ))
        })?;
        let mut cmd = Command::new(&bin);
        cmd.arg("-m")
            .arg(&gguf)
            .arg("--port")
            .arg(self.port.to_string())
            .arg("--host")
            .arg("127.0.0.1")
            .arg("-c")
            .arg(plan.context.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let child = cmd.spawn().map_err(|e| {
            AiError::msg(format!("failed to spawn llama-server: {e}"))
        })?;
        self.child = Some(child);
        for _ in 0..60 {
            thread::sleep(Duration::from_millis(500));
            if self.inner.ensure_ready(plan).is_ok() {
                return Ok(());
            }
        }
        Err(AiError::msg(
            "llama-server started but did not become ready in time",
        ))
    }

    fn openai_base_url(&self) -> Option<String> {
        Some(format!("http://127.0.0.1:{}", self.port))
    }

    fn generate(&mut self, req: &GenerateRequest) -> Result<GenerateResponse> {
        self.inner.generate(req)
    }

    fn shutdown(&mut self) -> Result<()> {
        if let Some(mut c) = self.child.take() {
            let _ = c.kill();
            let _ = c.wait();
        }
        Ok(())
    }
}
