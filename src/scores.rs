//! Cross-browser leaderboard, server-free. Players submit a score by opening a
//! prefilled GitHub issue (label `score`) whose body contains one line:
//!   SIGNAL-FLOOR-SCORE v=1 handle=<h> nw=<int> pull=<LABEL> pullval=<int>
//! The daily Action runs `tech-oracle harvest`: it reads those issues via the
//! GitHub API, validates/clamps, dedupes to one best entry per GitHub account
//! (anti-spam), and bakes a static `docs/api/leaderboard.json` the site reads.
//! No backend: the engine is the aggregator and moderator, exactly like the
//! certified champions registry. The read path is a static file, so it never
//! hits the GitHub API rate limit no matter how many people load the page.

use std::collections::HashMap;

#[derive(serde::Deserialize)]
struct GhUser {
    login: Option<String>,
}
#[derive(serde::Deserialize)]
struct GhIssue {
    body: Option<String>,
    user: Option<GhUser>,
    // The /issues endpoint also returns pull requests; this lets us skip them.
    pull_request: Option<serde_json::Value>,
}

struct Entry {
    handle: String,
    nw: f64,
    pull: String,
    pullval: f64,
}

// Sanity ceilings: a score above these is a forgery, not a flex. Clamp, do not drop.
const NW_CAP: f64 = 1e21;
const PV_CAP: f64 = 1e15;

fn sanitize_handle(s: &str) -> String {
    let h: String = s
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .take(24)
        .collect();
    if h.is_empty() { "ANON".to_string() } else { h.to_uppercase() }
}

fn sanitize_label(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-' || *c == '/')
        .take(28)
        .collect::<String>()
        .trim()
        .to_uppercase()
}

fn parse_line(body: &str) -> Option<Entry> {
    let line = body.lines().find(|l| l.contains("SIGNAL-FLOOR-SCORE"))?;
    let mut handle = String::new();
    let (mut nw, mut pullval) = (0f64, 0f64);
    let mut pull = String::new();
    for tok in line.split_whitespace() {
        if let Some(v) = tok.strip_prefix("handle=") {
            handle = sanitize_handle(v);
        } else if let Some(v) = tok.strip_prefix("nw=") {
            nw = v.parse::<f64>().unwrap_or(0.0);
        } else if let Some(v) = tok.strip_prefix("pull=") {
            pull = sanitize_label(&v.replace('_', " "));
        } else if let Some(v) = tok.strip_prefix("pullval=") {
            pullval = v.parse::<f64>().unwrap_or(0.0);
        }
    }
    if handle.is_empty() {
        handle = "ANON".into();
    }
    if !nw.is_finite() || nw < 0.0 {
        nw = 0.0;
    }
    if !pullval.is_finite() || pullval < 0.0 {
        pullval = 0.0;
    }
    Some(Entry { handle, nw: nw.min(NW_CAP), pull, pullval: pullval.min(PV_CAP) })
}

/// Read all `score` issues for `repo`, bake `out_dir/api/leaderboard.json`. Never
/// panics and never fails the build: on any network/parse error it logs and
/// leaves any existing leaderboard.json untouched.
pub fn harvest(repo: &str, out_dir: &str, generated: &str) {
    let client = crate::fetch::client();
    let url = format!(
        "https://api.github.com/repos/{repo}/issues?labels=score&state=all&per_page=100&sort=updated"
    );
    let mut req = client.get(&url).header("Accept", "application/vnd.github+json");
    if let Ok(tok) = std::env::var("GITHUB_TOKEN").or_else(|_| std::env::var("GH_TOKEN")) {
        if !tok.is_empty() {
            req = req.header("Authorization", format!("Bearer {tok}"));
        }
    }
    let issues: Vec<GhIssue> = match req.send().and_then(|r| r.error_for_status()).and_then(|r| r.json()) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("harvest: fetch failed ({e}); leaving leaderboard.json as-is");
            return;
        }
    };

    // Dedupe by GitHub account: keep each account's highest net worth. One person
    // cannot stuff the board, and reposting a bigger score updates your entry.
    let mut best: HashMap<String, Entry> = HashMap::new();
    for iss in issues {
        if iss.pull_request.is_some() {
            continue;
        }
        let body = match iss.body {
            Some(b) => b,
            None => continue,
        };
        let login = iss.user.and_then(|u| u.login).unwrap_or_else(|| "anon".into()).to_lowercase();
        if let Some(e) = parse_line(&body) {
            let keep = best.get(&login).map(|prev| e.nw > prev.nw).unwrap_or(true);
            if keep {
                best.insert(login, e);
            }
        }
    }

    let mut entries: Vec<Entry> = best.into_values().collect();
    entries.sort_by(|a, b| b.nw.partial_cmp(&a.nw).unwrap_or(std::cmp::Ordering::Equal));
    let networth: Vec<serde_json::Value> = entries
        .iter()
        .take(100)
        .map(|e| serde_json::json!({ "handle": e.handle, "nw": e.nw }))
        .collect();

    let mut byp: Vec<&Entry> = entries.iter().filter(|e| e.pullval > 0.0).collect();
    byp.sort_by(|a, b| b.pullval.partial_cmp(&a.pullval).unwrap_or(std::cmp::Ordering::Equal));
    let pulls: Vec<serde_json::Value> = byp
        .iter()
        .take(100)
        .map(|e| serde_json::json!({ "handle": e.handle, "pull": e.pull, "pullval": e.pullval }))
        .collect();

    let doc = serde_json::json!({
        "schema": "the-signal/leaderboard/1",
        "generated": generated,
        "count": entries.len(),
        "networth": networth,
        "pulls": pulls
    });
    let _ = std::fs::create_dir_all(format!("{out_dir}/api"));
    match std::fs::write(
        format!("{out_dir}/api/leaderboard.json"),
        serde_json::to_string_pretty(&doc).unwrap_or_default(),
    ) {
        Ok(_) => eprintln!("harvest: wrote leaderboard.json with {} entries", entries.len()),
        Err(e) => eprintln!("harvest: write failed ({e})"),
    }
}
