//! One-time historical backfill: reconstruct the corpus's daily attention
//! trajectory from the keyless archives that go back in time, so the manifold
//! skips the warmup phase. Three sources are blended per day, spanning the
//! diffusion funnel so even historical crossings register:
//!   - arXiv (technical origin, stage 0): papers submitted that day,
//!   - Hacker News (developer, stage 4): that day's top stories (Algolia archive),
//!   - Wikipedia (general public, stage 9): that day's most-viewed articles.
//!
//! Each day's blended signals are folded in through the IDENTICAL live path
//! (`observatory::build`), oldest first, so the term ledger, daily snapshots,
//! peaks, `days` counts and diffusion stages populate exactly as if the engine
//! had been running all along. `build` sorts and bounds `corpus.days`, so
//! out-of-order folding is safe.
//!
//! Run once via `tech-oracle backfill [days]`; the normal daily run appends
//! today's full ten-source snapshot on top.

use crate::fetch;
use crate::model::Signal;
use crate::observatory;
use chrono::{Duration, NaiveDate, Utc};
use std::time::Duration as StdDuration;

// ---- Hacker News (Algolia archive) ----------------------------------------

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

fn fetch_hn_day(client: &reqwest::blocking::Client, start: i64, end: i64, k: usize) -> Vec<Signal> {
    let url = format!(
        "https://hn.algolia.com/api/v1/search?tags=story&numericFilters=created_at_i>={start},created_at_i<{end}&hitsPerPage={k}"
    );
    let resp: AlgoliaResp = match client.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.json()) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    resp.hits
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
        .collect()
}

// ---- Wikipedia (most-viewed per day) --------------------------------------

#[derive(serde::Deserialize)]
struct WikiResp {
    items: Vec<WikiDay>,
}
#[derive(serde::Deserialize)]
struct WikiDay {
    articles: Vec<WikiArticle>,
}
#[derive(serde::Deserialize)]
struct WikiArticle {
    article: String,
    views: f64,
}

fn fetch_wiki_day(client: &reqwest::blocking::Client, d: NaiveDate, k: usize) -> Vec<Signal> {
    let url = format!(
        "https://wikimedia.org/api/rest_v1/metrics/pageviews/top/en.wikipedia/all-access/{}",
        d.format("%Y/%m/%d")
    );
    let resp: WikiResp = match client.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.json()) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    if let Some(day) = resp.items.into_iter().next() {
        for a in day.articles {
            if a.article.contains(':') || a.article == "Main_Page" || a.article == "-" {
                continue; // skip namespaced / meta pages
            }
            let title = a.article.replace('_', " ");
            if title.trim().is_empty() {
                continue;
            }
            out.push(Signal {
                signal_type: "wiki".into(),
                url: format!("https://en.wikipedia.org/wiki/{}", a.article),
                title,
                momentum_score: a.views,
            });
            if out.len() >= k {
                break;
            }
        }
    }
    out
}

// ---- arXiv (papers submitted that day) ------------------------------------

fn fetch_arxiv_day(client: &reqwest::blocking::Client, d: NaiveDate, k: usize) -> Vec<Signal> {
    let day = d.format("%Y%m%d").to_string();
    let cats = "cat:cs.AI+OR+cat:cs.LG+OR+cat:cs.CL+OR+cat:cs.SE+OR+cat:cs.CR+OR+cat:cs.DC+OR+cat:cs.NI+OR+cat:cs.DS+OR+cat:cs.RO+OR+cat:stat.ML";
    let url = format!(
        "http://export.arxiv.org/api/query?search_query=%28{cats}%29+AND+submittedDate:[{day}0000+TO+{day}2359]&max_results={k}&sortBy=submittedDate&sortOrder=descending"
    );
    let bytes = match client.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.bytes()) {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };
    let feed = match feed_rs::parser::parse(&bytes[..]) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let n = feed.entries.len();
    feed.entries
        .into_iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let title = e.title.map(|t| t.content)?;
            if title.trim().is_empty() {
                return None;
            }
            let url = e
                .links
                .iter()
                .find(|l| l.rel.as_deref() == Some("alternate"))
                .or_else(|| e.links.first())
                .map(|l| l.href.clone())
                .unwrap_or_default();
            Some(Signal { signal_type: "arxiv".into(), title, url, momentum_score: (n - i) as f64 })
        })
        .collect()
}

/// Backfill the last `days` calendar days (excluding today, which the live run
/// owns), blending arXiv + HN + Wikipedia per day. Returns days folded in.
pub fn run(days: i64) -> i64 {
    let client = fetch::client();
    let today = Utc::now().date_naive();
    let mut filled = 0i64;
    let (mut hn_tot, mut wiki_tot, mut arx_tot) = (0i64, 0i64, 0i64);
    // Oldest first so the ledger accumulates in chronological order.
    for back in (1..=days).rev() {
        let d = today - Duration::days(back);
        let date = d.format("%Y-%m-%d").to_string();
        let start = d.and_hms_opt(0, 0, 0).map(|t| t.and_utc().timestamp()).unwrap_or(0);
        let end = start + 86_400;

        let mut sigs = fetch_hn_day(&client, start, end, 150);
        let hn = sigs.len();
        let wiki = {
            let w = fetch_wiki_day(&client, d, 80);
            let n = w.len();
            sigs.extend(w);
            n
        };
        let arx = {
            let a = fetch_arxiv_day(&client, d, 60);
            let n = a.len();
            sigs.extend(a);
            n
        };
        hn_tot += hn as i64;
        wiki_tot += wiki as i64;
        arx_tot += arx as i64;

        if sigs.len() >= 5 {
            observatory::build(&sigs, &date);
            filled += 1;
            eprintln!("backfill {date}: {} signals (hn {hn} / wiki {wiki} / arxiv {arx})", sigs.len());
        } else {
            eprintln!("backfill {date}: only {} signals, skipped", sigs.len());
        }
        // Be polite to the public archives (arXiv asks for spacing especially).
        std::thread::sleep(StdDuration::from_millis(400));
    }
    eprintln!(
        "backfill complete: {filled}/{days} days // totals hn {hn_tot} / wiki {wiki_tot} / arxiv {arx_tot}"
    );
    filled
}
