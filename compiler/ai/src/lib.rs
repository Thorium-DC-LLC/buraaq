//! Buraaq AI — local models, planning, OpenAI-compatible serve.
//!
//! Complexity stays in the toolchain. The user types intent; the planner
//! chooses backend, precision, and layout. External engines (vLLM, llama.cpp,
//! remote OpenAI-compatible) run as peer processes — not VM isolation.

mod backend;
mod bench;
mod cache;
mod dataset;
mod doctor;
mod error;
mod land_ai;
mod model;
mod pack;
mod plan;
mod run;
mod serve;
mod status;
mod train;
mod train_runner;

pub use backend::{detect_backends, select_backend, BackendChoice, BackendInfo};
pub use bench::run_bench;
pub use cache::{cache_clean, cache_root, list_models, pull_model, CacheEntry};
pub use dataset::{dataset_inspect, dataset_split, dataset_validate};
pub use doctor::{doctor_fix, print_doctor, DoctorReport};
pub use error::AiError;
pub use land_ai::{print_land_ai_plan, write_land_ai_kit};
pub use model::{inspect_model, ModelInfo};
pub use pack::{ai_ship, write_ai_manifest};
pub use plan::{build_plan, print_plan, DeployPlan, PlanRequest};
pub use run::{run_chat, run_once};
pub use serve::{serve_model, ServeOptions};
pub use status::print_status;
pub use train::{plan_train, TrainConfig};
pub use train_runner::{ensure_train_env, run_train, train_ready, TrainOptions, venv_root};

pub type Result<T> = std::result::Result<T, AiError>;

/// Sanitize a model id for filesystem / URL path use. Rejects `..` and separators.
pub fn sanitize_model_id(id: &str) -> Result<String> {
    let t = id.trim();
    if t.is_empty() {
        return Err(AiError::msg("model id is empty"));
    }
    if t.contains("..") || t.contains('\\') || t.starts_with('/') || t.contains('\0') {
        return Err(AiError::msg(
            "invalid model id: path escapes and absolute paths are refused",
        ));
    }
    Ok(t.to_string())
}
