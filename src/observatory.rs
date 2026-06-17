//! THE OBSERVATORY: the engine's memory and quantitative core.
//!
//! Everything here is arithmetic over the daily corpus. No LLM, no API keys,
//! zero cost. It persists a growing time series of the whole tech discourse
//! (data/corpus.json), then derives the signals the desk bets on:
//!
//!   * velocity / acceleration of a term across the feeds
//!   * cross-source confirmation (how many feeds carry it today)
//!   * diffusion down the funnel: technical origin -> general public (THE CHASM)
//!   * sector indices (AI, CHIPS, RUST, ...) as a daily ticker
//!   * a lexicon FEAR / GREED gauge for the whole field
//!
//! The corpus is the moat: every day it runs, it knows more than it did.

use crate::model::Signal;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};

const CORPUS_PATH: &str = "data/corpus.json";
const KEEP_DAYS: usize = 180;
const PRUNE_AFTER_DAYS: i64 = 120;

/// The funnel, technical (low) to general public (high). A term that starts
/// low and reaches high has "crossed the chasm" -- left the dev bubble.
pub fn stage_of(src: &str) -> i64 {
    match src {
        "arxiv" => 0,
        "github" => 1,
        "crates" => 2,
        "lobsters" => 3,
        "hn" => 4,
        "devto" => 5,
        "reddit" => 6,
        "ars" => 7,
        "news" => 8,
        "wiki" => 9,
        _ => 4,
    }
}
fn is_technical(stage: i64) -> bool {
    stage <= 4
}
fn is_general(stage: i64) -> bool {
    stage >= 6
}
fn stage_label(stage: i64) -> &'static str {
    match stage {
        0 => "ARXIV",
        1 => "GITHUB",
        2 => "CRATES",
        3 => "LOBSTERS",
        4 => "HN",
        5 => "DEV.TO",
        6 => "REDDIT",
        7 => "ARS",
        8 => "THE NEWS",
        9 => "WIKIPEDIA",
        _ => "THE WIRE",
    }
}

const STOP: &[&str] = &[
    "the", "and", "for", "with", "this", "that", "from", "your", "you", "what", "why", "how",
    "new", "show", "using", "via", "are", "was", "will", "can", "has", "have", "not", "but",
    "its", "out", "get", "all", "one", "like", "just", "now", "day", "into", "ask", "tell",
    "more", "less", "than", "who", "our", "their", "they", "them", "about", "over", "when",
    "first", "best", "good", "make", "made", "use", "used", "open", "source", "free", "code",
    "app", "apps", "data", "tool", "tools", "model", "models", "build", "built", "how", "top",
    // common discourse noise that is not a real subject
    "time", "week", "year", "world", "today", "next", "based", "dev", "way", "says", "said",
    "want", "need", "back", "down", "long", "much", "many", "still", "really", "after", "before",
    "year", "days", "year", "part", "things", "thing", "people", "around", "every", "ever",
];

fn tokens(title: &str) -> Vec<String> {
    let stop: HashSet<&str> = STOP.iter().copied().collect();
    title
        .to_lowercase()
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3 && !stop.contains(w) && !w.chars().all(|c| c.is_numeric()))
        .map(|w| w.to_string())
        .collect()
}

// Sectors: a token belongs to a sector if it equals one of the sector's terms.
const SECTORS: &[(&str, &[&str])] = &[
    ("AI", &["ai", "llm", "llms", "gpt", "neural", "inference", "agent", "agents", "openai", "anthropic", "gemini", "llama", "diffusion", "transformer", "rag", "mistral", "deepseek"]),
    ("CHIPS", &["chip", "chips", "gpu", "gpus", "nvidia", "silicon", "tsmc", "arm", "semiconductor", "cuda", "wafer", "amd", "intel"]),
    ("RUST", &["rust", "cargo", "crate", "crates", "tokio", "wasm", "zig", "borrow"]),
    ("CRYPTO", &["crypto", "bitcoin", "ethereum", "blockchain", "token", "defi", "stablecoin", "btc"]),
    ("CLOUD", &["cloud", "kubernetes", "aws", "docker", "serverless", "devops", "postgres", "database", "redis", "kafka"]),
    ("SECURITY", &["security", "breach", "hack", "hacked", "exploit", "vulnerability", "ransomware", "malware", "phishing", "leak"]),
    ("ROBOTS", &["robot", "robots", "drone", "autonomous", "tesla", "waymo", "humanoid", "robotics"]),
    ("PLATFORM", &["meta", "google", "apple", "microsoft", "amazon", "tiktok", "reddit", "twitter", "platform"]),
];

