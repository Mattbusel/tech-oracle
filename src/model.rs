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

    // Self-grading scorecard. Every call carries a concrete, machine-checkable
    // win condition; the engine resolves it against later signals.
    #[serde(default)]
    pub status: String, // "OPEN" | "HIT" | "MISS"
    #[serde(default)]
    pub keyword: String, // token watched for resurfacing
    #[serde(default)]
    pub win_if: String, // human-readable win condition
    #[serde(default)]
    pub resolves_by: String, // YYYY-MM-DD deadline
    #[serde(default)]
    pub resolved_on: String, // YYYY-MM-DD when settled, or ""
    #[serde(default)]
    pub confidence: f64, // 0.50-0.95; sets the line and the stake payout
    #[serde(default)]
    pub market: String, // RESURFACE | SURVIVAL | MOMENTUM | HEAD-TO-HEAD | INDEX
    #[serde(default)]
    pub keyword2: String, // the other side, for HEAD-TO-HEAD
    #[serde(default)]
    pub target: i64, // index threshold, for INDEX bets
}
