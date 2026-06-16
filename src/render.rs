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
