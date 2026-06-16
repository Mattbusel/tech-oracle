//! Generation: pure rules + date-seeded rotating templates over signals.
//!
//! This whole module sits behind one function so a later "rules -> model"
//! upgrade is a swap, not a rewrite. Output is deterministic for a given
//! (date, signals): the template variant is chosen by the date seed.

use crate::model::{Prediction, Signal};
use chrono::{Duration, NaiveDate};

/// Days a call stays open before it misses if its subject never resurfaces.
const HORIZON_DAYS: i64 = 30;

const MARKETS: &[&str] = &["RESURFACE", "SURVIVAL", "MOMENTUM", "HEAD-TO-HEAD", "INDEX"];

pub fn generate(signals: &[Signal], date: &str, seed: i64, index: i64) -> Vec<Prediction> {
    let n = signals.len();
    signals
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let subject = subject_of(s);
            let keyword = pick_keyword(&subject);
            let resolves_by = horizon(date);

            // Rotate the market so the slate has variety in what it bets on.
            let mut market = MARKETS[(seed as usize).wrapping_add(i) % MARKETS.len()];
            let mut keyword2 = String::new();
            let mut target = 0i64;
            if market == "HEAD-TO-HEAD" {
                if n >= 2 {
                    keyword2 = pick_keyword(&subject_of(&signals[(i + 1) % n]));
                    if keyword2 == keyword || keyword2.is_empty() {
                        market = "RESURFACE";
                    }
                } else {
                    market = "RESURFACE";
                }
            }
            if market == "INDEX" {
                target = (index + 5 + ((seed as i64 + i as i64) % 6)).clamp(1, 100);
            }

            let kw = keyword.to_uppercase();
            let win_if = match market {
                "SURVIVAL" => format!("WIN IF \"{kw}\" HAS NOT GONE QUIET BY {resolves_by}"),
                "MOMENTUM" => format!("WIN IF \"{kw}\" IS STILL MOVING ACROSS THE FEEDS BY {resolves_by}"),
                "HEAD-TO-HEAD" => format!("WIN IF \"{kw}\" RESURFACES BEFORE \"{}\" BY {resolves_by}", keyword2.to_uppercase()),
                "INDEX" => format!("WIN IF THE PULSE INDEX CROSSES {target} BY {resolves_by}"),
                _ => format!("WIN IF \"{kw}\" RESURFACES ACROSS THE FEEDS BY {resolves_by}"),
            };

            // Confidence sets the line: lead picks are favorites, later picks are
            // longer shots. A small date-seeded jitter keeps it from being rote.
            let jitter = ((seed.unsigned_abs() as usize + i) % 7) as f64 * 0.01;
            let confidence = (0.78 - i as f64 * 0.06 + jitter).clamp(0.52, 0.86);
            Prediction {
                date: date.to_string(),
                prediction_text: fill_template(&s.signal_type, &subject, seed, i),
                source_title: s.title.clone(),
                source_url: s.url.clone(),
                signal_type: s.signal_type.clone(),
                status: "OPEN".to_string(),
                keyword,
                win_if,
                resolves_by,
                resolved_on: String::new(),
                confidence,
                market: market.to_string(),
                keyword2,
                target,
            }
        })
        .collect()
}

/// The most distinctive token of a subject: the longest non-trivial word.
fn pick_keyword(subject: &str) -> String {
    const STOP: &[&str] = &[
        "the", "and", "for", "with", "this", "that", "from", "your", "new", "show", "using",
        "via", "model", "models", "data", "apps", "code", "tool", "tools", "open", "source",
        "into", "what", "why", "how", "are", "will",
    ];
    let mut best = "";
    let mut best_len = 0;
    for w in subject.split(|c: char| !c.is_alphanumeric()) {
        let lw = w.to_lowercase();
        if lw.len() < 4 || STOP.contains(&lw.as_str()) {
            continue;
        }
        if lw.len() > best_len {
            best_len = lw.len();
            best = w;
        }
    }
    if best.is_empty() {
        for w in subject.split(|c: char| !c.is_alphanumeric()) {
            if w.len() > 2 {
                return w.to_lowercase();
            }
        }
        return subject.to_lowercase();
    }
    best.to_lowercase()
}