const GREED_WORDS: &[&str] = &[
    "launch", "launches", "breakthrough", "fastest", "record", "soars", "boom", "raises",
    "funding", "milestone", "wins", "surges", "unveils", "revolutionary", "first", "doubles",
    "fastest", "growth", "hit", "powerful", "best", "free", "open", "ships", "beats",
];
const FEAR_WORDS: &[&str] = &[
    "layoffs", "breach", "hack", "lawsuit", "ban", "banned", "outage", "bug", "vulnerability",
    "shutdown", "collapse", "fraud", "decline", "warns", "fails", "dead", "down", "crisis",
    "scam", "leak", "delay", "delayed", "sued", "fine", "probe",
];

// --------------------------------------------------------------------------
// Persisted corpus
// --------------------------------------------------------------------------

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct CorpusDay {
    pub date: String,
    pub total: usize,
    #[serde(default)]
    pub by_source: BTreeMap<String, usize>,
    #[serde(default)]
    pub terms: BTreeMap<String, usize>,
    #[serde(default)]
    pub sectors: BTreeMap<String, usize>,
    #[serde(default)]
    pub greed: i64,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct TermRec {
    pub first_seen: String,
    pub first_stage: i64,
    pub stages: BTreeMap<String, String>, // source -> first date seen there
    pub peak: usize,
    pub peak_date: String,
    pub days: i64,
    pub last_seen: String,
    pub crossed: bool,
    pub crossed_on: String,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Corpus {
    #[serde(default)]
    pub days: Vec<CorpusDay>,
    #[serde(default)]
    pub terms: BTreeMap<String, TermRec>,
}

// --------------------------------------------------------------------------
// The live observatory for today
// --------------------------------------------------------------------------

pub struct Observatory {
    pub today: String,
    pub corpus: Corpus,
    pub today_terms: HashMap<String, usize>,
    pub today_sources: HashMap<String, HashSet<String>>,
    pub greed: i64,
    pub sectors: Vec<(String, i64, i64)>, // name, index 0..100, delta vs yesterday
}

/// Build today's observatory from the day's signals: update the persisted
/// corpus (term ledger, diffusion stages, daily snapshot), then expose the
/// derived views. Persists data/corpus.json. Fail-soft on a missing file.
pub fn build(signals: &[Signal], date: &str) -> Observatory {
    let mut corpus: Corpus = std::fs::read_to_string(CORPUS_PATH)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();

    // Count today's terms and which sources carry each.
    let mut today_terms: HashMap<String, usize> = HashMap::new();
    let mut today_sources: HashMap<String, HashSet<String>> = HashMap::new();
    let mut by_source: BTreeMap<String, usize> = BTreeMap::new();
    let mut sector_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut greed_hits = 0i64;
    let mut fear_hits = 0i64;

    let sector_lut: HashMap<&str, &str> = SECTORS
        .iter()
        .flat_map(|(name, words)| words.iter().map(move |w| (*w, *name)))
        .collect();
    let greed: HashSet<&str> = GREED_WORDS.iter().copied().collect();
    let fear: HashSet<&str> = FEAR_WORDS.iter().copied().collect();

    for s in signals {
        *by_source.entry(s.signal_type.clone()).or_insert(0) += 1;
        let mut seen_sector: HashSet<&str> = HashSet::new();
        for tok in tokens(&s.title) {
            *today_terms.entry(tok.clone()).or_insert(0) += 1;
            today_sources.entry(tok.clone()).or_default().insert(s.signal_type.clone());
            if let Some(sec) = sector_lut.get(tok.as_str()) {
                if seen_sector.insert(*sec) {
                    *sector_counts.entry((*sec).to_string()).or_insert(0) += 1;
                }
            }
            if greed.contains(tok.as_str()) {
                greed_hits += 1;
            }
            if fear.contains(tok.as_str()) {
                fear_hits += 1;
            }
        }
    }

    let greed_idx = if greed_hits + fear_hits > 0 {
        (greed_hits * 100 / (greed_hits + fear_hits)).clamp(0, 100)
    } else {
        50
    };

    // Update the longitudinal term ledger (diffusion + peaks).
    for (term, &count) in &today_terms {
        let srcs = today_sources.get(term).cloned().unwrap_or_default();
        let min_stage_today = srcs.iter().map(|s| stage_of(s)).min().unwrap_or(4);
        let max_stage_today = srcs.iter().map(|s| stage_of(s)).max().unwrap_or(4);
        let rec = corpus.terms.entry(term.clone()).or_insert_with(|| TermRec {
            first_seen: date.to_string(),
            first_stage: min_stage_today,
            peak: 0,
            ..Default::default()
        });
        if rec.first_seen.is_empty() {
            rec.first_seen = date.to_string();
            rec.first_stage = min_stage_today;
        }
        for s in &srcs {
            rec.stages.entry(s.clone()).or_insert_with(|| date.to_string());
        }
        if count > rec.peak {
            rec.peak = count;
            rec.peak_date = date.to_string();
        }
        if rec.last_seen != date {
            rec.days += 1;
        }
        rec.last_seen = date.to_string();
        // Chasm crossing: technical origin, now reaching the general public.
        if !rec.crossed && is_technical(rec.first_stage) && is_general(max_stage_today) {
            rec.crossed = true;
            rec.crossed_on = date.to_string();
        }
    }

    // Prune stale terms so the ledger stays bounded.
    if let Ok(today_d) = chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        corpus.terms.retain(|_, r| {
            chrono::NaiveDate::parse_from_str(&r.last_seen, "%Y-%m-%d")
                .map(|d| (today_d - d).num_days() <= PRUNE_AFTER_DAYS)
                .unwrap_or(true)
        });
    }

    // Yesterday's sector counts for the ticker delta.
    let prev_sectors: BTreeMap<String, usize> = corpus
        .days
        .iter()
        .filter(|d| d.date.as_str() < date)
        .max_by(|a, b| a.date.cmp(&b.date))
        .map(|d| d.sectors.clone())
        .unwrap_or_default();

    // Keep only the meatier terms in the daily snapshot to bound file size.
    let snapshot_terms: BTreeMap<String, usize> =
        today_terms.iter().filter(|(_, &c)| c >= 2).map(|(k, v)| (k.clone(), *v)).collect();

    // Push today's snapshot (idempotent for re-runs).
    corpus.days.retain(|d| d.date != date);
    corpus.days.push(CorpusDay {
        date: date.to_string(),
        total: signals.len(),
        by_source: by_source.clone(),
        terms: snapshot_terms,
        sectors: sector_counts.clone(),
        greed: greed_idx,
    });
    corpus.days.sort_by(|a, b| a.date.cmp(&b.date));
    if corpus.days.len() > KEEP_DAYS {
        let cut = corpus.days.len() - KEEP_DAYS;
        corpus.days.drain(0..cut);
    }

    // Persist.
    if let Some(parent) = std::path::Path::new(CORPUS_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(j) = serde_json::to_string(&corpus) {
        let _ = std::fs::write(CORPUS_PATH, j);
    }

    // Sector ticker: index relative to the hottest sector today.
    let max_sec = sector_counts.values().copied().max().unwrap_or(1).max(1);
    let mut sectors: Vec<(String, i64, i64)> = SECTORS
        .iter()
        .map(|(name, _)| {
            let c = sector_counts.get(*name).copied().unwrap_or(0);
            let idx = (c as f64 / max_sec as f64 * 100.0).round() as i64;
            let delta = c as i64 - prev_sectors.get(*name).copied().unwrap_or(0) as i64;
            ((*name).to_string(), idx, delta)
        })
        .collect();
    sectors.sort_by(|a, b| b.1.cmp(&a.1));

    Observatory {
        today: date.to_string(),
        corpus,
        today_terms,
        today_sources,
        greed: greed_idx,
        sectors,
    }
}

impl Observatory {
    pub fn today_count(&self, term: &str) -> usize {
        self.today_terms.get(term).copied().unwrap_or(0)
    }

    pub fn cross_source(&self, term: &str) -> usize {
        self.today_sources.get(term).map(|s| s.len()).unwrap_or(0)
    }

    // ----- grading queries over the corpus time series -----------------------
    // A corpus-day snapshot only retains terms with >= 2 mentions that day, so
    // "presence" in the history already means a real day of discourse, never a
    // single stray match. These power the earned-but-fair grader.

    /// Calendar days strictly after `after` (up to and including today) on which
    /// the term had real presence (>= 2 mentions). The core "did it come back" count.
    pub fn active_days_after(&self, term: &str, after: &str) -> i64 {
        if term.is_empty() {
            return 0;
        }
        self.corpus
            .days
            .iter()
            .filter(|d| d.date.as_str() > after)
            .filter(|d| d.terms.get(term).copied().unwrap_or(0) >= 2)
            .count() as i64
    }

    /// The strongest single-day mention count strictly after `after`.
    pub fn peak_after(&self, term: &str, after: &str) -> i64 {
        if term.is_empty() {
            return 0;
        }
        self.corpus
            .days
            .iter()
            .filter(|d| d.date.as_str() > after)
            .filter_map(|d| d.terms.get(term).copied())
            .max()
            .unwrap_or(0) as i64
    }

    /// Total mentions strictly after `after`.
    pub fn total_after(&self, term: &str, after: &str) -> i64 {
        if term.is_empty() {
            return 0;
        }
        self.corpus
            .days
            .iter()
            .filter(|d| d.date.as_str() > after)
            .filter_map(|d| d.terms.get(term).copied())
            .sum::<usize>() as i64
    }

    /// The term's daily attention trajectory, oldest first, ending with today's
    /// true count: the price series the manifold rides. Past days come from the
    /// committed corpus snapshots (>= 2 presence floor); today uses the live count.
    pub fn trajectory(&self, term: &str) -> Vec<f64> {
        let mut v: Vec<f64> = self
            .corpus
            .days
            .iter()
            .filter(|d| d.date.as_str() < self.today.as_str())
            .map(|d| d.terms.get(term).copied().unwrap_or(0) as f64)
            .collect();
        v.push(self.today_count(term) as f64);
        v
    }

    /// The most recent date the term had real presence, across all history.
    pub fn last_active(&self, term: &str) -> Option<String> {
        if term.is_empty() {
            return None;
        }
        self.corpus
            .days
            .iter()
            .rev()
            .find(|d| d.terms.get(term).copied().unwrap_or(0) >= 2)
            .map(|d| d.date.clone())
    }

    /// The first date strictly after `after` the term had real presence.
    pub fn first_active_after(&self, term: &str, after: &str) -> Option<String> {
        if term.is_empty() {
            return None;
        }
        self.corpus
            .days
            .iter()
            .filter(|d| d.date.as_str() > after)
            .find(|d| d.terms.get(term).copied().unwrap_or(0) >= 2)
            .map(|d| d.date.clone())
    }

    /// Did the term cross from a technical origin to a general-public source on
    /// some date strictly after `after`? (The diffusion ledger marks `crossed`.)
    pub fn crossed_after(&self, term: &str, after: &str) -> bool {
        self.corpus
            .terms
            .get(term)
            .map(|t| t.crossed && t.crossed_on.as_str() > after)
            .unwrap_or(false)
    }

    /// Momentum: over the window since `after`, are mentions concentrated in the
    /// back half (still climbing) rather than the front (fading)? True only with
    /// enough of a window to judge and a non-empty recent half.
    pub fn rising_since(&self, term: &str, after: &str, today: &str) -> bool {
        if term.is_empty() {
            return false;
        }
        let win: Vec<usize> = self
            .corpus
            .days
            .iter()
            .filter(|d| d.date.as_str() > after && d.date.as_str() <= today)
            .map(|d| d.terms.get(term).copied().unwrap_or(0))
            .collect();
        if win.len() < 2 {
            return false;
        }
        let mid = win.len() / 2;
        let early: usize = win[..mid].iter().sum();
        let late: usize = win[mid..].iter().sum();
        late > 0 && late >= early
    }

    /// Historical daily counts for a term, oldest-first, excluding today.
    fn history(&self, term: &str) -> Vec<usize> {
        self.corpus
            .days
            .iter()
            .filter(|d| d.date.as_str() < self.today.as_str())
            .map(|d| d.terms.get(term).copied().unwrap_or(0))
            .collect()
    }

    /// Velocity: today's count vs the trailing 7-day average, as a percent.
    /// Returns a large positive number for a fresh spike off a zero baseline.
    pub fn velocity_pct(&self, term: &str) -> i64 {
        let today = self.today_count(term) as f64;
        let hist = self.history(term);
        let tail: Vec<usize> = hist.iter().rev().take(7).copied().collect();
        if tail.is_empty() {
            return if today > 0.0 { 100 } else { 0 };
        }
        let avg = tail.iter().sum::<usize>() as f64 / tail.len() as f64;
        if avg < 0.5 {
            return if today > 0.0 { 400 } else { 0 };
        }
        (((today - avg) / avg) * 100.0).round() as i64
    }

    /// Acceleration: change in velocity, today vs yesterday (sign of curvature).
    pub fn accel_pct(&self, term: &str) -> i64 {
        let hist = self.history(term);
        let n = hist.len();
        if n < 2 {
            return 0;
        }
        let today = self.today_count(term) as f64;
        let y1 = hist[n - 1] as f64;
        let y2 = hist[n - 2] as f64;
        let v_now = today - y1;
        let v_prev = y1 - y2;
        ((v_now - v_prev) * 100.0 / (y2.max(1.0))).round() as i64
    }

    /// The funnel reach today: the most-general source carrying the term.
    pub fn reach_stage(&self, term: &str) -> i64 {
        self.today_sources
            .get(term)
            .and_then(|s| s.iter().map(|x| stage_of(x)).max())
            .unwrap_or(4)
    }

    pub fn origin_stage(&self, term: &str) -> i64 {
        self.corpus.terms.get(term).map(|r| r.first_stage).unwrap_or_else(|| self.reach_stage(term))
    }

    /// A term is "crossing" if it was born technical and is touching the general
    /// public right now without having been marked crossed before.
    pub fn is_crossing(&self, term: &str) -> bool {
        is_technical(self.origin_stage(term)) && is_general(self.reach_stage(term))
    }

    pub fn origin_label(&self, term: &str) -> &'static str {
        stage_label(self.origin_stage(term))
    }
    pub fn reach_label(&self, term: &str) -> &'static str {
        stage_label(self.reach_stage(term))
    }

    /// Estimated half-life in days from the term's peak, if it is decaying.
    pub fn half_life(&self, term: &str) -> Option<f64> {
        let rec = self.corpus.terms.get(term)?;
        if rec.peak < 2 {
            return None;
        }
        let cur = self.today_count(term).max(
            self.history(term).last().copied().unwrap_or(0),
        ) as f64;
        if cur < 1.0 || cur >= rec.peak as f64 {
            return None;
        }
        let peak_d = chrono::NaiveDate::parse_from_str(&rec.peak_date, "%Y-%m-%d").ok()?;
        let today_d = chrono::NaiveDate::parse_from_str(&self.today, "%Y-%m-%d").ok()?;
        let elapsed = (today_d - peak_d).num_days().max(1) as f64;
        let ratio = (rec.peak as f64 / cur).ln();
        if ratio <= 0.0 {
            return None;
        }
        Some((elapsed * std::f64::consts::LN_2 / ratio * 10.0).round() / 10.0)
    }

    /// The compact reasoning tape printed under each call.
    pub fn rationale(&self, term: &str) -> String {
        let vel = self.velocity_pct(term);
        let xs = self.cross_source(term);
        let reach = self.reach_label(term);
        let mut bits = vec![
            format!("VEL {:+}%/7d", vel),
            format!("{}/{} FEEDS", xs, 10),
            format!("REACH {reach}"),
        ];
        let acc = self.accel_pct(term);
        if acc != 0 {
            bits.push(format!("ACCEL {:+}%", acc));
        }
        if self.is_crossing(term) {
            bits.push(format!("CROSSING FROM {}", self.origin_label(term)));
        }
        if let Some(hl) = self.half_life(term) {
            bits.push(format!("HALF-LIFE {hl}d"));
        }
        bits.join(" // ")
    }

    /// Terms moving fastest today (with a real baseline), for the movers board.
    pub fn top_movers(&self, n: usize) -> Vec<(String, i64, usize)> {
        let mut v: Vec<(String, i64, usize)> = self
            .today_terms
            .iter()
            .filter(|(_, &c)| c >= 2)
            .map(|(t, &c)| (t.clone(), self.velocity_pct(t), c))
            .collect();
        v.sort_by(|a, b| b.1.cmp(&a.1).then(b.2.cmp(&a.2)));
        v.truncate(n);
        v
    }

    /// Terms crossing the chasm now, or crossed within the last 14 days.
    pub fn chasm_watch(&self, n: usize) -> Vec<(String, String, String, i64)> {
        let today_d = chrono::NaiveDate::parse_from_str(&self.today, "%Y-%m-%d").ok();
        let mut out: Vec<(String, String, String, i64, i64)> = Vec::new();
        for (term, rec) in &self.corpus.terms {
            let crossing_now = self.is_crossing(term) && self.today_count(term) > 0;
            let recently = rec.crossed
                && today_d
                    .and_then(|t| {
                        chrono::NaiveDate::parse_from_str(&rec.crossed_on, "%Y-%m-%d")
                            .ok()
                            .map(|c| (t - c).num_days())
                    })
                    .map(|d| (0..=14).contains(&d))
                    .unwrap_or(false);
            if !crossing_now && !recently {
                continue;
            }
            let days = today_d
                .and_then(|t| {
                    chrono::NaiveDate::parse_from_str(&rec.first_seen, "%Y-%m-%d")
                        .ok()
                        .map(|f| (t - f).num_days())
                })
                .unwrap_or(0);
            out.push((
                term.clone(),
                stage_label(rec.first_stage).to_string(),
                self.reach_label(term).to_string(),
                days,
                self.velocity_pct(term),
            ));
        }
        out.sort_by(|a, b| b.4.cmp(&a.4));
        out.truncate(n);
        out.into_iter().map(|(t, o, r, d, _)| (t, o, r, d)).collect()
    }

    pub fn greed_label(&self) -> &'static str {
        match self.greed {
            0..=20 => "EXTREME FEAR",
            21..=40 => "FEAR",
            41..=59 => "NEUTRAL",
            60..=79 => "GREED",
            _ => "EXTREME GREED",
        }
    }
}

#[cfg(test)]
#[path = "tests_observatory.rs"]
mod tests_observatory;
