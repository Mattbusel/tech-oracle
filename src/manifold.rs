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

/// The shape of a trajectory's moment, the manifold's signature read. Reversal
/// states (PEAKING / BOTTOMING) are what it does best: a topic still rising but
/// whose geodesic curves back down is a top a momentum chaser cannot see coming.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Rising,    // moving up and the geodesic keeps climbing
    Peaking,   // up now, but the geodesic turns down within the horizon (a top)
    Falling,   // moving down and the geodesic keeps falling
    Bottoming, // down now, but the geodesic turns up within the horizon (a trough)
    Churning,  // no clean direction, noise dominates (spacelike)
    Flat,      // quiet, no meaningful motion
}

impl Phase {
    pub fn label(self) -> &'static str {
        match self {
            Phase::Rising => "RISING",
            Phase::Peaking => "PEAKING",
            Phase::Falling => "FALLING",
            Phase::Bottoming => "BOTTOMING",
            Phase::Churning => "CHURNING",
            Phase::Flat => "FLAT",
        }
    }
    /// A turning point the trend-followers structurally miss.
    pub fn is_turn(self) -> bool {
        matches!(self, Phase::Peaking | Phase::Bottoming)
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
    pub drift: f64,     // mean recent log-return (the causal velocity)
    pub accel: f64,     // change in drift (recent half vs older half): the curvature
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
            drift: 0.0,
            accel: 0.0,
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

    /// Integrate the geodesic forward `steps` days: cumulative projected change in
    /// log-attention, relative to today. The drift carries; the curvature bends and
    /// decays it, so a decelerating climb arcs over into a peak. This is the curve
    /// the forecast charts draw, and what `peak_in` reads to time a turn.
    pub fn forecast_path(&self, steps: usize) -> Vec<f64> {
        let mut vel = self.drift;
        let mut acc = self.accel; // full curvature, so the path can arc over a peak
        let mut level = 0.0;
        let mut out = Vec::with_capacity(steps);
        for _ in 0..steps {
            vel += acc;
            acc *= 0.6;
            level += vel;
            out.push(level);
        }
        out
    }

    /// The curvature-bent forward velocity: where the trajectory is actually headed
    /// next, drift turned by its acceleration. The reversal signal lives in its sign
    /// disagreeing with the current drift.
    pub fn forward_velocity(&self) -> f64 {
        self.drift + self.accel
    }

    /// If the trajectory is turning, how many days until the projected peak or
    /// trough. None if it is not turning. The manifold's signature capability:
    /// timing a reversal, not just spotting a trend.
    pub fn peak_in(&self) -> Option<i64> {
        if !self.defined() || !self.phase().is_turn() {
            return None;
        }
        let path = self.forecast_path(HORIZON * 2);
        // The extremum: argmax for a forming peak, argmin for a forming trough.
        let peaking = self.drift > 0.0;
        let mut best = 0usize;
        for i in 1..path.len() {
            if (peaking && path[i] > path[best]) || (!peaking && path[i] < path[best]) {
                best = i;
            }
        }
        // best is an index into a 0-based path; +1 makes it "days from now", and a
        // turn that is already underway reports as imminent (1 day).
        Some((best as i64 + 1).max(1))
    }

    /// The phase of the trajectory: its direction now, and whether its geodesic is
    /// curving back. PEAKING (rising now, forward velocity already negative) and
    /// BOTTOMING (falling now, forward velocity already positive) are the reversal
    /// calls the trend-followers structurally miss.
    pub fn phase(&self) -> Phase {
        if !self.defined() {
            return Phase::Flat;
        }
        const EPS: f64 = 0.004;
        let fwd = self.forward_velocity();
        if self.drift > EPS {
            if fwd < -EPS { Phase::Peaking } else { Phase::Rising }
        } else if self.drift < -EPS {
            if fwd > EPS { Phase::Bottoming } else { Phase::Falling }
        } else if self.regime == Regime::Spacelike {
            Phase::Churning
        } else {
            Phase::Flat
        }
    }

    /// A compact human-readable readout for the reasoning tape.
    pub fn tag(&self) -> String {
        if !self.defined() {
            return "MANIFOLD WARMING UP".to_string();
        }
        let turn = match (self.phase(), self.peak_in()) {
            (p, Some(k)) if p.is_turn() => format!(" // {} IN {}D", p.label(), k),
            (p, _) => format!(" // {}", p.label()),
        };
        format!(
            "MANIFOLD {} // gamma {:.2} // rel-mom {:+.2} // geodesic {:+.0}%{}",
            self.regime.label(),
            self.gamma,
            self.rel_return,
            self.trend * 100.0,
            turn
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

/// Follow the trajectory forward along its geodesic for HORIZON steps. Three
/// forces, all from SRFM: momentum carries velocity forward, curvature (the
/// acceleration) bends it and decays, and the metric resists travel through
/// high-velocity (curved) space (the `/ sqrt(1 + |v|)` term). The regime sets how
/// much momentum persists: a causal trend keeps going, stochastic noise reverts.
fn forecast_trend(drift: f64, accel: f64, regime: Regime) -> f64 {
    // The drift is the unbiased forward estimate, so it always carries. Only the
    // ACCELERATION (the bend in the path) is trusted by regime: a causal trend
    // believes its own curvature and rides a regime switch down; pure noise mostly
    // ignores it. This is what keeps the manifold from chasing a viral peak.
    let acc_w = match regime {
        Regime::Timelike => 1.0,
        Regime::Lightlike => 0.5,
        Regime::Spacelike => 0.2,
    };
    // The forward velocity is the drift bent once by its current curvature (scaled
    // by how much the regime trusts that curvature). A mild deceleration shrinks
    // the climb; a real reversal (curvature stronger than the drift) flips it, which
    // is how the manifold steps off a viral peak or rides a regime switch down.
    let fwd_vel = drift + accel * acc_w;
    let level = fwd_vel * HORIZON as f64;
    // Squash to [-1, 1]. GAIN maps a few weeks of typical daily drift onto a
    // meaningful slice of the range so trends read strong and noise reads flat.
    const GAIN: f64 = 6.0;
    (level * GAIN).tanh()
}

fn mean(xs: &[f64]) -> f64 {
    if xs.is_empty() {
        return 0.0;
    }
    xs.iter().sum::<f64>() / xs.len() as f64
}

/// Build a manifold reading from a topic's daily attention series, oldest first.
/// Counts may include quiet (zero) days; we work in log-attention ln(1 + count)
/// so a return is a growth rate and zeros are handled.
///
/// The relativistic framing: DRIFT (the consistent directional move) is the time
/// axis, NOISE (the random scatter) is the spatial axis. A trajectory dominated by
/// drift is TIMELIKE (causal, predictable); one dominated by noise is SPACELIKE
/// (stochastic); the boundary is LIGHTLIKE (a regime in transition).
pub fn analyze(series: &[f64]) -> Reading {
    let n = series.len();
    if n < MIN_POINTS {
        return Reading::neutral();
    }
    let lev: Vec<f64> = series.iter().map(|&c| (1.0 + c.max(0.0)).ln()).collect();
    let rets: Vec<f64> = (1..n).map(|i| lev[i] - lev[i - 1]).collect();

    let w = WINDOW.min(rets.len());
    let recent = &rets[rets.len() - w..];

    let drift = mean(recent); // the causal signal
    let noise = stdev(recent); // the spatial scatter
    let scale = drift.abs() + noise + 1e-9;

    // Beta: how directed the motion is (signal as a fraction of total motion). A
    // pure trend approaches the light speed of conviction; pure noise sits at rest.
    let beta = (drift / scale).clamp(-BETA_MAX, BETA_MAX);
    let gamma = 1.0 / (1.0 - beta * beta).sqrt();
    let rel_return = gamma * drift; // relativistic momentum

    // Normalized space-time interval: noise^2 - drift^2, in units of the scale.
    // Negative (drift wins) -> timelike; positive (noise wins) -> spacelike.
    let ds2 = (noise * noise - drift * drift) / (scale * scale);
    let regime = if ds2.abs() < LIGHTLIKE_EPS {
        Regime::Lightlike
    } else if ds2 < 0.0 {
        Regime::Timelike
    } else {
        Regime::Spacelike
    };

    // Acceleration of the trend: the recent half's drift vs the older half's. This
    // is what flips the forecast ahead of a regime switch or a viral peak, where a
    // momentum chaser keeps buying the top.
    let half = recent.len() / 2;
    let accel = mean(&recent[half..]) - mean(&recent[..half.max(1)]);

    // Curvature for the readout: how far today's move deviates from the trend, in
    // sigmas (a surprise / bend in the path).
    let last_ret = *recent.last().unwrap();
    let curvature = (last_ret - drift) / (noise + 1e-9);

    let trend = forecast_trend(drift, accel, regime);

    Reading { points: n, beta, gamma, rel_return, ds2, regime, curvature, trend, drift, accel }
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
