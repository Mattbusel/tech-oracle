use super::*;

#[test]
fn simulate_is_deterministic_and_gene_sensitive() {
    let calls = vec![(0.7, true), (0.7, false), (0.6, true), (0.8, true)];
    let g = Genes { aggr: 0.0, risk: 0.3, conf: 0.0 };
    assert_eq!(simulate(&g, &calls), simulate(&g, &calls));
    let bold = Genes { aggr: -0.1, risk: 0.9, conf: 0.0 };
    assert_ne!(simulate(&g, &calls), simulate(&bold, &calls));
}

#[test]
fn houses_split_by_temperament() {
    assert_eq!(house(&Genes { aggr: 0.0, risk: 0.8, conf: 0.0 }), "THE PLUNGERS");
    assert_eq!(house(&Genes { aggr: 0.0, risk: 0.2, conf: 0.0 }), "THE MISERS");
    assert_eq!(house(&Genes { aggr: 0.0, risk: 0.5, conf: 0.0 }), "THE STEADY");
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
    let a = Genes { aggr: 0.1, risk: 0.9, conf: 0.05 };
    let b = Genes { aggr: -0.1, risk: 0.1, conf: -0.05 };
    for _ in 0..200 {
        let c = crossover(&a, &b, &mut r);
        assert!((-0.12..=0.12).contains(&c.aggr));
        assert!((0.0..=1.0).contains(&c.risk));
        assert!((-0.08..=0.08).contains(&c.conf));
    }
}

#[test]
fn evolve_is_idempotent_per_day() {
    // Seed, then evolve twice on the same date with a judge-able record: the
    // generation and ages must not advance on the second call.
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
