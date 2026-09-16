use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use buraaq_frontend::{analyze_text, DocumentAnalysis};
use tokio::sync::RwLock;
use tower_lsp::lsp_types::Url;

#[derive(Clone)]
pub struct DocumentRecord {
    pub uri: Url,
    pub path: PathBuf,
    pub version: i32,
    pub text: Arc<str>,
    pub analysis: Option<DocumentAnalysis>,
}

#[derive(Default)]
pub struct DocumentStore {
    inner: RwLock<HashMap<Url, DocumentRecord>>,
}

impl DocumentStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn open(&self, uri: Url, path: PathBuf, version: i32, text: String) {
        let text: Arc<str> = Arc::from(text);
        let analysis = analyze_text(path.clone(), text.clone());
        let record = DocumentRecord {
            uri: uri.clone(),
            path,
            version,
            text,
            analysis: Some(analysis),
        };
        self.inner.write().await.insert(uri, record);
    }

    pub async fn change(&self, uri: &Url, version: i32, text: String) -> Option<DocumentAnalysis> {
        let mut map = self.inner.write().await;
        let Some(record) = map.get_mut(uri) else {
            return None;
        };
        record.version = version;
        record.text = Arc::from(text);
        let analysis = analyze_text(record.path.clone(), record.text.clone());
        record.analysis = Some(analysis.clone());
        Some(analysis)
    }

    pub async fn close(&self, uri: &Url) {
        self.inner.write().await.remove(uri);
    }

    pub async fn get(&self, uri: &Url) -> Option<DocumentRecord> {
        self.inner.read().await.get(uri).cloned()
    }

    pub async fn all(&self) -> Vec<DocumentRecord> {
        self.inner.read().await.values().cloned().collect()
    }
}
