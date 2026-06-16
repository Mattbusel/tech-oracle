use serde::{Deserialize, Serialize};

/// One normalized signal, regardless of source.
#[derive(Debug, Clone)]
pub struct Signal {
    /// Machine tag used to pick a template family and human label: "hn" | "arxiv" | "github".
    pub signal_type: String,
    pub title: String,
    pub url: String,
    /// Source-native momentum (HN score, GitHub stars-today, arXiv recency rank).
    pub momentum_score: f64,
}

/// One archived prediction. This is the persisted source of truth.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub date: String, // YYYY-MM-DD
    pub prediction_text: String,
    pub source_title: String,
    pub source_url: String,
    pub signal_type: String,
}
