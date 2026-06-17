use super::*;

#[test]
fn pearson_basics() {
    assert!((pearson(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]) - 1.0).abs() < 1e-9);
    assert!((pearson(&[1.0, 2.0, 3.0], &[3.0, 2.0, 1.0]) + 1.0).abs() < 1e-9);
    assert!(pearson(&[1.0, 1.0, 1.0], &[1.0, 2.0, 3.0]).abs() < 1e-9);
}

#[test]
fn report_is_well_formed_and_deterministic() {
    let a = run(0);
    let b = run(0);
    assert_eq!(a.algos.len(), 6, "all six contenders present");
    assert!(a.samples > 100, "a real sample count, got {}", a.samples);
    // determinism: same seeds -> identical leaderboard and numbers.
    for (x, y) in a.algos.iter().zip(b.algos.iter()) {
        assert_eq!(x.name, y.name);
        assert!((x.accuracy - y.accuracy).abs() < 1e-12);
        assert!((x.ic - y.ic).abs() < 1e-12);
    }
    for al in &a.algos {
        assert!((0.0..=1.0).contains(&al.accuracy));
        assert!((-1.0..=1.0).contains(&al.ic));
        assert!((0.0..=1.0).contains(&al.brier));
        assert_eq!(al.by_regime.len(), a.regimes.len());
    }
}

#[test]
fn random_walk_is_a_coin_flip_and_others_beat_it() {
    let rep = run(0);
    let rw = rep.algos.iter().find(|a| a.name == "RANDOM WALK").unwrap();
    // The null predicts 0.5 always: no directional edge, and the best contender
    // must beat it on accuracy.
    assert!((rw.accuracy - 0.0).abs() < 1e-9 || rw.accuracy <= 0.5 + 1e-9,
        "random walk should hold no directional edge, got {}", rw.accuracy);
    let best = &rep.algos[0];
    assert!(best.accuracy > 0.5, "the leader should beat a coin flip, got {}", best.accuracy);
}

#[test]
fn print_leaderboard() {
    let rep = run(0);
    eprintln!("\n=== BENCHMARK (horizon {}d, {} topics, {} samples) ===", rep.horizon, rep.topics, rep.samples);
    eprintln!("{:<13} {:>6} {:>7} {:>7}", "ALGO", "ACC", "IC", "BRIER");
    for a in &rep.algos {
        eprintln!("{:<13} {:>5.1}% {:>7.3} {:>7.3}", a.name, a.accuracy * 100.0, a.ic, a.brier);
    }
    eprintln!("\nper-regime directional accuracy:");
    eprintln!("{:<13} {}", "ALGO", rep.regimes.iter().map(|r| format!("{:>13}", r)).collect::<String>());
    for a in &rep.algos {
        let row: String = a.by_regime.iter().map(|(_, v)| format!("{:>12.1}%", v * 100.0)).collect();
        eprintln!("{:<13} {}", a.name, row);
    }
    eprintln!();
}

#[test]
fn manifold_is_competitive() {
    let rep = run(0);
    let mani = rep.algos.iter().find(|a| a.name == "MANIFOLD").unwrap();
    // The manifold should clear a coin flip overall on this mixed suite.
    assert!(mani.accuracy > 0.5, "manifold should have a real directional edge, got {}", mani.accuracy);
    // And it should be a top-half finisher.
    let rank = rep.algos.iter().position(|a| a.name == "MANIFOLD").unwrap();
    assert!(rank < 3, "manifold should finish in the top half, ranked {}", rank);
}
