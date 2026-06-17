//! One-time historical backfill: reconstruct the corpus's daily attention
//! trajectory from Hacker News' public Algolia archive so the manifold skips the
//! warmup phase. The corpus measures term mentions in titles; HN's archive is the
//! richest keyless source of exactly that, going back years.
//!
//! It uses the IDENTICAL live methodology: each historical day's top stories are
//! turned into Signals and folded in through `observatory::build`, oldest first,
//! so the term ledger, the daily snapshots, peaks and `days` counts all populate
//! exactly as if the engine had been running all along. `build` sorts and bounds
//! `corpus.days`, so out-of-order folding is safe.
//!
//! Run once via `tech-oracle backfill [days]`; then the normal daily run appends
//! today's full ten-source snapshot on top. The historical portion is HN-only (a
//! consistent, dev-centric attention proxy); the live portion is all ten sources.

use crate::fetch;
use crate::model::Signal;
use crate::observatory;
use chrono::{Duration, Utc};

#[derive(serde::Deserialize)]
struct AlgoliaHit {
    title: Option<String>,
    url: Option<String>,
    points: Option<f64>,
}

#[derive(serde::Deserialize)]
struct AlgoliaResp {
    hits: Vec<AlgoliaHit>,
}

/// The top stories created in [start, end) unix seconds, popularity-ranked, so the
/// volume and selection roughly match a day's front page (the live corpus scale).
fn fetch_hn_day(
    client: &reqwest::blocking::Client,
    start: i64,
    end: i64,
    k: usize,
) -> anyhow::Result<Vec<Signal>> {
    let url = format!(
        "https://hn.algolia.com/api/v1/search?tags=story&numericFilters=created_at_i>={start},created_at_i<{end}&hitsPerPage={k}"
    );
    let resp: AlgoliaResp = client.get(url).send()?.error_for_status()?.json()?;
    Ok(resp
        .hits
        .into_iter()
        .filter_map(|h| {
            let title = h.title?;
            if title.trim().is_empty() {
                return None;
            }
            Some(Signal {
                signal_type: "hn".into(),
                title,
                url: h.url.unwrap_or_default(),
                momentum_score: h.points.unwrap_or(0.0),
            })
        })
        .collect())
}

/// Backfill the last `days` calendar days (excluding today, which the live run
/// owns). Returns the number of days actually folded in.
pub fn run(days: i64) -> i64 {
    let client = fetch::client();
    let today = Utc::now().date_naive();
    let mut filled = 0i64;
    // Oldest first so the ledger accumulates in chronological order.
    for back in (1..=days).rev() {
        let d = today - Duration::days(back);
        let date = d.format("%Y-%m-%d").to_string();
        let start = d.and_hms_opt(0, 0, 0).map(|t| t.and_utc().timestamp()).unwrap_or(0);
        let end = start + 86_400;
        // A denser daily sample (top ~150 stories) so per-topic counts are smooth
        // enough for the manifold to read a real trajectory, not sparse noise.
        match fetch_hn_day(&client, start, end, 150) {
            Ok(sigs) if sigs.len() >= 5 => {
                observatory::build(&sigs, &date);
                filled += 1;
                eprintln!("backfill {date}: {} stories folded in", sigs.len());
            }
            Ok(sigs) => eprintln!("backfill {date}: only {} stories, skipped", sigs.len()),
            Err(e) => eprintln!("backfill {date}: {e}"),
        }
    }
    eprintln!("backfill complete: {filled}/{days} day(s) reconstructed from Hacker News");
    filled
}
