use super::*;

#[test]
fn neutral_below_min_points() {
    let r = analyze(&[3.0, 4.0]);
    assert!(!r.defined());
    assert_eq!(r.gamma, 1.0);
    assert_eq!(r.trend, 0.0);
    assert!((r.prob_rising() - 0.5).abs() < 1e-9);
    assert_eq!(r.confidence(), 0.5);
}

#[test]
fn is_deterministic() {
    let s = vec![2.0, 3.0, 5.0, 4.0, 7.0, 9.0, 8.0, 12.0];
    let a = analyze(&s);
    let b = analyze(&s);
    assert_eq!(a.trend, b.trend);
    assert_eq!(a.gamma, b.gamma);
    assert_eq!(a.ds2, b.ds2);
}

#[test]
fn steady_climb_forecasts_up_and_rises() {
    // A clean, sustained rise: attention growing every day.
    let s: Vec<f64> = (0..12).map(|i| 2.0 + i as f64 * 2.0).collect();
    let r = analyze(&s);
    assert!(r.defined());
    assert!(r.trend > 0.0, "a rising trajectory should forecast up, got {}", r.trend);
    assert!(r.prob_rising() > 0.5, "rising -> P(rising) above a coin flip, got {}", r.prob_rising());
}

#[test]
fn steady_decline_forecasts_down() {
    let s: Vec<f64> = (0..12).map(|i| 30.0 - i as f64 * 2.0).map(|v: f64| v.max(0.0)).collect();
    let r = analyze(&s);
    assert!(r.trend < 0.0, "a falling trajectory should forecast down, got {}", r.trend);
    assert!(r.prob_rising() < 0.5);
}

#[test]
fn gamma_is_at_least_one() {
    let s = vec![1.0, 9.0, 1.0, 12.0, 1.0, 15.0, 2.0];
    let r = analyze(&s);
    assert!(r.gamma >= 1.0);
    assert!(r.beta.abs() <= BETA_MAX + 1e-9);
}

#[test]
fn violent_series_is_not_timelike() {
    // Big alternating spikes: high normalized velocity/volatility -> ds2 rises out
    // of the clean causal (timelike) regime into transition or noise.
    let s = vec![1.0, 40.0, 1.0, 50.0, 1.0, 60.0, 1.0, 55.0];
    let r = analyze(&s);
    assert_ne!(r.regime, Regime::Timelike, "an erratic series should not read as a clean causal trend");
}

#[test]
fn calm_trend_reads_timelike() {
    // Small, smooth, consistent steps relative to the local light speed.
    let s: Vec<f64> = (0..14).map(|i| 100.0 + i as f64).collect();
    let r = analyze(&s);
    assert_eq!(r.regime, Regime::Timelike);
}

#[test]
fn regime_pools_cover_the_markets() {
    for reg in [Regime::Timelike, Regime::Lightlike, Regime::Spacelike] {
        assert!(!market_pool(reg).is_empty());
        assert!(market_pool(reg).contains(&"RESURFACE"));
    }
}

#[test]
fn confidence_stays_in_band() {
    let s = vec![2.0, 5.0, 9.0, 6.0, 11.0, 14.0, 10.0, 18.0, 22.0];
    let r = analyze(&s);
    let c = r.confidence();
    assert!((0.34..=0.92).contains(&c), "confidence out of band: {c}");
}
