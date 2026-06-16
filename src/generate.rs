//! Generation: pure rules + date-seeded rotating templates over signals.
//!
//! This whole module sits behind one function so a later "rules -> model"
//! upgrade is a swap, not a rewrite. Output is deterministic for a given
//! (date, signals): the template variant is chosen by the date seed.

use crate::model::{Prediction, Signal};

pub fn generate(signals: &[Signal], date: &str, seed: i64) -> Vec<Prediction> {
    signals
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let subject = subject_of(s);
            Prediction {
                date: date.to_string(),
                prediction_text: fill_template(&s.signal_type, &subject, seed, i),
                source_title: s.title.clone(),
                source_url: s.url.clone(),
                signal_type: s.signal_type.clone(),
            }
        })
        .collect()
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
            "The HN front page is piling onto {s} — expect the tooling around it to consolidate into one default winner within two quarters.",
            "{s} is the conversation on HN today; the signal says a major incumbent ships a near-identical feature before Q4.",
            "Watching {s} climb HN — this is the kind of thing that quietly becomes table-stakes infrastructure within ~6 months.",
            "{s} is pulling outsized attention on HN; betting the backlash think-pieces land within a month and the hype cools by next quarter.",
        ],
        "arxiv" => &[
            "Fresh cs.AI/cs.LG work on {s} just hit arXiv — calling it: this jumps from paper to a shipped product feature inside 9 months.",
            "{s} is the new research thread to watch; expect a wave of follow-on papers and an open-source reference implementation within a quarter.",
            "This {s} paper reads like a precursor — the signal points to it landing in a major lab's flagship model by year's end.",
            "{s} looks academic today, but this line of work tends to go commercial faster than people expect — call it two quarters to first product.",
        ],
        "github" => &[
            "{s} is rocketing up GitHub trending — the trajectory says it crosses into mainstream dev workflows within two quarters.",
            "Devs are starring {s} hard today; betting it picks up a corporate backer or a managed-cloud offering within ~6 months.",
            "{s} is having a moment on GitHub; expect a 'Show HN' surge and the first VC-backed competitor before Q4.",
            "Keep an eye on {s} — this kind of trending velocity usually precedes it becoming a default in its niche within the year.",
        ],
        _ => &["{s} is showing momentum; the signal points to it mattering more next quarter than it does today."],
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
