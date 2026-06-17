//! Generation: pure rules + date-seeded rotating templates over signals.
//!
//! This whole module sits behind one function so a later "rules -> model"
//! upgrade is a swap, not a rewrite. Output is deterministic for a given
//! (date, signals): the template variant is chosen by the date seed.

use crate::model::{Prediction, Signal};
use crate::observatory::Observatory;
use chrono::{Duration, NaiveDate};

/// Days a call stays open before it misses if its subject never resurfaces.
const HORIZON_DAYS: i64 = 30;
const FUTURES_DAYS: i64 = 90;

const MARKETS: &[&str] = &[
    "RESURFACE", "CHASM", "MOMENTUM", "OVER", "HEAD-TO-HEAD", "SURVIVAL", "CROSSOVER", "INDEX",
    "FUTURES", "LONGSHOT",
];

pub fn generate(
    signals: &[Signal],
    date: &str,
    seed: i64,
    index: i64,
    obs: &Observatory,
) -> Vec<Prediction> {
    let n = signals.len();
    signals
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let subject = subject_of(s);
            let keyword = pick_keyword(&subject);
            let kw = keyword.to_uppercase();

            // Features from the observatory: this is the engine showing its work.
            let vel = obs.velocity_pct(&keyword);
            let xsrc = obs.cross_source(&keyword);
            let crossing = obs.is_crossing(&keyword);
            let rationale = obs.rationale(&keyword);

            // Rotate the market, then validate it against what the data supports.
            let mut market = MARKETS[(seed as usize).wrapping_add(i) % MARKETS.len()];
            let mut keyword2 = String::new();
            let mut target = 0i64;
            let mut horizon_days = HORIZON_DAYS;

            match market {
                "CHASM" => {
                    // Only a bet if the term was born technical (room to cross).
                    if !(crossing || obs.origin_stage(&keyword) <= 4) {
                        market = "RESURFACE";
                    }
                }
                "HEAD-TO-HEAD" | "CROSSOVER" => {
                    if n >= 2 {
                        keyword2 = pick_keyword(&subject_of(&signals[(i + 1) % n]));
                        if keyword2 == keyword || keyword2.is_empty() {
                            market = "RESURFACE";
                        }
                    } else {
                        market = "RESURFACE";
                    }
                }
                "OVER" => {
                    let today = obs.today_count(&keyword).max(1) as i64;
                    target = today + 1 + ((seed + i as i64).rem_euclid(3));
                }
                "INDEX" => {
                    target = (index + 5 + ((seed + i as i64).rem_euclid(6))).clamp(1, 100);
                }
                "FUTURES" => horizon_days = FUTURES_DAYS,
                _ => {}
            }

            let resolves_by = horizon(date, horizon_days);
            let win_if = match market {
                "SURVIVAL" => format!("WIN IF \"{kw}\" HAS NOT GONE QUIET BY {resolves_by}"),
                "MOMENTUM" => format!("WIN IF \"{kw}\" IS STILL MOVING ACROSS THE FEEDS BY {resolves_by}"),
                "HEAD-TO-HEAD" => format!("WIN IF \"{kw}\" RESURFACES BEFORE \"{}\" BY {resolves_by}", keyword2.to_uppercase()),
                "CROSSOVER" => format!("WIN IF \"{kw}\" OUT-MENTIONS \"{}\" BY {resolves_by}", keyword2.to_uppercase()),
                "INDEX" => format!("WIN IF THE PULSE INDEX CROSSES {target} BY {resolves_by}"),
                "OVER" => format!("WIN IF \"{kw}\" CLEARS {target} MENTIONS IN A DAY BY {resolves_by}"),
                "CHASM" => format!("WIN IF \"{kw}\" REACHES THE GENERAL PUBLIC (REDDIT/NEWS/WIKIPEDIA) BY {resolves_by}"),
                "FUTURES" => format!("WIN IF \"{kw}\" STILL MATTERS BY {resolves_by} (90-DAY FUTURE)"),
                "LONGSHOT" => format!("LONGSHOT: WIN IF \"{kw}\" RESURFACES BY {resolves_by}"),
                _ => format!("WIN IF \"{kw}\" RESURFACES ACROSS THE FEEDS BY {resolves_by}"),
            };

            // Confidence comes from the data: cross-source confirmation and
            // positive velocity raise the line; later picks, longshots and the
            // genuinely-hard chasm bet lower it.
            let mut confidence = 0.55
                + (xsrc as f64 * 0.035).min(0.18)
                + if vel > 0 { 0.06 } else { -0.04 }
                - i as f64 * 0.03;
            if market == "LONGSHOT" {
                confidence = 0.40;
            }
            if market == "CHASM" {
                confidence -= 0.06;
            }
            let jitter = ((seed.unsigned_abs() as usize + i) % 5) as f64 * 0.01;
            let confidence = (confidence + jitter).clamp(0.34, 0.9);

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
                rationale,
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

fn horizon(date: &str, days: i64) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => (d + Duration::days(days)).format("%Y-%m-%d").to_string(),
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
        "reddit" => &[
            "Reddit cannot stop arguing about {s}. When it breaks containment like this, it goes properly mainstream within a quarter.",
            "{s} is blowing up on Reddit today. Betting it jumps from the feed to the news cycle within a month.",
            "The crowd has found {s}. This is usually the moment before it becomes everyone's problem or everyone's product.",
            "{s} is topping Reddit. Calling it: a brand or regulator reacts in public within two quarters.",
        ],
        "news" => &[
            "{s} is in the headlines. The signal says a bigger move (a deal, a rule, or a backlash) follows within two quarters.",
            "The press is circling {s}. Betting today's story is the small version of a much larger one by year end.",
            "{s} made the news. This is the kind of story that quietly reshapes a market before anyone admits it.",
            "{s} is the headline now. Calling it: the second-order consequence is the one that actually matters, and it lands within ~6 months.",
        ],
        "wiki" => &[
            "{s} is pulling general-public attention on Wikipedia now. The signal says it stays in the mainstream conversation through next quarter.",
            "Regular people are looking up {s}. This is what crossing the chasm looks like; betting a brand or regulator reacts within two quarters.",
            "{s} broke into Wikipedia's most-read. Calling it: the mainstream story gets bigger before it gets smaller.",
            "The public found {s}. This is usually the top of the hype, not the start; betting the cool-down lands within a quarter.",
        ],
        "crates" => &[
            "{s} is climbing real download charts. Adoption, not chatter; betting it becomes a default dependency in its niche within a year.",
            "Devs are actually pulling {s} into builds. The signal says a managed offering or corporate backer shows up within six months.",
            "{s} is gaining real adoption. This is the boring-and-everywhere trajectory; calling it a standard tool within the year.",
            "Downloads of {s} are accelerating. Expect the ecosystem to consolidate around it before Q4.",
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
