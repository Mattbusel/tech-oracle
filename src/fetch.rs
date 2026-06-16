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
// Helpers
// ---------------------------------------------------------------------------

fn collapse_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
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
