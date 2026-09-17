use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

use crate::backend::openai::OpenAiCompatBackend;
use crate::backend::{GenerateRequest, GenerateResponse, InferenceBackend};
use crate::plan::DeployPlan;
use crate::{AiError, Result};

pub struct VllmBackend {
    child: Option<Child>,
    port: u16,
    inner: OpenAiCompatBackend,
}

impl VllmBackend {
    pub fn new() -> Self {
        let port = 8020;
        Self {
            child: None,
            port,
            inner: OpenAiCompatBackend::with_base(format!("http://127.0.0.1:{port}")),
        }
    }
}

impl InferenceBackend for VllmBackend {
    fn name(&self) -> &str {
        "vllm"
    }

    fn ensure_ready(&mut self, plan: &DeployPlan) -> Result<()> {
        if self.inner.ensure_ready(plan).is_ok() {
            return Ok(());
        }
        // Prefer `vllm serve` CLI when present.
        let mut cmd = Command::new("vllm");
        cmd.arg("serve")
            .arg(&plan.model.id)
            .arg("--port")
            .arg(self.port.to_string())
            .arg("--host")
            .arg("127.0.0.1");
        if plan.tensor_parallel > 1 {
            cmd.arg("--tensor-parallel-size")
                .arg(plan.tensor_parallel.to_string());
        }
        cmd.stdout(Stdio::null()).stderr(Stdio::null());
        match cmd.spawn() {
            Ok(child) => {
                self.child = Some(child);
            }
            Err(_) => {
                // Fallback: python -m vllm.entrypoints.openai.api_server
                let mut py = Command::new("python");
                py.args([
                    "-m",
                    "vllm.entrypoints.openai.api_server",
                    "--model",
                    &plan.model.id,
                    "--port",
                    &self.port.to_string(),
                    "--host",
                    "127.0.0.1",
                ]);
                if plan.tensor_parallel > 1 {
                    py.arg("--tensor-parallel-size")
                        .arg(plan.tensor_parallel.to_string());
                }
                py.stdout(Stdio::null()).stderr(Stdio::null());
                let child = py.spawn().map_err(|e| {
                    AiError::msg(format!(
                        "failed to start vLLM ({e}). Install vLLM or use --backend llamacpp|openai."
                    ))
                })?;
                self.child = Some(child);
            }
        }
        for _ in 0..120 {
            thread::sleep(Duration::from_millis(1000));
            if self.inner.ensure_ready(plan).is_ok() {
                return Ok(());
            }
        }
        Err(AiError::msg(
            "vLLM started but did not become ready in time",
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
