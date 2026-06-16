//! Signal fetching. Each `fetch_*` is independent and returns a Result so the
//! caller can fail soft: a dead source is logged and skipped, never fatal.

use crate::model::Signal;
use std::time::Duration;

/// Shared HTTP client. `blocking::Client` is cheap to clone (Arc inside) and is
/// Send + Sync, so it can be handed to the per-source threads.
pub fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .user_agent("tech-oracle/0.1 (+https://github.com; static site generator)")
        .timeout(Duration::from_secs(20))
        .build()
        .expect("failed to build HTTP client")
}

// ---------------------------------------------------------------------------
// Hacker News (Firebase API)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct HnItem {
    title: Option<String>,
    url: Option<String>,
    score: Option<f64>,
}

pub fn fetch_hackernews(client: &reqwest::blocking::Client) -> anyhow::Result<Vec<Signal>> {
    let ids: Vec<u64> = client
        .get("https://hacker-news.firebaseio.com/v0/topstories.json")
        .send()?
        .error_for_status()?
        .json()?;
    let top: Vec<u64> = ids.into_iter().take(30).collect();

    // The 30 item lookups are independent -- fetch them concurrently.
    let signals = std::thread::scope(|scope| {
        let handles: Vec<_> = top
            .iter()
            .map(|&id| {
                let client = client.clone();
                scope.spawn(move || fetch_hn_item(&client, id).ok().flatten())
            })
            .collect();
        handles
            .into_iter()
            .filter_map(|h| h.join().ok().flatten())
            .collect::<Vec<Signal>>()
    });

    Ok(signals)
}

fn fetch_hn_item(client: &reqwest::blocking::Client, id: u64) -> anyhow::Result<Option<Signal>> {
    let url = format!("https://hacker-news.firebaseio.com/v0/item/{id}.json");
    // Deleted/dead items come back as JSON `null`.
    let item: Option<HnItem> = client.get(url).send()?.error_for_status()?.json()?;
    let Some(item) = item else { return Ok(None) };
    let Some(title) = item.title else { return Ok(None) };
    let link = item
        .url
        .unwrap_or_else(|| format!("https://news.ycombinator.com/item?id={id}"));
    Ok(Some(Signal {
        signal_type: "hn".into(),
        title: collapse_ws(&title),
        url: link,
        momentum_score: item.score.unwrap_or(0.0),
    }))
}

// ---------------------------------------------------------------------------
// arXiv (Atom feed)
// ---------------------------------------------------------------------------

pub fn fetch_arxiv(client: &reqwest::blocking::Client) -> anyhow::Result<Vec<Signal>> {
    let url = "http://export.arxiv.org/api/query?search_query=cat:cs.AI+OR+cat:cs.LG&sortBy=submittedDate&sortOrder=descending&max_results=30";
    let bytes = client.get(url).send()?.error_for_status()?.bytes()?;
    let feed = feed_rs::parser::parse(&bytes[..])?;

    let n = feed.entries.len();
    let signals = feed
        .entries
        .into_iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let title = e.title.map(|t| collapse_ws(&t.content))?;
            if title.is_empty() {
                return None;
            }
            // Prefer the human "alternate" abstract link; fall back to the first link.
            let url = e
                .links
                .iter()
                .find(|l| l.rel.as_deref() == Some("alternate"))
                .or_else(|| e.links.first())
                .map(|l| l.href.clone())?;
            Some(Signal {
                signal_type: "arxiv".into(),
                title,
                url,
                // The feed is sorted newest-first; recency rank is the momentum proxy.
                momentum_score: (n - i) as f64,
            })
        })
        .collect();

    Ok(signals)
}

// ---------------------------------------------------------------------------
// GitHub Trending (HTML scrape -- no official API)
// ---------------------------------------------------------------------------

