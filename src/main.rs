mod access;
mod card;
mod fetch;
mod generate;
mod model;
mod rank;
mod render;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use model::Prediction;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// Committed, public: the revealed track record. Renders the public page.
const DATA_PATH: &str = "data/predictions.json";
// Committed, public: the daily acceleration index history (THE PULSE).
const PULSE_PATH: &str = "data/pulse.json";
// Gitignored, synced from KV before the run: calls still under embargo.
const EMBARGO_IN: &str = "build/embargoed_in.json";
// Gitignored, synced to KV after the run: the subscriber edge payload.
const EARLY_OUT: &str = "build/early_payload.json";

pub const OUT_DIR: &str = "docs";
pub const OUT_HTML: &str = "docs/index.html";

fn main() {
    // The release window. Subscribers see a call immediately; the public page
    // reveals it `delay_days` later. 0 = no embargo (acts like a free blog).
    let delay_days: i64 = std::env::var("REVEAL_DELAY_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(1);

    let now = Utc::now().date_naive();
    let date = now.format("%Y-%m-%d").to_string();
    let human = now.format("%B %-d, %Y").to_string();
    // Deterministic seed for template rotation: same day -> same variant choice.
    let seed = now.num_days_from_ce() as i64;

    // CTA URLs are public by nature (a Payment Link is shareable); they come from
    // Actions vars, defaulting to "#" so local runs render without Stripe.
    let payment_link = env_or("STRIPE_PAYMENT_LINK", "#");
    let portal_url = env_or("STRIPE_PORTAL_URL", "#");
    let early_access_url = env_or("EARLY_ACCESS_URL", "#");

    // ---- fetch all six sources concurrently; each fails soft ----
    let client = fetch::client();
    let signals = std::thread::scope(|scope| {
        let c = || client.clone();
        let (c1, c2, c3, c4, c5, c6) = (c(), c(), c(), c(), c(), c());
        let hn = scope.spawn(move || fetch::fetch_hackernews(&c1));
        let ax = scope.spawn(move || fetch::fetch_arxiv(&c2));
        let gh = scope.spawn(move || fetch::fetch_github(&c3));
        let lo = scope.spawn(move || fetch::fetch_lobsters(&c4));
        let dv = scope.spawn(move || fetch::fetch_devto(&c5));
        let ar = scope.spawn(move || fetch::fetch_ars(&c6));

        let mut all = Vec::new();
        collect("Hacker News", hn.join(), &mut all);
        collect("arXiv", ax.join(), &mut all);
        collect("GitHub Trending", gh.join(), &mut all);
        collect("Lobsters", lo.join(), &mut all);
        collect("dev.to", dv.join(), &mut all);
        collect("Ars Technica", ar.join(), &mut all);
        all
    });
    eprintln!("collected {} signals total", signals.len());

    // A manifest of everything ingested today, for the printed page.
    let intake = build_intake(&signals);
    // The Pulse: one acceleration index aggregated from the day's signals,
    // persisted so it trends over time.
    let pulse = build_pulse(&signals, &date);
    // Lowercased corpus of today's signals, for self-grading open calls.
    let corpus = signals
        .iter()
        .map(|s| s.title.to_lowercase())
        .collect::<Vec<_>>()
        .join(" || ");

    let today_index = pulse.get("index").and_then(|v| v.as_i64()).unwrap_or(50);
    let picks = rank::rank_and_select(signals, seed, 4);
    let todays = generate::generate(&picks, &date, seed, today_index);
    eprintln!("generated {} call(s) for {date}", todays.len());

    // ---- merge across the embargo/reveal windows ----
    let mut revealed = load(DATA_PATH); // public, committed
    let mut embargoed = load(EMBARGO_IN); // subscriber pool, from KV

    // Idempotent re-run for the same day: drop today's everywhere, regenerate.
    revealed.retain(|p| p.date != date);
    embargoed.retain(|p| p.date != date);
    embargoed.extend(todays);

    // Self-grade: resolve any open call whose subject resurfaced in today's
    // signals (a HIT) or whose deadline has passed without resurfacing (a MISS).
    resolve_open(&mut revealed, &date, &corpus, today_index);
    resolve_open(&mut embargoed, &date, &corpus, today_index);

    // Promote any embargoed call old enough to go public.
    let mut still_embargoed = Vec::new();
    for p in embargoed {
        if age_days(&p.date, now) >= delay_days {
            revealed.push(p);
        } else {
            still_embargoed.push(p);
        }
    }
    let embargoed = still_embargoed;

    dedup(&mut revealed);

    // ---- persist both outputs ----
    save_json(DATA_PATH, &revealed); // committed by the Action
    write_early_payload(EARLY_OUT, &human, delay_days, &embargoed);

    // Access codes: encrypt the early feed under each shared code so a code
    // "just works" to unlock premium client-side. Set ACCESS_CODES (comma/space
    // separated). The engine does this in the normal build -- no extra scripts.
    let codes = env_or("ACCESS_CODES", "");
    if !codes.is_empty() {
        if let Ok(payload_json) = std::fs::read_to_string(EARLY_OUT) {
            access::publish(&format!("{OUT_DIR}/edge"), &payload_json, &codes);
        }
    }

    eprintln!(
        "revealed: {} public call(s); embargoed: {} subscriber-only call(s)",
        revealed.len(),
        embargoed.len()
    );

    // ---- render the public (delayed) page ----
    let mut archive: Vec<Prediction> = revealed.clone();
    archive.sort_by(|a, b| b.date.cmp(&a.date)); // newest first

    // Hero = the freshest revealed date's call(s), capped to the two lead calls.
    let featured_date = archive.first().map(|p| p.date.clone());
    let mut featured: Vec<Prediction> = match &featured_date {
        Some(d) => archive.iter().filter(|p| &p.date == d).cloned().collect(),
        None => Vec::new(),
    };
    featured.truncate(2);
    let featured_human = featured_date.as_deref().map(human_date).unwrap_or_default();

    if let Err(e) = render::render(
        &human,
        delay_days,
        &featured_human,
        &featured,
        &archive,
        &payment_link,
        &portal_url,
        &early_access_url,
        &intake,
        &pulse,
    ) {
        eprintln!("render error: {e}");
        std::process::exit(1);
    }
    // Ready-to-post syndication message for the daily auto-poster.
    let site = env_or("SITE_URL", "https://mattbusel.github.io/tech-oracle");
    let site = site.trim_end_matches('/');
    let idx = pulse.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
    let verdict = pulse.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    let theme = pulse.get("theme").and_then(|v| v.as_str()).unwrap_or("");
    let latest = featured.first().map(|p| p.prediction_text.as_str()).unwrap_or("");
    let hits = revealed.iter().filter(|p| p.status == "HIT").count();
    let misses = revealed.iter().filter(|p| p.status == "MISS").count();

    // Long form for Discord / Telegram / Mastodon: hook, the call, the record,
    // the index, a tail/fade nudge, and discovery hashtags.
    let social = format!(
        "The tech oracle's call for {human}:\n\n{latest}\n\nIt grades itself in public, currently {hits}-{misses}. Acceleration Index {idx} ({verdict}), hottest cluster {theme}.\n\nTail it or fade it: {site}/\n\n#tech #AI #buildinpublic #rustlang"
    );
    let _ = std::fs::write("build/social.txt", social);

    // Short form for Bluesky (300-char limit), trimmed call.
    let short_latest: String = if latest.chars().count() > 150 {
        format!("{}...", latest.chars().take(148).collect::<String>())
    } else {
        latest.to_string()
    };
    let social_short = format!(
        "Tech oracle, {hits}-{misses} and grading itself in public:\n\n{short_latest}\n\n{site}/ #tech #AI"
    );
    let _ = std::fs::write("build/social_short.txt", social_short);

    // Living README: update the marked block so the repo page is a daily showcase.
    update_readme_block(&human, idx, verdict, theme, hits, misses, latest, site);
    // Daily dispatch files for the GitHub issue (the free auto-newsletter).
    let _ = std::fs::write("build/dispatch_title.txt", format!("Dispatch {date}: {}", clip(latest, 70)));
    let _ = std::fs::write(
        "build/dispatch_body.md",
        format!("**{human}** // Acceleration Index **{idx} ({verdict})** // hottest cluster **{theme}** // self-graded record **{hits}-{misses}**\n\n> {latest}\n\nTail it or fade it: {site}/\n\n_Watch this repo to get tomorrow's call in your notifications._\n"),
    );

    eprintln!("wrote {DATA_PATH}, {EARLY_OUT}, {OUT_HTML}, feeds, social posts, and dispatch");
}

// --------------------------------------------------------------------------
// Helpers
// --------------------------------------------------------------------------

fn collect(
    name: &str,
    joined: std::thread::Result<anyhow::Result<Vec<model::Signal>>>,
    out: &mut Vec<model::Signal>,
) {
    match joined {
        Ok(Ok(mut v)) => {
            eprintln!("{name}: {} signals", v.len());
            out.append(&mut v);
        }
        Ok(Err(e)) => eprintln!("{name}: source failed, skipping ({e})"),
        Err(_) => eprintln!("{name}: source panicked, skipping"),
    }
}

/// Source label for a signal type, shared by the manifest and the page.
fn source_label(t: &str) -> &'static str {
    match t {
        "hn" => "HACKER NEWS",
        "arxiv" => "ARXIV",
        "github" => "GITHUB",
        "lobsters" => "LOBSTERS",
        "devto" => "DEV.TO",
        "ars" => "ARS TECHNICA",
        _ => "SIGNAL",
    }
}

