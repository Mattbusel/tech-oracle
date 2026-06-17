use super::*;
use crate::model::Signal;

fn sig(t: &str, title: &str, m: f64) -> Signal {
    Signal { signal_type: t.into(), title: title.into(), url: "http://x".into(), momentum_score: m }
}

#[test]
fn caps_and_dedups() {
    let sigs = vec![
        sig("hn", "Rust is taking over the backend world", 100.0),
        sig("hn", "Rust is taking over the backend world again", 90.0),
        sig("arxiv", "A new transformer architecture", 100.0),
    ];
    let picks = rank_and_select(sigs, 7, 4, &HashMap::new());
    assert!(picks.len() <= 4);
    let rusts = picks.iter().filter(|p| p.title.to_lowercase().contains("rust")).count();
    assert!(rusts <= 1, "near-duplicate topics should be deduped");
}

#[test]
fn learned_weights_change_the_order() {
    let sigs = vec![sig("hn", "alpha topic here", 100.0), sig("arxiv", "beta topic here", 100.0)];
    let mut w = HashMap::new();
    w.insert("arxiv".to_string(), 1.5);
    w.insert("hn".to_string(), 0.6);
    let picks = rank_and_select(sigs, 1, 1, &w);
    assert_eq!(picks.len(), 1);
    assert_eq!(picks[0].signal_type, "arxiv");
}

#[test]
fn empty_in_empty_out() {
    assert!(rank_and_select(vec![], 1, 4, &HashMap::new()).is_empty());
}
