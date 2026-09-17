mod detect;
mod llamacpp;
mod openai;
mod vllm;

use crate::plan::DeployPlan;
use crate::Result;

pub use detect::{detect_backends, BackendInfo};
pub use llamacpp::LlamaCppBackend;
pub use openai::OpenAiCompatBackend;
pub(crate) use openai::{proxy_chat, proxy_chat_sse, proxy_completions, proxy_embeddings};
pub use vllm::VllmBackend;

#[derive(Debug, Clone)]
pub struct GenerateRequest {
    pub model: String,
    pub prompt: String,
    pub system: Option<String>,
    pub max_tokens: u32,
}

#[derive(Debug, Clone)]
pub struct GenerateResponse {
    pub text: String,
}

pub trait InferenceBackend {
    fn name(&self) -> &str;
    fn ensure_ready(&mut self, plan: &DeployPlan) -> Result<()>;
    fn openai_base_url(&self) -> Option<String>;
    fn generate(&mut self, req: &GenerateRequest) -> Result<GenerateResponse>;
    fn shutdown(&mut self) -> Result<()>;
}

pub enum BackendChoice {
    OpenAi(OpenAiCompatBackend),
    Llama(LlamaCppBackend),
    Vllm(VllmBackend),
}

impl BackendChoice {
    pub fn as_mut(&mut self) -> &mut dyn InferenceBackend {
        match self {
            Self::OpenAi(b) => b,
            Self::Llama(b) => b,
            Self::Vllm(b) => b,
        }
    }
}

pub fn select_backend(plan: &DeployPlan) -> Result<BackendChoice> {
    let mut choice = match plan.backend.as_str() {
        "vllm" => BackendChoice::Vllm(VllmBackend::new()),
        "llamacpp" | "llama.cpp" | "llama" => BackendChoice::Llama(LlamaCppBackend::new()),
        "openai" | "openai_compat" => BackendChoice::OpenAi(OpenAiCompatBackend::from_env()),
        other => {
            return Err(crate::AiError::msg(format!(
                "unknown backend `{other}` (use vllm | llamacpp | openai)"
            )));
        }
    };
    choice.as_mut().ensure_ready(plan)?;
    Ok(choice)
}