fn horizon(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => (d + Duration::days(HORIZON_DAYS)).format("%Y-%m-%d").to_string(),
        Err(_) => date.to_string(),
    }
}

/// Pick a variant for the signal type, rotated by the date seed (+ item index so
/// two same-type picks on one day don't collide), and fill in the subject.
fn fill_template(signal_type: &str, subject: &str, seed: i64, idx: usize) -> String {
    let variants = variants_for(signal_type);
    let choice = ((seed as usize).wrapping_add(idx)) % variants.len();
    variants[choice].replace("{s}", subject)
}

fn variants_for(signal_type: &str) -> &'static [&'static str] {
    match signal_type {
        "hn" => &[
            "The HN front page is piling onto {s}. Expect the tooling around it to consolidate into one default winner within two quarters.",
            "{s} is the conversation on HN today. The signal says a major incumbent ships a near-identical feature before Q4.",
            "Watching {s} climb HN. This is the kind of thing that quietly becomes table-stakes infrastructure within six months.",
            "{s} is pulling outsized attention on HN. Betting the backlash think-pieces land within a month and the hype cools by next quarter.",
        ],
        "arxiv" => &[
            "Fresh cs.AI/cs.LG work on {s} just hit arXiv. Calling it now: this goes from paper to shipped product feature inside nine months.",
            "{s} is the new research thread to watch. Expect a wave of follow-on papers and an open-source reference implementation within a quarter.",
            "This {s} paper reads like a precursor. The signal points to it landing in a major lab's flagship model by year end.",
            "{s} looks academic today, but this line of work tends to go commercial fast. Call it two quarters to first product.",
        ],
        "github" => &[
            "{s} is rocketing up GitHub trending. The trajectory says it crosses into mainstream dev workflows within two quarters.",
            "Devs are starring {s} hard today. Betting it picks up a corporate backer or a managed-cloud offering within six months.",
            "{s} is having a moment on GitHub. Expect a Show HN surge and the first VC-backed competitor before Q4.",
            "Keep an eye on {s}. This kind of trending velocity usually precedes it becoming a default in its niche within the year.",
        ],
        "lobsters" => &[
            "Lobsters is chewing on {s}. When the careful crowd gets interested, it tends to be a default tool a year out.",
            "{s} is climbing Lobsters today. Betting it graduates from enthusiast favorite to production-boring within a year.",
            "The signal on {s} from Lobsters reads early-but-serious. Calling it: real adoption follows within two quarters.",
            "{s} is the quiet Lobsters pick. This is usually how the next standard library or pattern starts.",
        ],
        "devto" => &[
            "Dev.to writers are rallying around {s}. Expect tutorials to outpace the docs and adoption to snowball within a quarter.",
            "{s} is trending with practitioners on dev.to. The signal says it hits job postings as a required skill within a year.",
            "Lots of dev.to energy on {s}. Betting a framework or starter kit consolidates the space before Q4.",
            "{s} is what developers are writing about today. This kind of grassroots momentum usually precedes a hiring wave.",
        ],
        "ars" => &[
            "Ars is covering {s}. When it crosses from tech press to mainstream, regulation or a major incumbent move follows within two quarters.",
            "{s} just hit the wider tech press. The signal points to it shaping a product roadmap at a big player by year end.",
            "{s} is industry news now, not a rumor. Calling it: a concrete shipping consequence lands within ~6 months.",
            "Coverage of {s} is widening. This is the stage where it stops being optional for the incumbents.",
        ],
        _ => &["{s} is showing momentum. The signal points to it mattering more next quarter than it does today."],
    }
}

fn subject_of(s: &Signal) -> String {
    shorten(&s.title, 90)
}

/// Trim to a word boundary at most `max` chars; drop a trailing period.
fn shorten(t: &str, max: usize) -> String {
    let t = t.trim().trim_end_matches('.').trim();
    if t.chars().count() <= max {
        return t.to_string();
    }
    let mut out = String::new();
    for w in t.split_whitespace() {
        if out.len() + w.len() + 1 > max {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
    }
    if out.is_empty() {
        t.chars().take(max).collect()
    } else {
        out
    }
}