/// Build the intake manifest: per-source count, a bar width, and the single
/// hottest item from each source. Pure summary of what was ingested today.
fn build_intake(signals: &[model::Signal]) -> serde_json::Value {
    use std::collections::HashMap;
    let mut groups: HashMap<&str, Vec<&model::Signal>> = HashMap::new();
    for s in signals {
        groups.entry(s.signal_type.as_str()).or_default().push(s);
    }

    let mut rows: Vec<(String, String, usize, String, String, f64)> = groups
        .into_iter()
        .map(|(t, v)| {
            let count = v.len();
            let top = v.iter().max_by(|a, b| {
                a.momentum_score
                    .partial_cmp(&b.momentum_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let (title, url) = top.map(|s| (s.title.clone(), s.url.clone())).unwrap_or_default();
            let score = top.map(|s| s.momentum_score).unwrap_or(0.0);
            (t.to_string(), source_label(t).to_string(), count, title, url, score)
        })
        .collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2));

    let max = rows.iter().map(|r| r.2).max().unwrap_or(1).max(1);
    let sources: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(t, label, count, title, url, score)| {
            serde_json::json!({
                "type": t,
                "label": label,
                "count": count,
                "pct": ((count as f64 / max as f64) * 100.0).round() as i64,
                "top_title": title,
                "top_url": url,
                "top_score": score as i64,
            })
        })
        .collect();

    serde_json::json!({ "total": signals.len(), "sources": sources })
}

