//! Render the public (delayed) page from the revealed archive using minijinja.
//! The template file is embedded at compile time, so the binary stays
//! self-contained. This module knows nothing about payments; it only renders
//! whatever the caller decided is public, plus static subscribe links.

use crate::model::Prediction;
use chrono::NaiveDate;
use std::collections::HashMap;

#[allow(clippy::too_many_arguments)]
pub fn render(
    generated_human: &str,
    reveal_delay_days: i64,
    featured_date_human: &str,
    featured: &[Prediction],
    archive: &[Prediction],
    payment_link: &str,
    portal_url: &str,
    early_access_url: &str,
    intake: &serde_json::Value,
    pulse: &serde_json::Value,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(crate::OUT_DIR)?;

    // Newest-first. YYYY-MM-DD sorts lexicographically.
    let mut sorted: Vec<&Prediction> = archive.iter().collect();
    sorted.sort_by(|a, b| b.date.cmp(&a.date));
    let total = sorted.len();

    // Group the ledger into dated "pages" (fanfold pages), each with running
    // call numbers (newest = highest).
    let mut pages: Vec<serde_json::Value> = Vec::new();
    let mut i = 0;
    while i < sorted.len() {
        let date = sorted[i].date.clone();
        let mut items = Vec::new();
        while i < sorted.len() && sorted[i].date == date {
            let p = sorted[i];
            let status = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
            let conf = if p.confidence > 0.0 { p.confidence } else { 0.65 };
            items.push(serde_json::json!({
                "no": total - i,
                "prediction_text": p.prediction_text,
                "source_url": p.source_url,
                "signal_type": p.signal_type,
                "status": status,
                "win_if": p.win_if,
                "resolved_on": p.resolved_on,
                "odds": format!("{:.2}x", 1.0 / conf),
                "conf": (conf * 100.0).round() as i64,
                "market": if p.market.is_empty() { "RESURFACE".to_string() } else { p.market.clone() },
            }));
            i += 1;
        }
        pages.push(serde_json::json!({
            "date": date,
            "human": human_date(&date),
            "count": items.len(),
            "items": items,
        }));
    }

    // A flat oldest-first list for the punch-card "signal map".
    let mut calls: Vec<serde_json::Value> = sorted
        .iter()
        .rev()
        .map(|p| serde_json::json!({ "date": p.date, "signal_type": p.signal_type }))
        .collect();
    if calls.len() > 120 {
        calls = calls.split_off(calls.len() - 120); // keep the most recent 120 dots
    }

    // Record summary: per-source counts across the whole public archive.
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for p in archive {
        *counts.entry(p.signal_type.as_str()).or_insert(0) += 1;
    }
    let mut by_source: Vec<(String, String, usize, i64)> = counts
        .into_iter()
        .map(|(t, c)| {
            let pct = if total > 0 { (c as f64 / total as f64 * 100.0).round() as i64 } else { 0 };
            (t.to_string(), crate::source_label(t).to_string(), c, pct)
        })
        .collect();
    by_source.sort_by(|a, b| b.2.cmp(&a.2));
    let by_source: Vec<serde_json::Value> = by_source
        .into_iter()
        .map(|(t, label, count, pct)| serde_json::json!({ "type": t, "label": label, "count": count, "pct": pct }))
        .collect();

    // The scorecard: the viral artifact. Tally settled and open calls.
    let (mut hits, mut misses, mut open) = (0i64, 0i64, 0i64);
    for p in archive {
        match p.status.as_str() {
            "HIT" => hits += 1,
            "MISS" => misses += 1,
            _ => open += 1,
        }
    }
    let resolved = hits + misses;
    let rate = if resolved > 0 { Some(hits * 100 / resolved) } else { None };
    let verdict = match rate {
        Some(r) if r >= 70 => "THE ORACLE IS BEATING THE STREET",
        Some(r) if r >= 50 => "AHEAD OF THE CROWD",
        Some(_) => "UNDERWATER, AND NOT HIDING IT",
        None => "NO BETS SETTLED YET",
    };
    let scoreboard = serde_json::json!({
        "hits": hits, "misses": misses, "open": open,
        "resolved": resolved, "has_rate": rate.is_some(),
        "rate": rate.unwrap_or(0), "verdict": verdict,
    });

    // THE BOOK: a flat-stake virtual bankroll wagered on the oracle's own
    // calls, settled in chronological order. The line (decimal odds) is 1/conf,
    // so favorites pay little and longshots pay big.
    let mut chrono: Vec<&Prediction> = archive.iter().collect();
    chrono.sort_by(|a, b| a.date.cmp(&b.date));
    let start_bank = 1000.0_f64;
    let stake = 100.0_f64;
    let mut bank = start_bank;
    let mut bank_hist: Vec<f64> = Vec::new();
    let mut bank_dates: Vec<String> = Vec::new();
    let (mut cur, mut best_win, mut best_loss, mut settled) = (0i64, 0i64, 0i64, 0i64);
    let mut last_win: Option<bool> = None;
    for p in &chrono {
        let conf = if p.confidence > 0.0 { p.confidence.clamp(0.5, 0.95) } else { 0.65 };
        let win = match p.status.as_str() {
            "HIT" => true,
            "MISS" => false,
            _ => continue,
        };
        if win {
            bank += stake * ((1.0 / conf) - 1.0);
        } else {
            bank -= stake;
        }
        settled += 1;
        match last_win {
            Some(l) if l == win => cur += 1,
            _ => cur = 1,
        }
        last_win = Some(win);
        if win && cur > best_win { best_win = cur; }
        if !win && cur > best_loss { best_loss = cur; }
        bank_hist.push(bank);
        bank_dates.push(p.date.clone());
    }
    let pnl = bank - start_bank;
    let roi = (pnl / start_bank * 100.0).round() as i64;
    let (mn, mx) = bank_hist.iter().fold((f64::MAX, f64::MIN), |(a, b), &v| (a.min(v), b.max(v)));
    let span = bank_dates.len().saturating_sub(20);
    let book_history: Vec<serde_json::Value> = bank_hist
        .iter()
        .zip(bank_dates.iter())
        .skip(span)
        .map(|(&v, d)| {
            let pct = if mx > mn { ((v - mn) / (mx - mn) * 100.0).round().max(4.0) } else { 50.0 };
            serde_json::json!({ "date": d, "pct": pct as i64 })
        })
        .collect();
    let book = serde_json::json!({
        "bank": bank.round() as i64,
        "roi_str": format!("{}{}%", if pnl >= 0.0 { "+" } else { "" }, roi),
        "pnl_class": if pnl >= 0.0 { "sb-hit" } else { "sb-miss" },
        "streak": match last_win { Some(true) => format!("W{cur}"), Some(false) => format!("L{cur}"), None => "--".to_string() },
        "best_win": best_win, "best_loss": best_loss,
        "settled": settled, "history": book_history,
    });

    let since_human = sorted.last().map(|p| human_date(&p.date)).unwrap_or_default();
    let record = serde_json::json!({
        "total": total,
        "since": since_human,
        "by_source": by_source,
    });

    // Site base URL for feeds, structured data, and share links.
    let site = std::env::var("SITE_URL")
        .unwrap_or_else(|_| "https://mattbusel.github.io/tech-oracle".to_string());
    let site = site.trim_end_matches('/').to_string();

    // JSON-LD structured data (SEO: each call as a CreativeWork in an ItemList).
    let ld_items: Vec<serde_json::Value> = sorted
        .iter()
        .take(15)
        .enumerate()
        .map(|(i, p)| {
            serde_json::json!({
                "@type": "ListItem", "position": i + 1,
                "item": { "@type": "CreativeWork", "headline": p.prediction_text, "datePublished": p.date, "url": format!("{site}/#call-{}", total - i) }
            })
        })
        .collect();
    let jsonld = serde_json::json!({
        "@context": "https://schema.org", "@type": "WebSite", "name": "THE SIGNAL",
        "url": site, "description": "A self-grading public oracle of dated, falsifiable tech predictions.",
        "mainEntity": { "@type": "ItemList", "itemListElement": ld_items }
    })
    .to_string();

    // RSS feed: the syndication source (subscribe, aggregators, IFTTT/Zapier).
    let mut feed = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<rss version=\"2.0\"><channel>\n");
    feed.push_str(&format!(
        "<title>THE SIGNAL // dated tech calls</title>\n<link>{site}/</link>\n<description>A self-grading public oracle. Dated, falsifiable tech calls, graded in public.</description>\n"
    ));
    for (i, p) in sorted.iter().enumerate() {
        let no = total - i;
        let title = if p.prediction_text.chars().count() > 90 {
            format!("{}...", p.prediction_text.chars().take(88).collect::<String>())
        } else {
            p.prediction_text.clone()
        };
        let status = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
        let desc = format!("{} // {} // {}", p.prediction_text, status, p.win_if);
        feed.push_str(&format!(
            "<item><title>{}</title><link>{}/#call-{}</link><guid isPermaLink=\"false\">signal-{}</guid><pubDate>{}</pubDate><description>{}</description></item>\n",
            xml(&title), site, no, no, rfc822(&p.date), xml(&desc)
        ));
    }
    feed.push_str("</channel></rss>\n");
    std::fs::write(format!("{}/feed.xml", crate::OUT_DIR), feed)?;

    let sitemap = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\"><url><loc>{site}/</loc></url></urlset>\n"
    );
    std::fs::write(format!("{}/sitemap.xml", crate::OUT_DIR), sitemap)?;

    // Live floor positions: the most recent calls the desk marks against the
    // live feeds (client-side, continuously).
    let floor: Vec<serde_json::Value> = sorted
        .iter()
        .take(8)
        .map(|p| {
            let conf = if p.confidence > 0.0 { p.confidence } else { 0.65 };
            let kw = if p.keyword.is_empty() { "signal".to_string() } else { p.keyword.clone() };
            let t: String = p.prediction_text.chars().take(50).collect();
            serde_json::json!({
                "t": t, "kw": kw,
                "market": if p.market.is_empty() { "RESURFACE".to_string() } else { p.market.clone() },
                "odds": format!("{:.2}", 1.0 / conf),
                "status": if p.status.is_empty() { "OPEN" } else { p.status.as_str() },
            })
        })
        .collect();
    let floor_json = serde_json::to_string(&floor).unwrap_or_else(|_| "[]".to_string());

    let tmpl_src = include_str!("../templates/index.html");
    let mut env = minijinja::Environment::new();
    env.add_template("index", tmpl_src)?;
    let tmpl = env.get_template("index")?;

    let html = tmpl.render(minijinja::context! {
        generated_human => generated_human,
        reveal_delay_days => reveal_delay_days,
        featured_date_human => featured_date_human,
        featured => featured,
        pages => pages,
        calls => calls,
        record => record,
        intake => intake,
        pulse => pulse,
        scoreboard => scoreboard,
        book => book,
        jsonld => jsonld,
        floor_json => floor_json,
        total => total,
        payment_link => payment_link,
        portal_url => portal_url,
        early_access_url => early_access_url,
    })?;

    std::fs::write(crate::OUT_HTML, html)?;
    Ok(())
}

fn human_date(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d.format("%B %-d, %Y").to_string(),
        Err(_) => date.to_string(),
    }
}

fn rfc822(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d.format("%a, %d %b %Y 13:17:00 +0000").to_string(),
        Err(_) => date.to_string(),
    }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
