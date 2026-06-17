//! THE PROVING GROUND: a head-to-head forecasting benchmark of the manifold
//! against the canonical algorithms it is implicitly competing with.
//!
//! The task is identical for every contender: given a topic's attention series up
//! to day t, output P(attention is higher H days from now). We score each on a
//! controlled suite of synthetic topics whose regimes are known, so the test is
//! transparent and not rigged: smooth trends (where simple momentum should tie),
//! mean reversion, regime switches and viral spikes (where curvature and the
//! relativistic regime should matter), and pure noise (where everyone should land
//! at a coin flip). Metrics: directional accuracy, the information coefficient
//! (correlation of the score with the realized forward move, the standard quant
//! yardstick), and the Brier score (probability calibration, lower is better).
//!
//! The contenders, and what each represents in the wild:
//!   - MANIFOLD     this engine: relativistic geometry of the attention path.
//!   - MOMENTUM     EWMA trend-following: the core of "what's hot" feed ranking.
//!   - MA-CROSS     short vs long moving average: classic technical analysis.
//!   - PAGERANK     centrality on the co-movement graph: Google's importance-by-structure.
//!   - POPULARITY   bet on what is already big: the recommender-system reflex.
//!   - RANDOM WALK  the efficient-market null everything must beat.
//!
//! Deterministic (fixed seeds), dependency-free, recomputed every run.

const HORIZON: usize = 7; // forecast this many days ahead
const WARMUP: usize = 20; // need this much history before the first forecast
const SERIES_LEN: usize = 70; // days per synthetic topic
const PER_REGIME: usize = 8; // topics per regime

