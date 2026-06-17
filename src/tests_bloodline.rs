use super::*;

fn genes(aggr: f64, risk: f64) -> Genes {
    Genes { aggr, risk, ..Default::default() }
}

#[test]
fn simulate_is_deterministic_and_gene_sensitive() {
    let calls = vec![(0.7, true), (0.7, false), (0.6, true), (0.8, true)];
    let g = genes(0.0, 0.3);
    assert_eq!(simulate(&g, &calls, 7).bank, simulate(&g, &calls, 7).bank);
    let bold = Genes { aggr: -0.1, risk: 0.9, press: 0.8, ..Default::default() };
    assert_ne!(simulate(&g, &calls, 7).bank, simulate(&bold, &calls, 7).bank);
}

#[test]
fn plays_many_laps_for_volume() {
    // A 4-call record should still produce far more than 4 bets (it plays the
    // season many laps), and that scales down as the record grows.
    let small = vec![(0.7, true), (0.7, true), (0.6, false), (0.8, true)];
    let s = simulate(&genes(0.0, 0.4), &small, 3);
    assert!(s.bets > 10, "small record should still give volume, got {}", s.bets);
    assert_eq!(s.bets, s.wins + s.losses);
    assert!(s.peak >= 1000.0);
    let big: Vec<(f64, bool)> = (0..200).map(|i| (0.7, i % 5 != 0)).collect();
    let sb = simulate(&genes(0.0, 0.4), &big, 3);
    assert!(sb.bets <= 220, "large record plays ~one lap, got {}", sb.bets);
}

#[test]
fn a_fader_loses_against_an_accurate_oracle() {
    let calls: Vec<(f64, bool)> = (0..20).map(|i| (0.7, i % 5 != 0)).collect(); // 80% hit
    let tailer = Genes { fade: 0.0, ..Default::default() };
    let fader = Genes { fade: 1.0, ..Default::default() };
    assert!(simulate(&tailer, &calls, 5).bank > simulate(&fader, &calls, 5).bank);
}

#[test]
fn selectivity_skips_marginal_calls() {
    let calls = vec![(0.40, true), (0.40, true), (0.90, true)];
    let picky = Genes { select: 1.0, ..Default::default() }; // very high bar
    let loose = Genes { select: 0.0, ..Default::default() };
    assert!(simulate(&picky, &calls, 9).bets < simulate(&loose, &calls, 9).bets);
}

#[test]
fn houses_split_by_temperament() {
    assert_eq!(house(&genes(0.0, 0.8)), "THE PLUNGERS");
    assert_eq!(house(&genes(0.0, 0.2)), "THE MISERS");
    assert_eq!(house(&genes(0.0, 0.5)), "THE STEADY");
}

#[test]
fn champion_of_empty_is_neutral() {
    let bl = Bloodline::default();
    let g = bl.champion_genes();
    assert_eq!(g.aggr, 0.0);
    assert_eq!(g.risk, 0.0);
}

#[test]
fn crossover_stays_in_bounds() {
    let mut r = Rng(12345);
    let a = Genes { aggr: 0.1, risk: 0.9, conf: 0.05, select: 0.9, press: 0.9, fade: 0.9 };
    let b = Genes { aggr: -0.1, risk: 0.1, conf: -0.05, select: 0.1, press: 0.1, fade: 0.1 };
    for _ in 0..200 {
        let c = crossover(&a, &b, &mut r);
        assert!((-0.20..=0.20).contains(&c.aggr));
        assert!((0.0..=1.0).contains(&c.risk));
        assert!((-0.15..=0.15).contains(&c.conf));
        assert!((0.0..=1.0).contains(&c.select));
        assert!((0.0..=1.0).contains(&c.press));
        assert!((0.0..=1.0).contains(&c.fade));
    }
}

#[test]
fn evolve_is_idempotent_per_day() {
    let mut bl = Bloodline::default();
    let resolved: Vec<crate::model::Prediction> = Vec::new();
    bl.evolve_in_memory("2026-06-10", &resolved); // seed
    let gen1 = bl.gen;
    let ages1: Vec<i64> = bl.population.iter().map(|o| o.age).collect();
    bl.evolve_in_memory("2026-06-10", &resolved); // same day again
    assert_eq!(bl.gen, gen1, "gen must not advance twice in one day");
    let ages2: Vec<i64> = bl.population.iter().map(|o| o.age).collect();
    assert_eq!(ages1, ages2, "ages must not advance twice in one day");
}

#[test]
fn hall_of_fame_inducts_and_caps() {
    // Drive several generations on a judge-able record; HoF should populate and
    // never exceed its cap.
    let mut bl = Bloodline::default();
    let resolved: Vec<crate::model::Prediction> = (0..8)
        .map(|i| crate::model::Prediction {
            date: "2026-06-01".into(),
            prediction_text: "x".into(),
            source_title: "".into(),
            source_url: "".into(),
            signal_type: "hn".into(),
            status: if i % 4 == 0 { "MISS".into() } else { "HIT".into() },
            keyword: "k".into(),
            win_if: "".into(),
            resolves_by: "".into(),
            resolved_on: "".into(),
            confidence: 0.7,
            market: "RESURFACE".into(),
            keyword2: "".into(),
            target: 0,
            rationale: "".into(),
            ..Default::default()
        })
        .collect();
    bl.evolve_in_memory("2026-06-01", &resolved); // seed
    for d in 2..=9 {
        bl.evolve_in_memory(&format!("2026-06-{:02}", d), &resolved);
    }
    assert!(!bl.hall_of_fame.is_empty());
    assert!(bl.hall_of_fame.len() <= 12);
}