/// Resolve open calls against today's signal corpus. A call only resolves on a
/// day after it was made (so it cannot settle on its own source), HITs if its
/// keyword resurfaces before the deadline, and MISSes once the deadline passes.
fn resolve_open(preds: &mut [Prediction], today: &str, corpus: &str, index: i64) {
    for p in preds.iter_mut() {
        if p.status != "OPEN" || p.date.as_str() >= today {
            continue;
        }
        let within = p.resolves_by.is_empty() || today <= p.resolves_by.as_str();
        let expired = !p.resolves_by.is_empty() && today > p.resolves_by.as_str();
        let here = |kw: &str| !kw.is_empty() && corpus.contains(kw);

        let (mut hit, mut miss) = (false, false);
        match p.market.as_str() {
            "HEAD-TO-HEAD" => {
                let a = here(&p.keyword);
                let b = here(&p.keyword2);
                if within && a && !b {
                    hit = true;
                } else if b && !a {
                    miss = true; // the other side moved first
                } else if expired {
                    miss = true;
                }
            }
            "INDEX" => {
                if within && index >= p.target {
                    hit = true;
                } else if expired {
                    miss = true;
                }
            }
            // RESURFACE / SURVIVAL / MOMENTUM / default: keyword resurfaces.
            _ => {
                if p.keyword.is_empty() {
                    continue;
                }
                if within && here(&p.keyword) {
                    hit = true;
                } else if expired {
                    miss = true;
                }
            }
        }
        if hit {
            p.status = "HIT".to_string();
            p.resolved_on = today.to_string();
        } else if miss {
            p.status = "MISS".to_string();
            p.resolved_on = today.to_string();
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct PulseDay {
    date: String,
    index: i64,
    theme: String,
}

/// THE PULSE: aggregate the day's signals into a single 0-100 acceleration
/// index, detect the hottest topic cluster, persist the daily reading, and
/// derive the deltas and history the page dramatizes.
fn build_pulse(signals: &[model::Signal], date: &str) -> serde_json::Value {
    let total = signals.len();
    let breadth = signals.iter().map(|s| s.signal_type.as_str()).collect::<HashSet<_>>().len();
    let (theme, theme_share) = dominant_theme(signals);

    let vol = (total as f64 / 180.0).min(1.0); // ~6 sources * 30 items
    let br = breadth as f64 / 6.0;
    let index = ((0.5 * vol + 0.3 * br + 0.2 * theme_share) * 100.0).round() as i64;
    let index = index.clamp(0, 100);

    // Load history, drop any existing entry for today, remember the prior reading.
    let mut hist: Vec<PulseDay> = match std::fs::read_to_string(PULSE_PATH) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let prev = hist
        .iter()
        .filter(|d| d.date.as_str() < date)
        .max_by(|a, b| a.date.cmp(&b.date))
        .map(|d| d.index);
    hist.retain(|d| d.date != date);
    hist.push(PulseDay { date: date.to_string(), index, theme: theme.clone() });
    hist.sort_by(|a, b| a.date.cmp(&b.date));

    if let Some(parent) = std::path::Path::new(PULSE_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(j) = serde_json::to_string_pretty(&hist) {
        let _ = std::fs::write(PULSE_PATH, j);
    }

    // How many consecutive prior prints were lower than today (the "highest in N").
    let mut highest_days = 0i64;
    for d in hist.iter().rev().skip(1) {
        if d.index < index {
            highest_days += 1;
        } else {
            break;
        }
    }

    let delta = prev.map(|p| index - p);
    let verdict = match index {
        0..=24 => "DORMANT",
        25..=44 => "CALM",
        45..=64 => "ACTIVE",
        65..=84 => "SURGING",
        _ => "OVERHEATING",
    };
    let start = hist.len().saturating_sub(14);
    let history: Vec<serde_json::Value> = hist[start..]
        .iter()
        .map(|d| serde_json::json!({ "date": d.date, "index": d.index }))
        .collect();

    serde_json::json!({
        "index": index,
        "verdict": verdict,
        "theme": theme,
        "has_prev": prev.is_some(),
        "delta_word": match delta { Some(x) if x > 0 => "RISING", Some(x) if x < 0 => "FALLING", Some(_) => "FLAT", None => "NEW" },
        "delta_abs": delta.map(|d| d.abs()).unwrap_or(0),
        "highest_days": highest_days,
        "total": total,
        "breadth": breadth,
        "history": history,
    })
}

/// Most-mentioned meaningful token across all signal titles, plus how dominant
/// it is (amplified, capped at 1.0) for the index. Returns an uppercased theme.
fn dominant_theme(signals: &[model::Signal]) -> (String, f64) {
    const STOP: &[&str] = &[
        "the", "and", "for", "with", "this", "that", "from", "your", "you", "what", "why", "how",
        "new", "show", "using", "via", "are", "was", "will", "can", "has", "have", "not", "but",
        "its", "out", "get", "all", "one", "like", "just", "now", "day", "into", "ask", "tell",
        "more", "less", "than", "who", "our", "their", "they", "them", "about", "over", "when",
        "first", "best", "good", "make", "made", "use", "used", "open", "source", "free", "code",
    ];
    let stop: HashSet<&str> = STOP.iter().copied().collect();
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_tokens = 0usize;
    for s in signals {
        for w in s.title.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
            if w.len() < 3 || stop.contains(w) || w.chars().all(|c| c.is_numeric()) {
                continue;
            }
            *counts.entry(w.to_string()).or_insert(0) += 1;
            total_tokens += 1;
        }
    }
    match counts.into_iter().max_by_key(|(_, c)| *c) {
        Some((word, count)) => {
            let share = ((count as f64 / total_tokens.max(1) as f64) * 4.0).min(1.0);
            (word.to_uppercase(), share)
        }
        None => ("QUIET".to_string(), 0.0),
    }
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}...", s.chars().take(n - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Replace the marked block in README.md with today's dispatch so the repo page
/// is a living daily showcase. No-op if the markers are absent.
fn update_readme_block(human: &str, idx: i64, verdict: &str, theme: &str, hits: usize, misses: usize, latest: &str, site: &str) {
    let path = "README.md";
    let (start, end) = ("<!--SIGNAL:START-->", "<!--SIGNAL:END-->");
    if let Ok(txt) = std::fs::read_to_string(path) {
        if let (Some(s), Some(e)) = (txt.find(start), txt.find(end)) {
            let block = format!(
                "{start}\n## Today on THE SIGNAL\n\n**{human}** // Index **{idx} ({verdict})** // hottest **{theme}** // record **{hits}-{misses}**\n\n> {latest}\n\nLive: {site}/ // Watch this repo for the daily dispatch.\n"
            );
            let new = format!("{}{}{}", &txt[..s], block, &txt[e..]);
            let _ = std::fs::write(path, new);
        }
    }
}

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key)
        .ok()
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn load(path: &str) -> Vec<Prediction> {
    match std::fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_else(|e| {
            eprintln!("warn: could not parse {path} ({e}); treating as empty");
            Vec::new()
        }),
        Err(_) => Vec::new(),
    }
}

fn save_json(path: &str, archive: &[Prediction]) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(archive).expect("serialize");
    std::fs::write(path, json).unwrap_or_else(|e| panic!("write {path}: {e}"));
}