/// xorshift64; deterministic, dependency-free.
struct R(u64);
impl R {
    fn u(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    /// Approximately-normal noise (sum of uniforms), mean 0, sd ~1.
    fn z(&mut self) -> f64 {
        (0..12).map(|_| self.u()).sum::<f64>() - 6.0
    }
}

const REGIMES: &[&str] = &["TREND", "DECLINE", "MEAN-REVERT", "REGIME-SWITCH", "VIRAL", "NOISE"];

/// Generate one topic's attention-count series under a named regime.
fn gen(regime: &str, r: &mut R) -> Vec<f64> {
    let n = SERIES_LEN;
    let mut lvl = 1.6 + r.u() * 0.6; // log-attention level
    let mut out = Vec::with_capacity(n);
    let mut vel = 0.0_f64;
    for t in 0..n {
        match regime {
            "TREND" => lvl += 0.035 + 0.05 * r.z(),
            "DECLINE" => lvl += -0.035 + 0.05 * r.z(),
            "MEAN-REVERT" => {
                let target = 2.2;
                lvl += 0.35 * (target - lvl) + 0.10 * r.z();
            }
            "REGIME-SWITCH" => {
                // Trends up, then flips to a downtrend at the midpoint. The hard
                // case: a momentum chaser keeps buying the top.
                let slope = if t < n / 2 { 0.05 } else { -0.06 };
                lvl += slope + 0.05 * r.z();
            }
            "VIRAL" => {
                // Calm, a sudden spike, then exponential decay back down.
                if t == n / 3 {
                    vel = 1.4;
                }
                lvl += vel + 0.05 * r.z();
                vel *= 0.7;
            }
            _ /* NOISE */ => lvl += 0.16 * r.z(),
        }
        lvl = lvl.clamp(0.0, 5.0);
        out.push((lvl.exp() - 1.0).max(0.0).round());
    }
    out
}

fn ln1p(c: f64) -> f64 {
    (1.0 + c.max(0.0)).ln()
}
fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Pearson correlation; 0 if undefined.
fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len();
    if n < 2 {
        return 0.0;
    }
    let mx = xs.iter().sum::<f64>() / n as f64;
    let my = ys.iter().sum::<f64>() / n as f64;
    let mut sxy = 0.0;
    let mut sxx = 0.0;
    let mut syy = 0.0;
    for i in 0..n {
        let dx = xs[i] - mx;
        let dy = ys[i] - my;
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    let d = (sxx * syy).sqrt();
    if d < 1e-12 {
        0.0
    } else {
        sxy / d
    }
}

// ---- the contenders: each maps a history (counts up to and including t) to a
// probability in [0,1] that attention rises over the horizon ----------------

fn algo_manifold(hist: &[f64]) -> f64 {
    crate::manifold::analyze(hist).prob_rising()
}

fn algo_momentum(hist: &[f64]) -> f64 {
    // EWMA of log-returns: recent trend persists.
    let mut ewma = 0.0;
    let alpha = 0.3;
    for w in hist.windows(2) {
        let ret = ln1p(w[1]) - ln1p(w[0]);
        ewma = alpha * ret + (1.0 - alpha) * ewma;
    }
    sigmoid(14.0 * ewma)
}

fn algo_ma_cross(hist: &[f64]) -> f64 {
    let lev: Vec<f64> = hist.iter().map(|&c| ln1p(c)).collect();
    let n = lev.len();
    let short = 5.min(n);
    let long = 20.min(n);
    let sma = lev[n - short..].iter().sum::<f64>() / short as f64;
    let lma = lev[n - long..].iter().sum::<f64>() / long as f64;
    sigmoid(10.0 * (sma - lma))
}

fn algo_popularity(hist: &[f64]) -> f64 {
    // The recommender reflex: what is already big keeps winning. Score by how far
    // current attention sits above its own recent average.
    let lev: Vec<f64> = hist.iter().map(|&c| ln1p(c)).collect();
    let n = lev.len();
    let w = 20.min(n);
    let mean = lev[n - w..].iter().sum::<f64>() / w as f64;
    sigmoid(3.0 * (lev[n - 1] - mean))
}

/// PageRank centrality on the co-movement graph of all topics at time t: topics
/// whose recent attention moves together form edges; a central topic is "important
/// to the discourse network." Returns a centrality per topic, min-max scaled to
/// [0,1], used as the rise probability (the importance-by-structure hypothesis).
fn algo_pagerank(histories: &[Vec<f64>], t: usize) -> Vec<f64> {
    let m = histories.len();
    let w = 20;
    // recent return vectors
    let rets: Vec<Vec<f64>> = histories
        .iter()
        .map(|h| {
            let s = (t + 1).saturating_sub(w);
            (s + 1..=t).map(|i| ln1p(h[i]) - ln1p(h[i - 1])).collect()
        })
        .collect();
    // adjacency = positive correlation of co-movement
    let mut adj = vec![vec![0.0_f64; m]; m];
    for i in 0..m {
        for j in 0..m {
            if i != j {
                adj[i][j] = pearson(&rets[i], &rets[j]).max(0.0);
            }
        }
    }
    // power-iteration PageRank, damping 0.85
    let d = 0.85;
    let mut pr = vec![1.0 / m as f64; m];
    for _ in 0..30 {
        let mut next = vec![(1.0 - d) / m as f64; m];
        for i in 0..m {
            let out: f64 = adj[i].iter().sum();
            if out < 1e-12 {
                for n in next.iter_mut() {
                    *n += d * pr[i] / m as f64;
                }
            } else {
                for j in 0..m {
                    next[j] += d * pr[i] * adj[i][j] / out;
                }
            }
        }
        pr = next;
    }
    let lo = pr.iter().cloned().fold(f64::INFINITY, f64::min);
    let hi = pr.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let span = (hi - lo).max(1e-12);
    pr.iter().map(|&p| (p - lo) / span).collect()
}

#[derive(Clone)]
pub struct AlgoResult {
    pub name: String,
    pub blurb: String,
    pub accuracy: f64, // directional, 0..1
    pub ic: f64,       // information coefficient, -1..1
    pub brier: f64,    // calibration, lower better
    pub by_regime: Vec<(String, f64)>, // per-regime directional accuracy
}

pub struct BenchReport {
    pub horizon: usize,
    pub topics: usize,
    pub samples: usize,
    pub regimes: Vec<String>,
    pub algos: Vec<AlgoResult>, // sorted best-first by accuracy
    pub real_eligible: usize,   // topics with enough live history to grade for real
}

/// Run the full benchmark on the synthetic suite.
pub fn run(real_eligible: usize) -> BenchReport {
    // Build the suite: PER_REGIME topics per regime, deterministic seeds.
    let mut histories: Vec<Vec<f64>> = Vec::new();
    let mut labels: Vec<String> = Vec::new();
    let mut seed = 0x9E3779B97F4A7C15u64;
    for reg in REGIMES {
        for _ in 0..PER_REGIME {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            let mut r = R(seed | 1);
            histories.push(gen(reg, &mut r));
            labels.push((*reg).to_string());
        }
    }
    let m = histories.len();

    // Single-series contenders.
    let singles: &[(&str, &str, fn(&[f64]) -> f64)] = &[
        ("MANIFOLD", "this engine: relativistic geometry of the attention path", algo_manifold),
        ("MOMENTUM", "EWMA trend-following: the heart of hot-feed ranking", algo_momentum),
        ("MA-CROSS", "short vs long moving average: classic technical analysis", algo_ma_cross),
        ("POPULARITY", "bet on what is already big: the recommender reflex", algo_popularity),
        ("RANDOM WALK", "the efficient-market null: predict no change", |_| 0.5),
    ];

    // Collect (score, realized_forward_return, regime) per algo across the
    // walk-forward. PageRank is collected alongside (it needs the whole suite).
    let n_single = singles.len();
    let mut scores: Vec<Vec<f64>> = vec![Vec::new(); n_single + 1]; // +1 for PageRank
    let mut realized: Vec<Vec<f64>> = vec![Vec::new(); n_single + 1];
    let mut regime_of: Vec<Vec<usize>> = vec![Vec::new(); n_single + 1];
    let pr_idx = n_single;

    let last_t = SERIES_LEN - HORIZON - 1;
    for t in WARMUP..=last_t {
        let pr = algo_pagerank(&histories, t);
        for (ti, h) in histories.iter().enumerate() {
            let fwd = ln1p(h[t + HORIZON]) - ln1p(h[t]);
            let reg = REGIMES.iter().position(|r| *r == labels[ti]).unwrap();
            for (ai, (_, _, f)) in singles.iter().enumerate() {
                scores[ai].push(f(&h[..=t]));
                realized[ai].push(fwd);
                regime_of[ai].push(reg);
            }
            scores[pr_idx].push(pr[ti]);
            realized[pr_idx].push(fwd);
            regime_of[pr_idx].push(reg);
        }
    }

    let metrics = |sc: &[f64], rz: &[f64], rg: &[usize]| -> AlgoResult {
        // directional accuracy (skip flat outcomes); IC; Brier.
        let mut correct_f = 0.0_f64;
        let mut total = 0;
        let mut brier = 0.0;
        let mut per: Vec<(f64, f64)> = vec![(0.0, 0.0); REGIMES.len()]; // (correct, total)
        for i in 0..sc.len() {
            let up = rz[i] > 0.0;
            brier += (sc[i] - if up { 1.0 } else { 0.0 }).powi(2);
            if rz[i].abs() > 1e-9 {
                // A score of exactly 0.5 is "no call": score it as a coin flip
                // (half credit) so the martingale null sits honestly at 0.5.
                let credit = if (sc[i] - 0.5).abs() < 1e-9 {
                    0.5
                } else if (sc[i] > 0.5) == up {
                    1.0
                } else {
                    0.0
                };
                correct_f += credit;
                per[rg[i]].0 += credit;
                per[rg[i]].1 += 1.0;
                total += 1;
            }
        }
        let by_regime = REGIMES
            .iter()
            .enumerate()
            .map(|(k, name)| {
                let (c, t) = per[k];
                ((*name).to_string(), if t > 0.0 { c / t } else { 0.0 })
            })
            .collect();
        AlgoResult {
            name: String::new(),
            blurb: String::new(),
            accuracy: if total > 0 { correct_f / total as f64 } else { 0.0 },
            ic: pearson(sc, rz),
            brier: brier / sc.len().max(1) as f64,
            by_regime,
        }
    };

    let mut algos: Vec<AlgoResult> = Vec::new();
    for (ai, (name, blurb, _)) in singles.iter().enumerate() {
        let mut m = metrics(&scores[ai], &realized[ai], &regime_of[ai]);
        m.name = (*name).to_string();
        m.blurb = (*blurb).to_string();
        algos.push(m);
    }
    let mut prm = metrics(&scores[pr_idx], &realized[pr_idx], &regime_of[pr_idx]);
    prm.name = "PAGERANK".to_string();
    prm.blurb = "centrality on the co-movement graph: importance-by-structure".to_string();
    algos.push(prm);

    algos.sort_by(|a, b| b.accuracy.partial_cmp(&a.accuracy).unwrap_or(std::cmp::Ordering::Equal));

    BenchReport {
        horizon: HORIZON,
        topics: m,
        samples: scores[0].len(),
        regimes: REGIMES.iter().map(|s| s.to_string()).collect(),
        algos,
        real_eligible,
    }
}

#[cfg(test)]
#[path = "tests_bench.rs"]
mod tests_bench;
