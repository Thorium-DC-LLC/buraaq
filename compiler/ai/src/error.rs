use std::fmt;

#[derive(Debug)]
pub struct AiError {
    message: String,
}

impl AiError {
    pub fn msg(m: impl Into<String>) -> Self {
        Self {
            message: m.into(),
        }
    }

    pub fn oom_hint(model: &str, need_gb: f64, gpus: &[(String, f64)]) -> Self {
        let mut s = format!(
            "Model cannot fit on this host.\n\n{model} estimate: {need_gb:.1} GB VRAM\n\nAvailable:\n"
        );
        if gpus.is_empty() {
            s.push_str("  (no GPU detected — CPU/RAM only)\n");
        } else {
            for (i, (name, vram)) in gpus.iter().enumerate() {
                s.push_str(&format!("  GPU {i}: {name} — {vram:.1} GB\n"));
            }
        }
        s.push_str("\nTry:\n");
        s.push_str(&format!(
            "  buraaq ai serve {model} --quantize 4bit\n"
        ));
        s.push_str(&format!("  buraaq ai serve {model} --backend llamacpp\n"));
        s.push_str("  buraaq ai doctor\n");
        Self { message: s }
    }
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AiError {}

impl From<std::io::Error> for AiError {
    fn from(e: std::io::Error) -> Self {
        Self::msg(e.to_string())
    }
}

impl From<serde_json::Error> for AiError {
    fn from(e: serde_json::Error) -> Self {
        Self::msg(e.to_string())
    }
}

impl From<ureq::Error> for AiError {
    fn from(e: ureq::Error) -> Self {
        Self::msg(e.to_string())
    }
}
