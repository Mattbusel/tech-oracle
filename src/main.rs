mod fetch;
mod generate;
mod model;
mod rank;
mod render;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use model::Prediction;
use std::collections::HashSet;

// Committed, public: the revealed track record. Renders the public page.
const DATA_PATH: &str = "data/predictions.json";
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

    // ---- fetch the three sources concurrently; each fails soft ----
    let client = fetch::client();
    let signals = std::thread::scope(|scope| {
        let (c1, c2, c3) = (client.clone(), client.clone(), client.clone());
        let hn = scope.spawn(move || fetch::fetch_hackernews(&c1));
        let ax = scope.spawn(move || fetch::fetch_arxiv(&c2));
        let gh = scope.spawn(move || fetch::fetch_github(&c3));

        let mut all = Vec::new();
        collect("Hacker News", hn.join(), &mut all);
        collect("arXiv", ax.join(), &mut all);
        collect("GitHub Trending", gh.join(), &mut all);
        all
    });
    eprintln!("collected {} signals total", signals.len());

    let picks = rank::rank_and_select(signals, seed, 2);
    let todays = generate::generate(&picks, &date, seed);
    eprintln!("generated {} call(s) for {date}", todays.len());

    // ---- merge across the embargo/reveal windows ----
    let mut revealed = load(DATA_PATH); // public, committed
    let mut embargoed = load(EMBARGO_IN); // subscriber pool, from KV

    // Idempotent re-run for the same day: drop today's everywhere, regenerate.
    revealed.retain(|p| p.date != date);
    embargoed.retain(|p| p.date != date);
    embargoed.extend(todays);

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
    write_early_payload(EARLY_OUT, &human, delay_days, &embargoed); // pushed to KV

    eprintln!(
        "revealed: {} public call(s); embargoed: {} subscriber-only call(s)",
        revealed.len(),
        embargoed.len()
    );

    // ---- render the public (delayed) page ----
    let mut archive: Vec<Prediction> = revealed.clone();
    archive.sort_by(|a, b| b.date.cmp(&a.date)); // newest first

    // Hero = the freshest revealed date's call(s).
    let featured_date = archive.first().map(|p| p.date.clone());
    let featured: Vec<Prediction> = match &featured_date {
        Some(d) => archive.iter().filter(|p| &p.date == d).cloned().collect(),
        None => Vec::new(),
    };
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
    ) {
        eprintln!("render error: {e}");
        std::process::exit(1);
    }
    eprintln!("wrote {DATA_PATH}, {EARLY_OUT}, and {OUT_HTML}");
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
