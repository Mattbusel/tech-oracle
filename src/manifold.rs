//! THE MANIFOLD: the oracle's prediction core, repurposed from SRFM (Special
//! Relativity in Financial Modeling, the author's quant lab on manifold price
//! prediction). The insight that carries over: a time series is not a flat line,
//! it is a trajectory through a curved space-time, and where it goes next is a
//! geodesic on that manifold.
//!
//! Here the "asset" is a topic and its "price" is attention: the daily mention
//! count in the corpus. For a topic's attention trajectory we compute
//!   - beta:   the relativistic velocity of attention change (move / local light speed),
//!   - gamma:  the Lorentz factor, so fast moves carry disproportionate weight,
//!   - ds2:    the space-time interval, whose sign classifies the local regime,
//!   - regime: TIMELIKE (causal, a real trend), SPACELIKE (stochastic noise), or
//!             LIGHTLIKE (a critical transition, a regime about to flip),
//!   - curvature: the geodesic-deviation signal (is the trend bending / accelerating),
//!   - trend:  a forward geodesic projection over the horizon, in [-1, 1].
//!
//! The engine is then SHAPED by topology (the regime selects which market to bet)
//! and PREDICTED through the manifold (the geodesic forecast drives confidence and
//! the live mark-to-market likelihood). Dependency-free and deterministic.

/// Velocities are normalized to a local "speed of light" (the rolling max move),
/// so c = 1 in those units. A move can approach but never reach it.
pub const BETA_MAX: f64 = 0.9999;
/// Half-width of the lightlike band on the normalized interval: |ds2| under this
/// is a critical transition rather than a clean trend or clean noise.
pub const LIGHTLIKE_EPS: f64 = 0.15;
/// How many steps forward the geodesic is integrated for the forecast.
pub const HORIZON: usize = 7;
/// Lookback for the velocity normalization and the curvature z-score.
pub const WINDOW: usize = 20;
/// Below this many trajectory points the manifold is undefined; fall back neutral.
pub const MIN_POINTS: usize = 3;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Regime {
    Timelike,  // ds2 < 0: causal, a real directional trend
    Lightlike, // |ds2| small: a critical transition, the trend is flipping
    Spacelike, // ds2 > 0: stochastic, dominated by noise / one-off spikes
}

