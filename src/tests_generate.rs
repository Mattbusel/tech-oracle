use super::*;

fn obs() -> Observatory {
    Observatory {
        today: "2026-06-10".into(),
        corpus: crate::observatory::Corpus::default(),
        today_terms: std::collections::HashMap::new(),
        today_sources: std::collections::HashMap::new(),
        greed: 50,
        sectors: vec![],
    }
}

#[test]
fn pick_keyword_takes_the_distinctive_token() {
    assert_eq!(pick_keyword("the rust programming language"), "programming");
}

#[test]
fn challenge_template_fills_subject() {
    let t = challenge_template("local-first sync", 3, 0);
    assert!(t.contains("local-first sync"));
    assert!(!t.contains("{s}"));
}

#[test]
fn every_call_is_well_formed() {
    let sigs = vec![
        Signal { signal_type: "hn".into(), title: "Rust takes over the backend".into(), url: "http://x".into(), momentum_score: 100.0 },
        Signal { signal_type: "arxiv".into(), title: "A new mixture of experts model".into(), url: "http://y".into(), momentum_score: 90.0 },
    ];
    let o = obs();
    for seed in 0..12i64 {
        let out = generate(&sigs, "2026-06-10", seed, 50, &o, 0.0, 0.8);
        assert_eq!(out.len(), sigs.len());
        for p in &out {
            assert!(!p.win_if.is_empty());
            assert!(!p.prediction_text.is_empty());
            assert!(MARKETS.contains(&p.market.as_str()), "unknown market {}", p.market);
            assert!((0.34..=0.9).contains(&p.confidence));
            assert_eq!(p.status, "OPEN");
            assert!(p.resolves_by > p.date);
        }
    }
}