/// Emit the subscriber edge payload: embargoed calls plus the date each becomes
/// public. The Action pushes this into Cloudflare KV; the Worker serves it.
fn write_early_payload(path: &str, human: &str, delay_days: i64, embargoed: &[Prediction]) {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let mut sorted: Vec<&Prediction> = embargoed.iter().collect();
    sorted.sort_by(|a, b| b.date.cmp(&a.date));

    let preds: Vec<serde_json::Value> = sorted
        .iter()
        .map(|p| {
            serde_json::json!({
                "date": p.date,
                "prediction_text": p.prediction_text,
                "source_title": p.source_title,
                "source_url": p.source_url,
                "signal_type": p.signal_type,
                "public_reveal_date": reveal_date(&p.date, delay_days),
            })
        })
        .collect();

    let payload = serde_json::json!({
        "generated_human": human,
        "reveal_delay_days": delay_days,
        "count": preds.len(),
        "predictions": preds,
    });
    let json = serde_json::to_string_pretty(&payload).expect("serialize payload");
    std::fs::write(path, json).unwrap_or_else(|e| panic!("write {path}: {e}"));
}

fn dedup(v: &mut Vec<Prediction>) {
    let mut seen = HashSet::new();
    v.retain(|p| seen.insert(format!("{}|{}", p.date, p.prediction_text)));
}

/// Whole days elapsed between a YYYY-MM-DD string and `now`. Unparseable dates
/// are treated as ancient so they reveal rather than getting stuck embargoed.
fn age_days(date: &str, now: NaiveDate) -> i64 {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => (now - d).num_days(),
        Err(_) => i64::MAX,
    }
}

fn reveal_date(date: &str, delay_days: i64) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => (d + Duration::days(delay_days))
            .format("%Y-%m-%d")
            .to_string(),
        Err(_) => date.to_string(),
    }
}

fn human_date(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d.format("%B %-d, %Y").to_string(),
        Err(_) => date.to_string(),
    }
}
