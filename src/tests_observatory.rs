use super::*;
use std::collections::{BTreeMap, HashMap, HashSet};

fn sample() -> Observatory {
    let mut days = Vec::new();
    for d in 1..=7 {
        let mut terms = BTreeMap::new();
        terms.insert("rust".to_string(), 1usize);
        days.push(CorpusDay {
            date: format!("2026-06-{:02}", d),
            total: 10,
            by_source: BTreeMap::new(),
            terms,
            sectors: BTreeMap::new(),
            greed: 50,
        });
    }
    let mut terms = BTreeMap::new();
    let mut stages = BTreeMap::new();
    stages.insert("arxiv".to_string(), "2026-06-01".to_string());
    stages.insert("news".to_string(), "2026-06-10".to_string());
    terms.insert(
        "rust".to_string(),
        TermRec {
            first_seen: "2026-06-01".into(),
            first_stage: 0,
            stages,
            peak: 5,
            peak_date: "2026-06-05".into(),
            days: 8,
            last_seen: "2026-06-10".into(),
            crossed: true,
            crossed_on: "2026-06-10".into(),
        },
    );
    let corpus = Corpus { days, terms };
    let mut today_terms = HashMap::new();
    today_terms.insert("rust".to_string(), 5usize);
    let mut today_sources = HashMap::new();
    let mut s = HashSet::new();
    s.insert("arxiv".to_string());
    s.insert("news".to_string());
    today_sources.insert("rust".to_string(), s);
    Observatory {
        today: "2026-06-10".into(),
        corpus,
        today_terms,
        today_sources,
        greed: 50,
        sectors: vec![],
    }
}

#[test]
fn stage_funnel_orders_technical_to_general() {
    assert_eq!(stage_of("arxiv"), 0);
    assert_eq!(stage_of("hn"), 4);
    assert_eq!(stage_of("wiki"), 9);
    assert!(is_technical(stage_of("arxiv")));
    assert!(is_general(stage_of("news")));
    assert!(!is_general(stage_of("hn")));
}

#[test]
fn counts_and_velocity() {
    let o = sample();
    assert_eq!(o.today_count("rust"), 5);
    assert_eq!(o.cross_source("rust"), 2);
    // today 5 vs trailing average 1 => +400%
    assert_eq!(o.velocity_pct("rust"), 400);
}

#[test]
fn crossing_detected_from_technical_origin_to_general_reach() {
    let o = sample();
    assert_eq!(o.origin_stage("rust"), 0);
    assert_eq!(o.reach_stage("rust"), 8);
    assert!(o.is_crossing("rust"));
    let r = o.rationale("rust");
    assert!(r.contains("VEL"));
    assert!(r.contains("CROSSING"));
}

#[test]
fn greed_label_bands() {
    let mut o = sample();
    o.greed = 10;
    assert_eq!(o.greed_label(), "EXTREME FEAR");
    o.greed = 90;
    assert_eq!(o.greed_label(), "EXTREME GREED");
    o.greed = 50;
    assert_eq!(o.greed_label(), "NEUTRAL");
}