impl Regime {
    pub fn label(self) -> &'static str {
        match self {
            Regime::Timelike => "TIMELIKE",
            Regime::Lightlike => "LIGHTLIKE",
            Regime::Spacelike => "SPACELIKE",
        }
    }
    /// How much to trust a directional forecast in this regime. A causal trend is
    /// trustworthy; pure noise is not; a transition is in between.
    pub fn certainty(self) -> f64 {
        match self {
            Regime::Timelike => 1.0,
            Regime::Lightlike => 0.6,
            Regime::Spacelike => 0.4,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Reading {
    pub points: usize,
    pub beta: f64,
    pub gamma: f64,
    pub rel_return: f64, // gamma * last log-return: relativistic momentum
    pub ds2: f64,
    pub regime: Regime,
    pub curvature: f64, // geodesic-deviation z-score (signed)
    pub trend: f64,     // forward geodesic projection, tanh-squashed to [-1, 1]
}

impl Reading {
    pub fn neutral() -> Self {
        Reading {
            points: 0,
            beta: 0.0,
            gamma: 1.0,
            rel_return: 0.0,
            ds2: -1.0,
            regime: Regime::Lightlike,
            curvature: 0.0,
            trend: 0.0,
        }
    }

    /// Did the manifold actually have enough trajectory to say something?
    pub fn defined(&self) -> bool {
        self.points >= MIN_POINTS
    }

    /// The manifold's probability (0..1) that attention rises or sustains over the
    /// horizon: the forward trend scaled by how trustworthy the regime is, pulled
    /// toward a coin flip when flat or noisy.
    pub fn prob_rising(&self) -> f64 {
        (0.5 + 0.5 * self.trend * self.regime.certainty()).clamp(0.02, 0.98)
    }

    /// Prediction confidence (0..1): conviction from speed (gamma), the regime's
    /// trustworthiness, and how strongly the geodesic points anywhere at all.
    pub fn confidence(&self) -> f64 {
        if !self.defined() {
            return 0.5;
        }
        let speed = (1.0 - 1.0 / self.gamma).clamp(0.0, 1.0); // 0 calm .. ->1 fast
        (0.5 + 0.18 * self.trend.abs() * self.regime.certainty() + 0.12 * speed).clamp(0.34, 0.92)
    }

    /// A compact human-readable readout for the reasoning tape.
    pub fn tag(&self) -> String {
        if !self.defined() {
            return "MANIFOLD WARMING UP".to_string();
        }
        format!(
            "MANIFOLD {} // gamma {:.2} // rel-mom {:+.2} // curv {:+.1} // geodesic {:+.0}%",
            self.regime.label(),
            self.gamma,
            self.rel_return,
            self.curvature,
            self.trend * 100.0
        )
    }
}

/// Population standard deviation.
fn stdev(xs: &[f64]) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let var = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / xs.len() as f64;
    var.sqrt()
}

/// Z-score of the last element against the whole slice.
fn zscore_last(xs: &[f64]) -> f64 {
    if xs.len() < 2 {
        return 0.0;
    }
    let mean = xs.iter().sum::<f64>() / xs.len() as f64;
    let sd = stdev(xs).max(1e-9);
    (xs[xs.len() - 1] - mean) / sd
}

/// Follow the trajectory forward along its geodesic for HORIZON steps. Three
/// forces, all from SRFM: momentum carries velocity forward, curvature (the
/// acceleration) bends it and decays, and the metric resists travel through
/// high-velocity (curved) space (the `/ sqrt(1 + |v|)` term). The regime sets how
/// much momentum persists: a causal trend keeps going, stochastic noise reverts.
fn forecast_trend(v0: f64, a0: f64, regime: Regime) -> f64 {
    let persist = match regime {
        Regime::Timelike => 0.85,
        Regime::Lightlike => 0.55,
        Regime::Spacelike => 0.25,
    };
    let mut vel = v0;
    let mut acc = a0;
    let mut level = 0.0;
    for _ in 0..HORIZON {
        acc *= 0.6; // curvature decays
        vel = vel * persist + acc;
        level += vel / (1.0 + vel.abs()).sqrt(); // metric resistance
    }
    level.tanh()
}

/// Build a manifold reading from a topic's daily attention series, oldest first.
/// Counts may include quiet (zero) days; we work in log-attention ln(1 + count)
/// so a return is a growth rate and zeros are handled.
pub fn analyze(series: &[f64]) -> Reading {
    let n = series.len();
    if n < MIN_POINTS {
        return Reading::neutral();
    }
    let lev: Vec<f64> = series.iter().map(|&c| (1.0 + c.max(0.0)).ln()).collect();
    let rets: Vec<f64> = (1..n).map(|i| lev[i] - lev[i - 1]).collect();

    // Recent window. The local "speed of light" is the largest move in it, so a
    // calm topic and a violent one are judged on their own scales.
    let w = WINDOW.min(rets.len());
    let recent = &rets[rets.len() - w..];
    let maxv = recent.iter().fold(1e-6_f64, |m, &r| m.max(r.abs()));

    let betas: Vec<f64> = recent.iter().map(|&r| (r / maxv).clamp(-BETA_MAX, BETA_MAX)).collect();
    let gammas: Vec<f64> = betas.iter().map(|&b| 1.0 / (1.0 - b * b).sqrt()).collect();

    let beta = *betas.last().unwrap();
    let gamma = *gammas.last().unwrap();
    let last_ret = *recent.last().unwrap();
    let rel_return = gamma * last_ret;

    // Space-time interval in normalized (c = 1) units: ds2 = -1 + |v|^2 + |a|^2 + sigma^2.
    let accel = if recent.len() >= 2 { last_ret - recent[recent.len() - 2] } else { 0.0 };
    let accel_n = accel / maxv;
    let vol_n = stdev(recent) / maxv;
    let ds2 = -1.0 + beta * beta + accel_n * accel_n + vol_n * vol_n;
    let regime = if ds2.abs() < LIGHTLIKE_EPS {
        Regime::Lightlike
    } else if ds2 < 0.0 {
        Regime::Timelike
    } else {
        Regime::Spacelike
    };

    // Geodesic-deviation signal: (delta gamma / gamma) * sign(beta), z-scored.
    let mut geo = vec![0.0_f64; gammas.len()];
    for i in 1..gammas.len() {
        let dg = gammas[i] - gammas[i - 1];
        geo[i] = (dg / gammas[i].max(1e-9)) * betas[i].signum();
    }
    let curvature = zscore_last(&geo);

    let trend = forecast_trend(last_ret, accel, regime);

    Reading { points: n, beta, gamma, rel_return, ds2, regime, curvature, trend }
}

/// The regime's preferred markets. Topology shapes the bet: a causal trend is bet
/// as a trend (it keeps moving / survives), a transition is bet as a crossing or a
/// rivalry (the field is reorganizing), and noise is bet as a spike or a longshot.
pub fn market_pool(regime: Regime) -> &'static [&'static str] {
    match regime {
        Regime::Timelike => &["MOMENTUM", "SURVIVAL", "RESURFACE", "FUTURES"],
        Regime::Lightlike => &["CHASM", "HEAD-TO-HEAD", "CROSSOVER", "RESURFACE"],
        Regime::Spacelike => &["OVER", "LONGSHOT", "RESURFACE", "INDEX"],
    }
}

#[cfg(test)]
#[path = "tests_manifold.rs"]
mod tests_manifold;