pub fn fetch_github(client: &reqwest::blocking::Client) -> anyhow::Result<Vec<Signal>> {
    let html = client
        .get("https://github.com/trending?since=daily")
        .send()?
        .error_for_status()?
        .text()?;

    // Each trending row exposes its repo in an <h2> anchor (href="/owner/repo")
    // and its stars-today as "N stars today". We capture both in document order
    // and zip them. If the markup shifts the regexes simply match less and we
    // return fewer/zero signals -- never a crash.
    let repo_re =
        regex::Regex::new(r#"<h2[^>]*>\s*<a[^>]*href="/([A-Za-z0-9._-]+/[A-Za-z0-9._-]+)""#).unwrap();
    let stars_re = regex::Regex::new(r#"([\d,]+)\s+stars\s+today"#).unwrap();

    let repos: Vec<String> = repo_re
        .captures_iter(&html)
        .filter_map(|c| c.get(1).map(|m| m.as_str().to_string()))
        .take(25)
        .collect();
    let stars: Vec<f64> = stars_re
        .captures_iter(&html)
        .filter_map(|c| c.get(1).map(|m| parse_leading_number(m.as_str())))
        .collect();

    let out = repos
        .into_iter()
        .enumerate()
        .map(|(i, repo)| Signal {
            signal_type: "github".into(),
            url: format!("https://github.com/{repo}"),
            title: repo,
            momentum_score: stars.get(i).copied().unwrap_or(0.0),
        })
        .collect();

    Ok(out)
}

// ---------------------------------------------------------------------------
// Lobsters (JSON, no auth)
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct LobItem {
    title: String,
    url: Option<String>,
    score: Option<f64>,
    short_id: Option<String>,
}

pub fn fetch_lobsters(client: &reqwest::blocking::Client) -> anyhow::Result<Vec<Signal>> {
    let items: Vec<LobItem> = client
        .get("https://lobste.rs/hottest.json")
        .send()?
        .error_for_status()?
        .json()?;
    Ok(items
        .into_iter()
        .take(25)
        .filter_map(|it| {
            let title = collapse_ws(&it.title);
            if title.is_empty() {
                return None;
            }
            let url = it
                .url
                .filter(|u| !u.is_empty())
                .or_else(|| it.short_id.map(|s| format!("https://lobste.rs/s/{s}")))?;
            Some(Signal {
                signal_type: "lobsters".into(),
                title,
                url,
                momentum_score: it.score.unwrap_or(0.0),
            })
        })
        .collect())
}

// ---------------------------------------------------------------------------
// dev.to (JSON, no auth) -- top articles of the last week
// ---------------------------------------------------------------------------

#[derive(serde::Deserialize)]
struct DevItem {
    title: String,
    url: String,
    positive_reactions_count: Option<f64>,
}

pub fn fetch_devto(client: &reqwest::blocking::Client) -> anyhow::Result<Vec<Signal>> {
    let items: Vec<DevItem> = client
        .get("https://dev.to/api/articles?per_page=30&top=7")
        .send()?
        .error_for_status()?
        .json()?;
    Ok(items
        .into_iter()
        .take(25)
        .filter_map(|it| {
            let title = collapse_ws(&it.title);
            if title.is_empty() {
                return None;
            }
            Some(Signal {
                signal_type: "devto".into(),
                title,
                url: it.url,
                momentum_score: it.positive_reactions_count.unwrap_or(0.0),
            })
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Ars Technica (RSS/Atom feed, no auth)
// ---------------------------------------------------------------------------

pub fn fetch_ars(client: &reqwest::blocking::Client) -> anyhow::Result<Vec<Signal>> {
    let bytes = client
        .get("https://feeds.arstechnica.com/arstechnica/index")
        .send()?
        .error_for_status()?
        .bytes()?;
    let feed = feed_rs::parser::parse(&bytes[..])?;
    let n = feed.entries.len();
    Ok(feed
        .entries
        .into_iter()
        .enumerate()
        .filter_map(|(i, e)| {
            let title = e.title.map(|t| collapse_ws(&t.content))?;
            if title.is_empty() {
                return None;
            }
            let url = e
                .links
                .iter()
                .find(|l| l.rel.as_deref() == Some("alternate"))
                .or_else(|| e.links.first())
                .map(|l| l.href.clone())?;
            Some(Signal {
                signal_type: "ars".into(),
                title,
                url,
                momentum_score: (n - i) as f64,
            })
        })
        .collect())
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Collapse whitespace and sanitize borrowed source text to our house style:
/// normalize typographic dashes/quotes/ellipsis to ASCII and strip emoji and
/// pictographic symbols. Accented letters are kept (legible, not in scope).
fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\u{2014}' | '\u{2013}' | '\u{2012}' | '\u{2015}' | '\u{2212}' => { out.push('-'); continue; }
            '\u{2018}' | '\u{2019}' | '\u{201B}' | '\u{2032}' => { out.push('\''); continue; }
            '\u{201C}' | '\u{201D}' | '\u{2033}' => { out.push('"'); continue; }
            '\u{2026}' => { out.push_str("..."); continue; }
            '\u{00A0}' => { out.push(' '); continue; }
            _ => {}
        }
        let c = ch as u32;
        let strip = c >= 0x1F000
            || (0x2600..=0x27BF).contains(&c)   // misc symbols + dingbats
            || (0x2190..=0x21FF).contains(&c)   // arrows
            || (0x2300..=0x23FF).contains(&c)   // technical (incl. emoji-ish)
            || (0x2B00..=0x2BFF).contains(&c)   // misc symbols/arrows
            || (0x1F1E6..=0x1F1FF).contains(&c) // regional indicators
            || c == 0xFE0F || c == 0x200D;      // variation selector, ZWJ
        if !strip {
            out.push(ch);
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Parse the leading integer out of strings like "1,234 stars today".
fn parse_leading_number(s: &str) -> f64 {
    let digits: String = s
        .trim()
        .chars()
        .take_while(|c| c.is_ascii_digit() || *c == ',')
        .collect();
    digits.replace(',', "").parse().unwrap_or(0.0)
}
