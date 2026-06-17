//! Ranking heuristic + dedup. Cross-source comparability is achieved by
//! normalizing each signal against the max momentum within its own source,
//! then picking the top distinct items across the combined list.

use crate::model::Signal;
use std::collections::HashMap;

pub fn rank_and_select(
    signals: Vec<Signal>,
    seed: i64,
    max_picks: usize,
    weights: &HashMap<String, f64>,
) -> Vec<Signal> {
    if signals.is_empty() {
        return Vec::new();
    }

    // Per-source max momentum (>= 1.0 to avoid divide-by-zero).
    let mut max_by: HashMap<String, f64> = HashMap::new();
    for s in &signals {
        let e = max_by.entry(s.signal_type.clone()).or_insert(0.0);
        if s.momentum_score > *e {
            *e = s.momentum_score;
        }
    }

    let mut scored: Vec<(f64, Signal)> = signals
        .into_iter()
        .map(|s| {
            let max = max_by.get(&s.signal_type).copied().unwrap_or(1.0).max(1.0);
            let norm = s.momentum_score / max;
            // A negligible, date-seeded per-source nudge so ties (every source has a
            // 1.0 leader) rotate which source leads day to day instead of always HN.
            let src_idx = match s.signal_type.as_str() {
                "hn" => 0,
                "github" => 1,
                "arxiv" => 2,
                _ => 3,
            };
            let bonus = (((seed as usize).wrapping_add(src_idx)) % 3) as f64 * 1e-6;
            // The engine learns: sources whose calls actually landed get their
            // signals weighted up; chronic missers get weighted down.
            let w = weights.get(&s.signal_type).copied().unwrap_or(1.0);
            (norm * w + bonus, s)
        })
        .collect();

    scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));

    // Greedily take the top items, skipping near-duplicate topics.
    let mut picked: Vec<Signal> = Vec::new();
    for (_, s) in scored {
        if picked.len() >= max_picks {
            break;
        }
        if picked.iter().any(|p| near_duplicate(&p.title, &s.title)) {
            continue;
        }
        picked.push(s);
    }
    picked
}

/// Jaccard token overlap on lowercased word sets; > 0.6 means "same topic".
fn near_duplicate(a: &str, b: &str) -> bool {
    use std::collections::HashSet;
    let ta: HashSet<String> = tokens(a);
    let tb: HashSet<String> = tokens(b);
    if ta.is_empty() || tb.is_empty() {
        return false;
    }
    let inter = ta.intersection(&tb).count() as f64;
    let union = ta.union(&tb).count() as f64;
    inter / union > 0.6
}

fn tokens(s: &str) -> std::collections::HashSet<String> {
    const STOP: &[&str] = &[
        "the", "a", "an", "and", "or", "for", "to", "of", "in", "on", "with", "is", "are",
        "via", "using", "show", "hn", "new",
    ];
    s.to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() > 2 && !STOP.contains(w))
        .map(|w| w.to_string())
        .collect()
}
