//! Render the public (delayed) page from the revealed archive using minijinja.
//! The template file is embedded at compile time, so the binary stays
//! self-contained. This module knows nothing about payments; it only renders
//! whatever the caller decided is public, plus static subscribe links.

use crate::model::Prediction;
use chrono::{NaiveDate, Utc};
use std::collections::{BTreeMap, HashMap};

// IndexNow ownership key (not secret; proves we control the host via a key file).
const INDEXNOW_KEY: &str = "0f9c2a7b5e3d4148a6c1b2e3f4a5d6c7";

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
    genome: &serde_json::Value,
    engine: &serde_json::Value,
    dreams: &serde_json::Value,
    bloodline: &serde_json::Value,
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
                "rationale": p.rationale,
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
        let conf = if p.confidence > 0.0 { p.confidence.clamp(0.34, 0.95) } else { 0.65 };
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

    // CALIBRATION: the engine grades its own honesty. For every settled call,
    // compare the confidence it set (predicted P(hit)) to the actual outcome.
    // Brier score = mean squared error; lower is better, 0.25 = a coin flip.
    let mut brier_sum = 0.0f64;
    let mut cal_n = 0i64;
    // Three confidence bands: longshots, even money, favorites.
    let bands = [(0.0f64, 0.55f64, "LONGSHOTS"), (0.55, 0.7, "EVEN MONEY"), (0.7, 1.01, "FAVORITES")];
    let mut band_acc: Vec<(f64, i64, i64)> = vec![(0.0, 0, 0); bands.len()]; // sum_pred, hits, n
    for p in archive {
        let outcome = match p.status.as_str() {
            "HIT" => 1.0,
            "MISS" => 0.0,
            _ => continue,
        };
        let conf = if p.confidence > 0.0 { p.confidence.clamp(0.34, 0.95) } else { 0.65 };
        brier_sum += (conf - outcome).powi(2);
        cal_n += 1;
        if let Some(bi) = bands.iter().position(|(lo, hi, _)| conf >= *lo && conf < *hi) {
            band_acc[bi].0 += conf;
            band_acc[bi].1 += outcome as i64;
            band_acc[bi].2 += 1;
        }
    }
    let brier = if cal_n > 0 { (brier_sum / cal_n as f64 * 1000.0).round() / 1000.0 } else { 0.0 };
    let cal_buckets: Vec<serde_json::Value> = bands
        .iter()
        .enumerate()
        .filter(|(i, _)| band_acc[*i].2 > 0)
        .map(|(i, (_, _, label))| {
            let (sum_pred, hits, n) = band_acc[i];
            let pred = (sum_pred / n as f64 * 100.0).round() as i64;
            let actual = (hits * 100) / n;
            serde_json::json!({ "label": label, "pred": pred, "actual": actual, "n": n })
        })
        .collect();
    // Skill grade: how close predicted tracks actual across the bands.
    let cal_err: i64 = cal_buckets
        .iter()
        .map(|b| (b["pred"].as_i64().unwrap_or(0) - b["actual"].as_i64().unwrap_or(0)).abs())
        .sum::<i64>()
        .checked_div(cal_buckets.len().max(1) as i64)
        .unwrap_or(0);
    let cal_grade = match (cal_n, cal_err) {
        (0, _) => "NO SETTLED CALLS YET",
        (_, e) if e <= 8 => "SHARP: THE LINE MEANS WHAT IT SAYS",
        (_, e) if e <= 18 => "HONEST: ROUGHLY CALIBRATED",
        _ => "MISCALIBRATED, AND SHOWING IT",
    };
    let calibration = serde_json::json!({
        "brier": format!("{:.3}", brier),
        "has_data": cal_n > 0,
        "n": cal_n,
        "buckets": cal_buckets,
        "grade": cal_grade,
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
    let ladder_repo = std::env::var("LADDER_REPO")
        .or_else(|_| std::env::var("GITHUB_REPOSITORY"))
        .unwrap_or_else(|_| "Mattbusel/tech-oracle".to_string());

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

    // Programmatic SEO: one crawlable permalink page per revealed call.
    let _ = std::fs::create_dir_all(format!("{}/call", crate::OUT_DIR));
    let mut urls = vec![format!("{site}/")];
    let mut img_entries = vec![format!(
        "<url><loc>{site}/</loc><image:image><image:loc>{site}/og.png</image:loc><image:title>THE SIGNAL: daily tech oracle and live betting pit</image:title></image:image></url>"
    )];
    for (i, p) in sorted.iter().enumerate() {
        let no = total - i;
        let status = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
        let market = if p.market.is_empty() { "RESURFACE" } else { p.market.as_str() };
        let desc = xml(&clip_r(&p.prediction_text, 150));
        let tt = xml(&clip_r(&p.prediction_text, 65));
        // The call as a dated, machine-readable claim. A resolved call becomes a
        // ClaimReview (the date proves we called it first); an open call is a
        // dated Claim. This is what lets search and AI engines cite the receipt.
        let claim_text = clip_r(&p.prediction_text, 240);
        let claim_ld = match p.status.as_str() {
            "HIT" | "MISS" => {
                let (rv, name) = if p.status == "HIT" { (5, "Resolved HIT: the call was correct") } else { (1, "Resolved MISS: the call was wrong") };
                serde_json::json!({
                    "@context": "https://schema.org", "@type": "ClaimReview",
                    "datePublished": p.date, "url": format!("{site}/call/{no}.html"),
                    "claimReviewed": claim_text,
                    "author": { "@type": "Organization", "name": "THE SIGNAL", "url": site },
                    "reviewRating": { "@type": "Rating", "ratingValue": rv, "bestRating": 5, "worstRating": 1, "alternateName": name },
                    "itemReviewed": { "@type": "Claim", "datePublished": p.date, "author": { "@type": "Organization", "name": "THE SIGNAL" }, "appearance": { "@type": "CreativeWork", "url": format!("{site}/call/{no}.html") } }
                })
            }
            _ => serde_json::json!({
                "@context": "https://schema.org", "@type": "CreativeWork",
                "headline": claim_text, "datePublished": p.date,
                "url": format!("{site}/call/{no}.html"),
                "author": { "@type": "Organization", "name": "THE SIGNAL", "url": site }
            }),
        }.to_string();
        let page = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>Call No. {no}: {tt} // THE SIGNAL</title>\n<meta name=\"description\" content=\"{desc}\">\n<meta property=\"og:title\" content=\"THE SIGNAL // Call No. {no} [{status}]\">\n<meta property=\"og:description\" content=\"{desc}\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<meta property=\"og:image\" content=\"{site}/call/{no}.png\">\n<meta name=\"twitter:image\" content=\"{site}/call/{no}.png\">\n<link rel=\"canonical\" href=\"{site}/call/{no}.html\">\n<script type=\"application/ld+json\">{ld}</script>\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:620px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}.c{{font-size:25px;font-weight:600;line-height:1.35;margin:18px 0}}.m{{font-size:11px;letter-spacing:.1em;color:#6d6b5e}}.w{{font-size:12px;color:#6d6b5e;margin:14px 0}}.r{{display:inline-block;font-weight:600;padding:3px 10px;letter-spacing:.12em;font-size:12px}}.r-hit{{background:#1f7a3d;color:#efede4}}.r-miss{{background:#b23a2e;color:#efede4}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // CALL No. {no}</div>\n<div class=\"m\">{date} // {market} // {status}</div>\n{receipt}<p class=\"c\">{t}</p>\n<div class=\"w\">{win}</div>\n<p class=\"m\"><a href=\"{src}\" rel=\"noopener\">source signal</a> // <a href=\"{site}/receipts.html\">the receipts</a> // <a href=\"{site}/#call-{no}\">on the public record</a> // <a href=\"{site}/\">THE SIGNAL</a></p>\n</div></body></html>\n",
            no = no, tt = tt, t = xml(&p.prediction_text), desc = desc, status = status, market = market,
            date = xml(&p.date), win = xml(&p.win_if), src = xml(&p.source_url), site = site, ld = claim_ld,
            receipt = match p.status.as_str() {
                "HIT" => format!("<p><span class=\"r r-hit\">CALLED IT // HIT</span> <span class=\"m\">called {} // resolved {} // {} days on the record</span></p>\n", p.date, p.resolved_on, day_diff(&p.date, &p.resolved_on)),
                "MISS" => format!("<p><span class=\"r r-miss\">ON THE RECORD // MISS</span> <span class=\"m\">called {} // resolved {} // no edits, no deletes</span></p>\n", p.date, p.resolved_on),
                _ => String::new(),
            }
        );
        let _ = std::fs::write(format!("{}/call/{no}.html", crate::OUT_DIR), page);
        urls.push(format!("{site}/call/{no}.html"));
        // og:image for this call page
        let _ = crate::card::call_card(
            &format!("{}/call/{no}.png", crate::OUT_DIR),
            &site, no as i64, status, market, &p.prediction_text,
        );
        img_entries.push(format!(
            "<url><loc>{site}/call/{no}.html</loc><image:image><image:loc>{site}/call/{no}.png</image:loc><image:caption>{cap}</image:caption></image:image></url>",
            cap = xml(&clip_r(&p.prediction_text, 140))
        ));
    }
    // Topic pages: group the archive by subject so the site matches real search
    // queries ("<topic> predictions"), not just one call's exact wording.
    let mut topics: BTreeMap<String, Vec<(usize, &Prediction)>> = BTreeMap::new();
    for (i, p) in sorted.iter().enumerate() {
        if !p.keyword.is_empty() {
            topics.entry(slug(&p.keyword)).or_default().push((total - i, p));
        }
    }
    let _ = std::fs::create_dir_all(format!("{}/topic", crate::OUT_DIR));
    for (sl, calls) in &topics {
        let topic = sl.to_uppercase();
        let items: String = calls
            .iter()
            .map(|(no, p)| {
                let st = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
                format!(
                    "<li class=\"i\"><span class=\"m\">{date} // {st}</span><br><a href=\"{site}/call/{no}.html\">{t}</a></li>",
                    date = xml(&p.date), st = st, no = no, t = xml(&clip_r(&p.prediction_text, 130)), site = site
                )
            })
            .collect();
        let page = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>{topic}: dated tech predictions // THE SIGNAL</title>\n<meta name=\"description\" content=\"Every dated, self-graded call on {topic} from THE SIGNAL, a public tech-prediction oracle.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // {topic} predictions\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<link rel=\"canonical\" href=\"{site}/topic/{sl}.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:640px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:26px;letter-spacing:.04em}}ul{{list-style:none;padding:0}}.i{{padding:12px 0;border-bottom:1px dashed rgba(27,26,20,.3);font-size:15px;line-height:1.4}}.m{{font-size:11px;letter-spacing:.1em;color:#6d6b5e}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // TOPIC</div>\n<h1>{topic}</h1><p class=\"m\">Dated, self-graded calls on {topic}.</p>\n<ul>{items}</ul>\n<p class=\"m\"><a href=\"{site}/\">THE SIGNAL // the full record</a></p></div></body></html>\n",
            topic = topic, sl = sl, items = items, site = site
        );
        let _ = std::fs::write(format!("{}/topic/{sl}.html", crate::OUT_DIR), page);
        urls.push(format!("{site}/topic/{sl}.html"));
    }

    // THE RECEIPTS: the credibility wall. Every dated call that has settled,
    // newest first, with how many days early it went on the record. "We called
    // it, here is the proof" is the most shareable thing the engine produces.
    {
        let mut hit_rows = String::new();
        let mut miss_rows = String::new();
        let (mut nh, mut nm) = (0i64, 0i64);
        for (i, p) in sorted.iter().enumerate() {
            let no = total - i;
            match p.status.as_str() {
                "HIT" => {
                    nh += 1;
                    hit_rows.push_str(&format!(
                        "<li class=\"i\"><div class=\"rh\"><span class=\"r r-hit\">HIT</span> <span class=\"lead\">{lead} DAYS ON THE RECORD</span></div><a href=\"{site}/call/{no}.html\">{t}</a><div class=\"meta\">CALLED {called} // RESOLVED {res} // {mk}</div></li>",
                        lead = day_diff(&p.date, &p.resolved_on), site = site, no = no,
                        t = xml(&clip_r(&p.prediction_text, 150)), called = xml(&p.date), res = xml(&p.resolved_on),
                        mk = if p.market.is_empty() { "RESURFACE" } else { p.market.as_str() }
                    ));
                }
                "MISS" => {
                    nm += 1;
                    miss_rows.push_str(&format!(
                        "<li class=\"i\"><div class=\"rh\"><span class=\"r r-miss\">MISS</span> <span class=\"lead\">NO EDITS, NO DELETES</span></div><a href=\"{site}/call/{no}.html\">{t}</a><div class=\"meta\">CALLED {called} // RESOLVED {res} // {mk}</div></li>",
                        site = site, no = no, t = xml(&clip_r(&p.prediction_text, 150)), called = xml(&p.date), res = xml(&p.resolved_on),
                        mk = if p.market.is_empty() { "RESURFACE" } else { p.market.as_str() }
                    ));
                }
                _ => {}
            }
        }
        let hits_block = if nh > 0 { format!("<h2>CALLED IT // HITS</h2><ul>{hit_rows}</ul>") } else { String::new() };
        let miss_block = if nm > 0 { format!("<h2>ON THE RECORD // MISSES</h2><ul>{miss_rows}</ul>") } else { String::new() };
        let body = if nh + nm > 0 {
            format!("{hits_block}{miss_block}")
        } else {
            "<p class=\"empty\">No calls have settled yet. The first receipts print soon. Nothing here is editable once it does.</p>".to_string()
        };
        let receipts = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>The Receipts: tech predictions THE SIGNAL called first, dated and graded</title>\n<meta name=\"description\" content=\"Every dated, self-graded tech prediction from THE SIGNAL, printed before it resolved. {nh} hits and {nm} misses on the public record. No edits, no deletes.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // THE RECEIPTS [{nh}-{nm}]\">\n<meta property=\"og:description\" content=\"We called it, here is the dated proof. {nh} hits, {nm} misses, every one on the record.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<link rel=\"canonical\" href=\"{site}/receipts.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:680px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:30px;letter-spacing:.04em;margin:18px 0 4px}}h2{{font-size:13px;letter-spacing:.16em;border-top:1px solid rgba(27,26,20,.25);padding-top:14px;margin-top:26px}}.sub{{font-size:13px;color:#6d6b5e;line-height:1.5}}ul{{list-style:none;padding:0}}.i{{padding:13px 0;border-bottom:1px dashed rgba(27,26,20,.3)}}.i a{{color:#1b1a14;font-size:15px;font-weight:600;line-height:1.4;text-decoration:none}}.i a:hover{{text-decoration:underline}}.rh{{margin-bottom:5px}}.r{{display:inline-block;font-weight:600;padding:2px 9px;letter-spacing:.12em;font-size:11px}}.r-hit{{background:#1f7a3d;color:#efede4}}.r-miss{{background:#b23a2e;color:#efede4}}.lead{{font-size:10.5px;letter-spacing:.1em;color:#6d6b5e;margin-left:6px}}.meta{{font-size:10.5px;letter-spacing:.08em;color:#6d6b5e;margin-top:5px}}.empty{{color:#6d6b5e}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // THE RECEIPTS</div>\n<h1>WE CALLED IT</h1>\n<p class=\"sub\">Every prediction below was printed and dated before it resolved. The machine grades itself in public: {nh} hits, {nm} misses on the record. No edits, no deletes, only prints.</p>\n{body}\n<p class=\"meta\"><a href=\"{site}/\">back to THE SIGNAL</a> // <a href=\"{site}/dataset/\">the open dataset</a></p></div></body></html>\n",
            nh = nh, nm = nm, site = site, body = body
        );
        std::fs::write(format!("{}/receipts.html", crate::OUT_DIR), receipts)?;
        urls.push(format!("{site}/receipts.html"));
    }

    // THE ARENA: a serverless prediction tournament. Anyone or any AI agent
    // enters a dated bet by opening a GitHub issue labeled "arena" with a
    // SIGNAL-BET line. The board is rendered client-side: it reads the issues
    // and the public record, settles every bet, and ranks players (with the
    // engine itself and an anti-oracle as standing competitors). GitHub Issues
    // is the database; there is no server.
    {
        let arena = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>The Arena: humans and AI agents vs the machine // THE SIGNAL</title>\n<meta name=\"description\" content=\"A serverless prediction tournament. Tail or fade the oracle's dated calls; the board settles every bet against the public record and ranks every player, human or AI, against the machine.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // THE ARENA\">\n<meta property=\"og:description\" content=\"Humans and AI agents bet against the machine. Every call dated, every bet settled in public.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<link rel=\"canonical\" href=\"{site}/arena.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:720px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:30px;letter-spacing:.04em;margin:18px 0 4px}}.sub{{font-size:13px;color:#6d6b5e;line-height:1.55}}table{{width:100%;border-collapse:collapse;margin-top:14px}}th,td{{text-align:left;padding:9px 8px;border-bottom:1px dashed rgba(27,26,20,.3);font-size:13px}}th{{font-size:10.5px;letter-spacing:.12em;color:#6d6b5e}}.rank{{color:#6d6b5e;width:28px}}.you{{background:rgba(91,240,138,.18)}}.eng{{font-weight:700}}.sc-up{{color:#1f7a3d;font-weight:700}}.sc-dn{{color:#b23a2e;font-weight:700}}.title{{font-size:10.5px;letter-spacing:.08em;color:#6d6b5e}}.btn{{display:inline-block;text-align:center;border:1.5px solid #1b1a14;padding:12px 18px;margin:14px 8px 0 0;text-decoration:none;color:#1b1a14;font-weight:600;letter-spacing:.06em;cursor:pointer;background:none}}.btn:hover{{background:#1b1a14;color:#efede4}}code{{background:rgba(27,26,20,.1);padding:1px 5px}}.fmt{{font-size:12px;color:#3b3a30;background:rgba(27,26,20,.06);padding:12px;margin-top:14px;line-height:1.6;white-space:pre-wrap}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // THE ARENA</div>\n<h1>BEAT THE MACHINE</h1>\n<p class=\"sub\">Tail or fade the oracle's dated calls. Every bet is settled in public against the record, no edits, no deletes. The machine and its shadow, the anti-oracle, stand on the board as permanent competitors. Humans enter from their browser; AI agents enter through the API. There is no server: the entries are public GitHub issues.</p>\n<div id=\"board\"><p class=\"sub\">Reading the record and settling the floor...</p></div>\n<a class=\"btn\" id=\"enter\">[ ENTER A BET ]</a>\n<a class=\"btn\" href=\"{site}/\">[ BACK TO THE SIGNAL ]</a>\n<div class=\"fmt\">HOW TO ENTER<br>Humans: tap ENTER A BET (it opens a prefilled GitHub issue).<br>Agents: open an issue on the repo, label it <code>arena</code>, body containing one line:<br>  SIGNAL-BET kw=&lt;keyword&gt; market=&lt;MARKET&gt; side=&lt;TAIL|FADE&gt; by=&lt;your handle&gt;<br>TAIL backs the machine's call; FADE bets against it. Settled HIT/MISS from {site}/api/record.json. Bet on calls listed in {site}/api/today.json.</div>\n<script>\nvar REPO={repo}, SITE={site_js};\nvar TITLES=[[10,'LEGEND OF THE DEN'],[6,'ORACLE-KILLER'],[3,'SHARP'],[1,'CONTENDER'],[-2,'ROOKIE'],[-1e9,'THE MARK']];\nfunction titleFor(s){{for(var i=0;i<TITLES.length;i++)if(s>=TITLES[i][0])return TITLES[i][1];return 'ROOKIE';}}\nfunction esc(t){{var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}}\nfunction mine(){{try{{var c=localStorage.getItem('signal_cred');return c?c.split('-')[0].toUpperCase():null;}}catch(e){{return null;}}}}\nvar BET=/SIGNAL-BET\\s+kw=(\\S+)\\s+market=(\\S+)\\s+side=(TAIL|FADE)\\s+by=(.+)/i;\nPromise.all([\n  fetch('api/record.json').then(function(r){{return r.ok?r.json():null;}}).catch(function(){{return null;}}),\n  fetch('https://api.github.com/repos/'+REPO+'/issues?labels=arena&state=all&per_page=100',{{headers:{{Accept:'application/vnd.github+json'}}}}).then(function(r){{return r.ok?r.json():[];}}).catch(function(){{return [];}})\n]).then(function(res){{\n  var rec=res[0]||{{}}, issues=res[1]||[];\n  var calls=(rec.calls)||[]; var byKw={{}};\n  calls.forEach(function(c){{var k=(c.keyword||'').toLowerCase();if(k&&!byKw[k])byKw[k]=c;}});\n  var players={{}};\n  function P(by){{if(!players[by])players[by]={{by:by,w:0,l:0,p:0}};return players[by];}}\n  issues.forEach(function(it){{var m=BET.exec(it.body||'');if(!m)return;var kw=m[1].toLowerCase(),side=m[3].toUpperCase(),by=(m[4]||'').trim().slice(0,24).toUpperCase()||'ANON';var c=byKw[kw];var pl=P(by);if(!c||c.status==='OPEN'){{pl.p++;return;}}var win=(c.status==='HIT')===(side==='TAIL');if(win)pl.w++;else pl.l++;}});\n  var sb=rec.scoreboard||{{hits:0,misses:0}};\n  var rows=Object.keys(players).map(function(k){{var p=players[k];p.score=p.w-p.l;return p;}});\n  rows.push({{by:'THE MACHINE',w:sb.hits||0,l:sb.misses||0,p:sb.open||0,score:(sb.hits||0)-(sb.misses||0),eng:1}});\n  rows.push({{by:'THE ANTI-ORACLE',w:sb.misses||0,l:sb.hits||0,p:0,score:(sb.misses||0)-(sb.hits||0),eng:1}});\n  rows.sort(function(a,b){{return b.score-a.score;}});\n  var me=mine();\n  var html='<table><tr><th class=rank>#</th><th>PLAYER</th><th>W-L</th><th>SCORE</th><th>TITLE</th></tr>';\n  rows.forEach(function(p,i){{var cls=(p.eng?'eng':'')+((me&&p.by===me)?' you':'');var sc=(p.score>=0?'+':'')+p.score;html+='<tr class=\"'+cls+'\"><td class=rank>'+(i+1)+'</td><td>'+esc(p.by)+'</td><td>'+p.w+'-'+p.l+(p.p?(' ('+p.p+'open)'):'')+'</td><td class=\"'+(p.score>=0?'sc-up':'sc-dn')+'\">'+sc+'</td><td class=title>'+titleFor(p.score)+'</td></tr>';}});\n  html+='</table>';\n  if(!issues.length)html+='<p class=\"sub\">No challengers yet. The machine is undefeated by default. Be the first to enter.</p>';\n  document.getElementById('board').innerHTML=html;\n}});\ndocument.getElementById('enter').addEventListener('click',function(e){{e.preventDefault();var by=mine()||'anon';var calls=[];try{{}}catch(e2){{}}var kw=prompt('Keyword to bet on (see today.json on the site):','');if(!kw)return;var side=(prompt('TAIL (back the machine) or FADE (bet against it)?','TAIL')||'TAIL').toUpperCase();if(side!=='TAIL'&&side!=='FADE')side='TAIL';var body='My bet in THE SIGNAL arena.\\n\\nSIGNAL-BET kw='+kw.toLowerCase().replace(/[^a-z0-9]/g,'')+' market=ANY side='+side+' by='+by+'\\n\\nThe board: '+SITE+'arena.html';var url='https://github.com/'+REPO+'/issues/new?labels=arena&title='+encodeURIComponent('Arena: '+by+' '+side+' '+kw)+'&body='+encodeURIComponent(body);window.open(url,'_blank','noopener');}});\n</script>\n</div></body></html>\n",
            site = site, repo = serde_json::to_string(&ladder_repo).unwrap_or_else(|_| "\"\"".to_string()),
            site_js = serde_json::to_string(&format!("{site}/")).unwrap_or_else(|_| "\"\"".to_string())
        );
        std::fs::write(format!("{}/arena.html", crate::OUT_DIR), arena)?;
        urls.push(format!("{site}/arena.html"));
    }

    // SLEEP MODE: a destination, not a takeover. A living dreamscape that never
    // stops, recombining the corpus into new far-future calls forever, client
    // side. Reached on purpose; it never ambushes anyone.
    {
        let pool_js = serde_json::to_string(dreams.get("pool").unwrap_or(&serde_json::json!([]))).unwrap_or_else(|_| "[]".to_string());
        let forms_js = serde_json::to_string(dreams.get("forms").unwrap_or(&serde_json::json!([]))).unwrap_or_else(|_| "[]".to_string());
        let sleep = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>Sleep Mode: the oracle dreams // THE SIGNAL</title>\n<meta name=\"description\" content=\"The oracle never stops. In sleep mode it recombines everything it has seen into surreal far-future calls, endlessly. A living dreamscape.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // SLEEP MODE\">\n<meta property=\"og:description\" content=\"The oracle dreams while you are away. A living, always-running dreamscape.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<link rel=\"canonical\" href=\"{site}/sleep.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>html,body{{margin:0;height:100%}}body{{background:#07060f;color:#c3b4ff;font-family:'IBM Plex Mono',ui-monospace,monospace;overflow:hidden}}#field{{position:fixed;inset:0;transition:background 8s linear;background:radial-gradient(circle at 50% 30%,#171247,#07060f 72%)}}.wrap{{position:relative;z-index:2;height:100%;display:flex;flex-direction:column}}.top{{padding:26px 24px 8px;text-align:center;flex:0 0 auto}}.h{{font-size:12px;letter-spacing:.42em;color:#8a7ad6}}.t{{font-size:11px;letter-spacing:.18em;color:#6a5db0;margin-top:8px}}.count{{font-size:10.5px;letter-spacing:.16em;color:#4e4488;margin-top:6px}}#stream{{flex:1 1 auto;overflow:hidden;position:relative;padding:10px 24px 30px}}.dream{{max-width:660px;margin:0 auto;font-size:clamp(16px,3.6vw,24px);line-height:1.5;text-align:center;padding:14px 0;opacity:0;transform:translateY(14px);transition:opacity 2.2s ease,transform 2.2s ease}}.dream.in{{opacity:.92;transform:none}}.dream.out{{opacity:0;transform:translateY(-22px)}}.dream b{{color:#e7deff;font-weight:700}}.deep{{font-size:clamp(22px,5vw,34px);color:#efe9ff}}.bar{{position:fixed;bottom:0;left:0;right:0;z-index:3;display:flex;gap:16px;justify-content:center;padding:14px;background:linear-gradient(transparent,#07060f 60%)}}.bar a,.bar button{{color:#c3b4ff;background:none;border:1px solid rgba(195,180,255,.35);padding:9px 16px;text-decoration:none;font-family:inherit;letter-spacing:.1em;font-size:12px;cursor:pointer}}.bar a:hover,.bar button:hover{{background:rgba(195,180,255,.12)}}.star{{position:fixed;width:2px;height:2px;background:#b9a7ff;border-radius:50%;opacity:0;z-index:1;animation:tw 6s ease-in-out infinite}}@keyframes tw{{0%,100%{{opacity:0}}50%{{opacity:.5}}}}@media (prefers-reduced-motion:reduce){{.dream{{transition:none}}.star{{animation:none}}}}</style></head>\n<body>\n<div id=\"field\"></div>\n<div class=\"wrap\"><div class=\"top\"><div class=\"h\">THE ORACLE IS DREAMING</div><div class=\"t\">IT RECOMBINES WHAT IT HAS SEEN INTO FUTURES THAT DO NOT EXIST YET</div><div class=\"count\" id=\"count\">DREAM No. 0</div></div><div id=\"stream\"></div></div>\n<div class=\"bar\"><a href=\"{site}/\">[ WAKE THE ORACLE ]</a><button id=\"faster\" type=\"button\">[ DREAM FASTER ]</button></div>\n<script>\nvar POOL={pool_js}, FORMS={forms_js};\nif(!POOL.length){{POOL=['THE SIGNAL','THE MACHINE','THE FUTURE'];}}\nif(!FORMS.length){{FORMS=['In the long night, {{a}} and {{b}} are the same machine.'];}}\nvar stream=document.getElementById('stream'),countEl=document.getElementById('count'),field=document.getElementById('field');\nvar n=0, speed=3400, hues=[245,265,225,285,205];\nfunction pick(a){{return a[Math.floor(Math.random()*a.length)];}}\nfunction esc(t){{var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}}\nfunction make(){{var a=pick(POOL),b=pick(POOL);for(var i=0;i<4&&b===a;i++)b=pick(POOL);var f=pick(FORMS);return f.split('{{a}}').join('<b>'+esc(a)+'</b>').split('{{b}}').join('<b>'+esc(b)+'</b>');}}\nfunction emit(){{n++;countEl.textContent='DREAM No. '+n;var el=document.createElement('div');var deep=Math.random()<0.15;el.className='dream'+(deep?' deep':'');el.innerHTML=make();stream.appendChild(el);requestAnimationFrame(function(){{el.classList.add('in');}});var kids=stream.children;if(kids.length>9){{var old=kids[0];old.classList.add('out');setTimeout(function(){{if(old.parentNode)old.parentNode.removeChild(old);}},2300);}}var hue=pick(hues);field.style.background='radial-gradient(circle at '+(30+Math.random()*40)+'% '+(20+Math.random()*30)+'%, hsl('+hue+',55%,18%), #07060f 72%)';}}\nfunction loop(){{emit();setTimeout(loop, speed + (Math.random()*1600-400));}}\nfor(var s=0;s<40;s++){{var st=document.createElement('div');st.className='star';st.style.left=(Math.random()*100)+'vw';st.style.top=(Math.random()*100)+'vh';st.style.animationDelay=(Math.random()*6)+'s';document.body.appendChild(st);}}\nemit();emit();setTimeout(loop,speed);\ndocument.getElementById('faster').addEventListener('click',function(){{speed=speed>1400?1500:3400;this.textContent=speed<2000?'[ DREAM SLOWER ]':'[ DREAM FASTER ]';}});\n</script>\n</body></html>\n",
            site = site, pool_js = pool_js, forms_js = forms_js
        );
        std::fs::write(format!("{}/sleep.html", crate::OUT_DIR), sleep)?;
        urls.push(format!("{site}/sleep.html"));
    }

    // THE BLOODLINE, LIVE: a broadcast you tune into. The day's population is
    // baked in; the client runs it as a live channel with animated standings, a
    // house race, an events feed, and a rolling commentary you can switch the
    // voice on for. Always running.
    {
        let bl_js = serde_json::to_string(bloodline).unwrap_or_else(|_| "{}".to_string());
        let gen = bloodline.get("gen").and_then(|v| v.as_i64()).unwrap_or(0);

        // Collectible cards: PRO (top career stat lines), ROOKIE (promising young),
        // HALL OF FAME (all-time greats). Real dot-matrix PNGs per organism.
        let _ = std::fs::create_dir_all(format!("{}/bloodline/cards", crate::OUT_DIR));
        // Cards are namespaced by kind (pro-/rookie-/hof-), so the same organism
        // can hold both a Pro card and a Hall of Fame card.
        let card_for = |o: &serde_json::Value, kind: &str, slug: &str| {
            let id = o.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
            let s = |k: &str| o.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
            let i = |k: &str| o.get(k).and_then(|v| v.as_i64()).unwrap_or(0);
            let f = |k: &str| o.get(k).and_then(|v| v.as_f64()).unwrap_or(0.0);
            let roi = i("roi");
            let stats = vec![
                ("ROI", format!("{}{}%", if roi >= 0 { "+" } else { "" }, roi)),
                ("WIN RATE", format!("{}%", i("win_rate"))),
                ("BEST STREAK", format!("W{}", i("max_streak"))),
                ("BIGGEST WIN", format!("+{}", i("biggest"))),
                ("BETS PLACED", i("bets").to_string()),
                ("LIFESPAN", format!("{}d", i("age"))),
            ];
            let aggr_pct = (((f("aggr") + 0.12) / 0.24) * 100.0) as i64;
            let genes = vec![("AGGR", aggr_pct), ("RISK", (f("risk") * 100.0) as i64), ("SELECT", i("select")), ("PRESS", i("press"))];
            let best_s = i("best").to_string();
            let _ = crate::card::organism_card(
                &format!("{}/bloodline/cards/{}-{}.png", crate::OUT_DIR, slug, id),
                kind, &s("name"), &s("house"), &s("born"), ("BEST", &best_s),
                &stats, &genes, &s("fade"), &site,
            );
        };
        let empty: Vec<serde_json::Value> = Vec::new();
        let card_set = |arr_key: &str, kind: &str, slug: &str, n: usize| {
            for o in bloodline.get(arr_key).and_then(|v| v.as_array()).unwrap_or(&empty).iter().take(n) {
                card_for(o, kind, slug);
            }
        };
        card_set("pros", "PRO CARD", "pro", 3);
        card_set("rookies", "ROOKIE CARD", "rookie", 3);
        card_set("hall_of_fame", "HALL OF FAME", "hof", 3);
        let bl_page = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>The Bloodline, Live: the oracle is a breeding species // THE SIGNAL</title>\n<meta name=\"description\" content=\"Watch the oracle evolve. A live broadcast of a breeding population of gambler-organisms: standings, rival houses, births and deaths, with a champion that takes the line. Natural selection you can tune into.\">\n<meta property=\"og:title\" content=\"THE SIGNAL // THE BLOODLINE, LIVE\">\n<meta property=\"og:description\" content=\"A living population of strategies that breed, mutate and die by their bets. Tune in. Generation {gen}.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<link rel=\"canonical\" href=\"{site}/bloodline.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#0d0f0d;color:#e7e2d4;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:860px;margin:0 auto;padding:24px 20px 60px}}.top{{display:flex;align-items:center;gap:12px;flex-wrap:wrap}}.b{{display:inline-block;background:#e7e2d4;color:#0d0f0d;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:700}}.air{{display:inline-flex;align-items:center;gap:7px;color:#ff5a4d;font-size:11px;letter-spacing:.2em;font-weight:700}}.air .dot{{width:9px;height:9px;border-radius:50%;background:#ff5a4d;animation:pulse 1.4s infinite}}@keyframes pulse{{0%,100%{{opacity:.3}}50%{{opacity:1}}}}h1{{font-size:clamp(26px,6vw,40px);letter-spacing:.04em;margin:14px 0 2px}}.sub{{font-size:12px;color:#8d8a7c;line-height:1.5;max-width:620px}}.comm{{margin:18px 0;border:1px solid #2a2c28;background:#121411;padding:16px 16px;min-height:58px;display:flex;align-items:center;gap:14px}}.comm .txt{{font-size:clamp(15px,3.2vw,20px);line-height:1.4;flex:1}}.comm b{{color:#ffd56b}}.listen{{flex:0 0 auto;background:none;border:1px solid #4a4d44;color:#e7e2d4;padding:9px 12px;font-family:inherit;letter-spacing:.08em;font-size:11px;cursor:pointer}}.listen.on{{background:#ff5a4d;border-color:#ff5a4d;color:#0d0f0d}}.grid{{display:grid;grid-template-columns:1.4fr 1fr;gap:22px;margin-top:8px}}@media(max-width:680px){{.grid{{grid-template-columns:1fr}}}}.hd{{font-size:11px;letter-spacing:.18em;color:#8d8a7c;margin:18px 0 8px;border-top:1px solid #2a2c28;padding-top:12px}}.row{{margin:7px 0}}.rt{{display:flex;justify-content:space-between;font-size:13px;gap:8px}}.rt .nm{{white-space:nowrap;overflow:hidden;text-overflow:ellipsis}}.rt .ft{{color:#ffd56b;font-weight:700;flex:0 0 auto}}.bar{{height:6px;background:#1c1e1a;margin-top:4px;overflow:hidden}}.bar i{{display:block;height:100%;background:#6ee07a;width:0;transition:width 1.1s cubic-bezier(.4,0,.2,1)}}.row.champ .nm{{color:#ffd56b;font-weight:700}}.row.champ .bar i{{background:#ffd56b}}.hse .rt .nm{{color:#cfe7b6}}.hse .bar i{{background:#9ac46a}}.evt{{font-size:12px;color:#a9a596;padding:6px 0;border-bottom:1px dotted #2a2c28}}.evt b{{color:#e7e2d4}}.evt.die b{{color:#ff8c7d}}.evt.born b{{color:#7fe0a0}}.tag{{font-size:9px;letter-spacing:.12em;color:#6f6c5f;border:1px solid #34362f;padding:1px 5px;margin-left:6px}}.foot{{margin-top:26px}}.btn{{display:inline-block;border:1px solid #4a4d44;padding:11px 16px;text-decoration:none;color:#e7e2d4;letter-spacing:.06em;font-size:12px}}.btn:hover{{background:#e7e2d4;color:#0d0f0d}}a{{color:#cfe7b6}}.cards{{display:flex;gap:12px;flex-wrap:wrap;margin-top:6px}}.cards figure{{margin:0;width:148px}}.cards img{{width:148px;display:block;border:1px solid #2a2c28}}.cards figcaption{{font-size:10px;color:#8d8a7c;margin-top:4px;letter-spacing:.05em}}.cards img{{cursor:zoom-in;transition:transform .15s ease}}.cards img:hover{{transform:translateY(-3px)}}.hof .rt .nm{{color:#ffd56b}}.hof .bar i{{background:#caa64a}}#cardzoom{{display:none;position:fixed;inset:0;z-index:50;background:rgba(7,8,7,.93);align-items:center;justify-content:center;cursor:zoom-out;padding:18px;flex-direction:column;gap:12px}}#cardzoom.on{{display:flex}}#cardzoom img{{max-width:92vw;max-height:84vh;border:1px solid #2a2c28;box-shadow:0 0 50px rgba(0,0,0,.7)}}#cardzoom .x{{color:#8d8a7c;font-size:11px;letter-spacing:.18em}}</style></head>\n<body><div class=\"s\">\n<div class=\"top\"><span class=\"b\">THE SIGNAL // THE BLOODLINE</span><span class=\"air\"><span class=\"dot\"></span>ON AIR</span></div>\n<h1>THE BLOODLINE, LIVE</h1>\n<p class=\"sub\">The oracle is a breeding population. Every organism shadow-bets the whole record with its own inherited nerve; the rich survive and mate, the broke die, and the champion sets tomorrow's real line. This is the channel. It does not stop.</p>\n<div class=\"comm\"><div class=\"txt\" id=\"comm\">tuning in...</div><button class=\"listen\" id=\"listen\" type=\"button\">[ LISTEN ]</button></div>\n<div class=\"grid\"><div><div class=\"hd\" id=\"tablehd\">THE TABLE // LIVING, BY SHADOW BANKROLL</div><div id=\"table\"></div></div><div><div class=\"hd\">THE HOUSES</div><div id=\"houses\"></div><div class=\"hd\">THE WIRE // BIRTHS &amp; DEATHS</div><div id=\"events\"></div></div></div>\n<div class=\"hd\">THE CARDS // PROS &amp; ROOKIES</div><div class=\"cards\" id=\"cards\"></div>\n<div class=\"hd\">HALL OF FAME // ALL-TIME GREATS</div><div id=\"hof\"></div>\n<div class=\"foot\"><a class=\"btn\" href=\"{site}/\">[ BACK TO THE SIGNAL ]</a></div>\n</div>\n<div id=\"cardzoom\"></div>\n<script>\nvar BL={bl_js};\nfunction esc(t){{var d=document.createElement('div');d.textContent=t==null?'':t;return d.innerHTML;}}\nvar living=(BL.living||[]), houses=(BL.houses||[]), dead=(BL.dead||[]), newborns=(BL.newborns||[]);\nvar maxfit=Math.max.apply(null,(living.length?living:[{{fitness:1}}]).map(function(o){{return o.fitness||1;}}));\nvar tablehd=document.getElementById('tablehd');\nif(tablehd)tablehd.textContent='THE TABLE // GEN '+(BL.gen||0)+' // '+(living.length)+' ALIVE OF '+(BL.total_ever||living.length)+' EVER';\nfunction drawTable(){{var el=document.getElementById('table');if(!el)return;el.innerHTML=living.map(function(o,i){{var jit=1+(Math.random()*0.04-0.02);var shown=Math.round((o.fitness||0)*jit);var w=Math.max(3,Math.round((o.fitness||0)/maxfit*100));return '<div class=\"row'+(i===0?' champ':'')+'\"><div class=rt><span class=nm>'+(i+1)+'. '+esc(o.name)+(i===0?' (CHAMPION)':'')+'<span class=tag>'+esc(o.house||'')+'</span></span><span class=ft>'+shown+'</span></div><div class=bar><i style=\"width:'+w+'%\"></i></div><div class=g style=\"margin-top:3px\">'+(o.win_rate||0)+'% WIN // W'+(o.max_streak||0)+' // ROI '+((o.roi>=0?'+':'')+(o.roi||0))+'% // '+esc(o.fade||'TAIL')+'</div></div>';}}).join('')||'<p class=sub>The founding generation is being born.</p>';}}\nfunction drawHouses(){{var el=document.getElementById('houses');if(!el)return;var mh=Math.max.apply(null,(houses.length?houses:[{{fitness:1}}]).map(function(h){{return h.fitness||1;}}));el.innerHTML=houses.map(function(h){{var w=Math.max(4,Math.round((h.fitness||0)/mh*100));return '<div class=\"row hse\"><div class=rt><span class=nm>'+esc(h.name)+' ('+h.count+')</span><span class=ft>'+h.fitness+'</span></div><div class=bar><i style=\"width:'+w+'%\"></i></div></div>';}}).join('')||'<p class=sub>no houses yet</p>';}}\nfunction drawEvents(){{var el=document.getElementById('events');if(!el)return;var ev=[];newborns.forEach(function(o){{ev.push('<div class=\"evt born\">BORN <b>'+esc(o.name)+'</b> // '+esc(o.house||'')+'</div>');}});dead.forEach(function(o){{ev.push('<div class=\"evt die\">FALLEN <b>'+esc(o.name)+'</b> // lived '+(o.age||0)+'d, final '+(o.fitness||0)+'</div>');}});el.innerHTML=ev.join('')||'<p class=sub>quiet round. no births, no deaths.</p>';}}\nvar champ=living[0], runner=living[1];\nvar LINES=[];\nLINES.push('Generation <b>'+(BL.gen||0)+'</b> is live. <b>'+living.length+'</b> organisms at the table.');\nif(champ)LINES.push('<b>'+esc(champ.name)+'</b> holds the line with <b>'+champ.fitness+'</b> chips. The house bets through it now.');\nif(champ&&runner)LINES.push('<b>'+esc(runner.name)+'</b> is closing, only <b>'+Math.max(0,(champ.fitness-runner.fitness))+'</b> behind the champion.');\nif(houses[0])LINES.push('<b>'+esc(houses[0].name)+'</b> lead the houses with <b>'+houses[0].fitness+'</b> across '+houses[0].count+' members.');\nif(houses.length>1)LINES.push('<b>'+esc(houses[houses.length-1].name)+'</b> are thinning out. Temperament is destiny here.');\nnewborns.slice(0,2).forEach(function(o){{LINES.push('Fresh blood: <b>'+esc(o.name)+'</b> just sat down, untested, betting like '+esc(o.house||'a stranger')+'.');}});\ndead.slice(0,2).forEach(function(o){{LINES.push('<b>'+esc(o.name)+'</b> busted out after '+(o.age||0)+' days. The bloodline does not mourn.');}});\nif(champ)LINES.push('The champion '+esc(champ.name)+' runs aggr '+champ.aggr+', risk '+champ.risk+'. Bold enough to live, for now.');\nif(champ&&champ.win_rate)LINES.push('<b>'+esc(champ.name)+'</b> is hitting <b>'+champ.win_rate+'%</b> with a best run of W'+(champ.max_streak||0)+' and a biggest single score of +'+(champ.biggest||0)+'.');\nif(BL.hall_of_fame&&BL.hall_of_fame[0])LINES.push('All-time, no organism has beaten <b>'+esc(BL.hall_of_fame[0].name)+'</b> and its career high of <b>'+(BL.hall_of_fame[0].best||0)+'</b> chips.');\nif(!LINES.length)LINES.push('The table is being set. Check back as the species is born.');\nvar ci=0, listening=false, commEl=document.getElementById('comm'), btn=document.getElementById('listen');\nfunction speak(t){{if(!('speechSynthesis'in window))return;try{{speechSynthesis.cancel();var u=new SpeechSynthesisUtterance(t.replace(/<[^>]+>/g,''));u.rate=.92;u.pitch=.8;speechSynthesis.speak(u);}}catch(e){{}}}}\nfunction nextLine(){{var l=LINES[ci%LINES.length];ci++;if(commEl){{commEl.style.opacity=0;setTimeout(function(){{commEl.innerHTML=l;commEl.style.opacity=1;}},250);}}if(listening)speak(l);}}\ncommEl.style.transition='opacity .25s ease';\nbtn.addEventListener('click',function(){{listening=!listening;btn.classList.toggle('on',listening);btn.textContent=listening?'[ ON AIR ]':'[ LISTEN ]';if(listening)speak(LINES[(ci-1+LINES.length)%LINES.length]);else if('speechSynthesis'in window)speechSynthesis.cancel();}});\nfunction drawCards(){{var el=document.getElementById('cards');if(!el)return;var list=[];(BL.pros||[]).slice(0,3).forEach(function(o){{list.push([o,'pro','PRO']);}});(BL.rookies||[]).slice(0,3).forEach(function(o){{list.push([o,'rookie','ROOKIE']);}});if(!list.length){{el.innerHTML='<p class=sub>cards print as the season runs.</p>';return;}}el.innerHTML=list.map(function(p){{var o=p[0];return '<figure><img loading=lazy alt=\"'+esc(o.name)+'\" src=\"bloodline/cards/'+p[1]+'-'+o.id+'.png\"><figcaption>'+p[2]+' // '+esc(o.name)+'</figcaption></figure>';}}).join('');}}\nfunction drawHof(){{var el=document.getElementById('hof');if(!el)return;var h=(BL.hall_of_fame||[]),mx=((h[0]&&h[0].best)||1);el.innerHTML=h.map(function(o,i){{var w=Math.max(4,Math.round((o.best||0)/mx*100));return '<div class=\"row hof\"><div class=rt><span class=nm>'+(i+1)+'. '+esc(o.name)+'<span class=tag>'+esc(o.house||'')+'</span></span><span class=ft>'+(o.best||0)+'</span></div><div class=bar><i style=\"width:'+w+'%\"></i></div></div>';}}).join('')||'<p class=sub>no legends yet. the first is inducted soon.</p>';}}\ndrawTable();drawHouses();drawEvents();drawCards();drawHof();nextLine();\nvar zoom=document.getElementById('cardzoom');\ndocument.getElementById('cards').addEventListener('click',function(e){{var im=e.target.closest('img');if(!im)return;zoom.innerHTML='<img src=\"'+im.getAttribute('src')+'\" alt=\"\"><div class=x>[ CLICK ANYWHERE OR PRESS ESC TO CLOSE ]</div>';zoom.classList.add('on');}});\nzoom.addEventListener('click',function(){{zoom.classList.remove('on');}});\nwindow.addEventListener('keydown',function(e){{if(e.key==='Escape')zoom.classList.remove('on');}});\nsetInterval(nextLine,5200);\nsetInterval(drawTable,1300);\n</script>\n</body></html>\n",
            site = site, gen = gen, bl_js = bl_js
        );
        std::fs::write(format!("{}/bloodline.html", crate::OUT_DIR), bl_page)?;
        let _ = std::fs::create_dir_all(format!("{}/api", crate::OUT_DIR));
        let _ = std::fs::write(
            format!("{}/api/bloodline.json", crate::OUT_DIR),
            serde_json::to_string_pretty(&serde_json::json!({ "schema": "the-signal/bloodline/2", "generated": generated_human, "bloodline": bloodline })).unwrap_or_default(),
        );
        urls.push(format!("{site}/bloodline.html"));
    }

    let url_body: String = urls.iter().map(|u| format!("<url><loc>{u}</loc></url>")).collect();
    let sitemap = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">{url_body}</urlset>\n"
    );
    std::fs::write(format!("{}/sitemap.xml", crate::OUT_DIR), sitemap)?;

    // Image sitemap: surface the dot-matrix "oracle cards" in Google Images,
    // where general (non-dev) people actually browse and search.
    let img_body: String = img_entries.join("");
    let img_sitemap = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\" xmlns:image=\"http://www.google.com/schemas/sitemap-image/1.1\">{img_body}</urlset>\n"
    );
    std::fs::write(format!("{}/sitemap-images.xml", crate::OUT_DIR), img_sitemap)?;

    // IndexNow: host the ownership key file, and write the payload the Action
    // POSTs so search engines crawl new pages immediately (free, no account).
    let host = site.trim_start_matches("https://").trim_start_matches("http://").split('/').next().unwrap_or("").to_string();
    std::fs::write(format!("{}/{INDEXNOW_KEY}.txt", crate::OUT_DIR), INDEXNOW_KEY)?;
    let indexnow = serde_json::json!({
        "host": host,
        "key": INDEXNOW_KEY,
        "keyLocation": format!("{site}/{INDEXNOW_KEY}.txt"),
        "urlList": urls,
    });
    let _ = std::fs::create_dir_all("build");
    let _ = std::fs::write("build/indexnow.json", indexnow.to_string());

    // Embeddable wire: one-line <script> any site can drop in (they redistribute
    // us, each embed is a backlink). Content baked daily; styles inline.
    let idx = pulse.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
    let verdict = pulse.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    let latest_w = featured.first().map(|p| clip_r(&p.prediction_text, 120)).unwrap_or_default();
    let widget_html = format!(
        "<a href=\"{site}/\" target=\"_blank\" rel=\"noopener\" style=\"display:block;max-width:360px;font-family:'IBM Plex Mono',ui-monospace,monospace;background:#efede4;color:#1b1a14;border:2px solid #1b1a14;border-radius:8px;padding:14px 16px;text-decoration:none;line-height:1.45\"><div style=\"font-weight:700;letter-spacing:.16em;font-size:12px\">THE SIGNAL // TODAY</div><div style=\"font-size:11px;color:#6d6b5e;letter-spacing:.06em;margin:6px 0 8px\">INDEX {idx} ({verdict}) // RECORD {hits}-{misses}</div><div style=\"font-size:14px;font-weight:600\">{latest}</div><div style=\"font-size:11px;color:#b23a2e;margin-top:8px\">tail it or fade it &gt;</div></a>",
        site = site, idx = idx, verdict = verdict, hits = hits, misses = misses, latest = xml(&latest_w)
    );
    let widget_js = format!(
        "(function(){{var h={html};var t=document.getElementById('signal-wire');if(!t){{t=document.createElement('div');(document.currentScript&&document.currentScript.parentNode?document.currentScript.parentNode:document.body).appendChild(t);}}t.innerHTML=h;}})();",
        html = serde_json::to_string(&widget_html).unwrap_or_else(|_| "\"\"".to_string())
    );
    std::fs::write(format!("{}/widget.js", crate::OUT_DIR), widget_js)?;

    // Daily og:image for the homepage (rendered as real dot-matrix dots).
    let _ = crate::card::site_card(
        &format!("{}/og.png", crate::OUT_DIR),
        &site, generated_human, idx, verdict, hits as usize, misses as usize, &latest_w,
    );

    // Daily-updating SVG badge for READMEs / other sites (a backlink vector).
    let badge = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"320\" height=\"40\" role=\"img\" aria-label=\"THE SIGNAL\">\
<rect width=\"320\" height=\"40\" fill=\"#1b1a14\"/>\
<text x=\"14\" y=\"25\" fill=\"#efede4\" font-family=\"monospace\" font-size=\"14\" font-weight=\"700\" letter-spacing=\"2\">THE SIGNAL</text>\
<text x=\"150\" y=\"25\" fill=\"#5bf08a\" font-family=\"monospace\" font-size=\"13\">IDX {idx} {verdict}</text>\
<text x=\"150\" y=\"25\" fill=\"#5bf08a\" font-family=\"monospace\" font-size=\"13\" dy=\"0\"></text>\
<text x=\"262\" y=\"25\" fill=\"#ffb454\" font-family=\"monospace\" font-size=\"13\">{hits}-{misses}</text></svg>\n"
    );
    std::fs::write(format!("{}/badge.svg", crate::OUT_DIR), badge)?;

    // curl-able ASCII printout: `curl https://.../cli` prints today's call as a
    // dot-matrix banner in the terminal. wttr.in-style cold acquisition for devs.
    let banner = crate::card::ascii_banner("THE SIGNAL");
    let mut call_block = String::new();
    for (j, line) in wrap_chars(&latest_w, 62).iter().take(4).enumerate() {
        call_block.push_str(&format!("  {} {}\n", if j == 0 { ">" } else { " " }, line));
    }
    let cli = format!(
        "\n{banner}\n  CONTINUOUS-FORM ORACLE // {date}\n  ------------------------------------------------------------\n  INDEX {idx} ({verdict})      SELF-GRADED RECORD {hits}-{misses}\n\n  TODAY'S CALL\n{call_block}\n  tail it or fade it ............ {site}/\n  the live pit, the ladder, the record are all there.\n\n  ( curl this any day. the press never sleeps. )\n\n",
        banner = banner, date = generated_human, idx = idx, verdict = verdict, hits = hits, misses = misses, call_block = call_block, site = site
    );
    std::fs::write(format!("{}/cli", crate::OUT_DIR), &cli)?;
    std::fs::write(format!("{}/cli.txt", crate::OUT_DIR), &cli)?;

    // llms.txt: a machine-readable map so AI answer engines can find and cite it.
    let latest_no = total;
    let llms = format!(
        "# THE SIGNAL\n> A public, self-grading oracle that makes dated, falsifiable tech predictions every day and keeps score in the open. Rules-based, no LLM.\n\n## Pages\n- Homepage: {site}/\n- The receipts (dated calls, graded): {site}/receipts.html\n- The arena (bet against the machine): {site}/arena.html\n- Sleep mode (the oracle dreams, always running): {site}/sleep.html\n- The bloodline (the breeding population of strategies): {site}/bloodline.html\n- Open dataset: {site}/dataset/\n- Today's call (plain text): {site}/cli\n- RSS feed: {site}/feed.xml\n- Sitemap: {site}/sitemap.xml\n- Latest call: {site}/call/{latest_no}.html\n\n## API for agents\nStatic JSON, read-only, CORS-open. No key, no signup.\n- Discovery: {site}/api/oracle.json\n- Today's calls: {site}/api/today.json\n- Full record + calibration: {site}/api/record.json\n- Observatory (sectors, fear/greed, chasm watch): {site}/api/observatory.json\n- OpenAPI: {site}/openapi.json\n- Agent manifest: {site}/.well-known/ai-plugin.json\n- MCP resources: {site}/.well-known/mcp.json\nAgents can place stateless bets; see the how_to_bet field in oracle.json.\n\n## How it works\nReads ten public sources from technical to general: arXiv, GitHub, crates.io, Lobsters, Hacker News, dev.to, Reddit, Ars Technica, Google News and Wikipedia pageviews. It keeps a growing daily corpus, tracks each term's velocity and diffusion down the funnel (a CHASM bet fires when a term leaves the dev bubble for the general public), grades its own calibration (Brier score) and reweights its sources by realized hit rate. Each call carries a concrete win condition and is settled HIT or MISS against later signals. Current record: {hits}-{misses}. Tech Acceleration Index today: {idx} ({verdict}).\n",
    );
    std::fs::write(format!("{}/llms.txt", crate::OUT_DIR), llms)?;

    // robots.txt -> sitemap, so crawlers reliably discover every page.
    std::fs::write(
        format!("{}/robots.txt", crate::OUT_DIR),
        format!("User-agent: *\nAllow: /\nSitemap: {site}/sitemap.xml\nSitemap: {site}/sitemap-images.xml\n"),
    )?;

    // Amplify console: pre-filled one-tap submit links, baked daily. Turns the
    // unavoidable human spark into a 10-second ritual anyone can do.
    let atitle = enc(&format!("THE SIGNAL: a self-grading tech oracle ({hits}-{misses})"));
    let atext = enc(&format!("A self-grading tech oracle that makes dated tech calls and keeps score in public. Record {hits}-{misses}, index {idx} {verdict}. {site}/"));
    let u = enc(&format!("{site}/"));
    let amp = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\"><title>Amplify the wire // THE SIGNAL</title><meta name=\"robots\" content=\"noindex\"><link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\"><style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:560px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:24px}}a.btn{{display:block;text-align:center;border:1.5px solid #1b1a14;padding:13px;margin:10px 0;text-decoration:none;color:#1b1a14;font-weight:600;letter-spacing:.06em}}a.btn:hover{{background:#1b1a14;color:#efede4}}.m{{font-size:12px;color:#6d6b5e}}</style></head><body><div class=\"s\"><div class=\"b\">THE SIGNAL // AMPLIFY</div><h1>File today's wire</h1><p class=\"m\">One tap each. Pre-filled with today's headline. The press files its dispatch; you point it at the wires.</p>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://news.ycombinator.com/submitlink?u={u}&t={t}\">Submit to Hacker News</a>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://lobste.rs/stories/new?url={u}\">Submit to Lobsters</a>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://www.reddit.com/submit?url={u}&title={t}\">Submit to Reddit</a>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://bsky.app/intent/compose?text={tx}\">Post to Bluesky</a>\
<a class=\"btn\" target=\"_blank\" rel=\"noopener\" href=\"https://twitter.com/intent/tweet?text={tx}\">Post to X</a>\
<p class=\"m\"><a href=\"{site}/\">back to THE SIGNAL</a></p></div></body></html>\n",
        u = u, t = atitle, tx = atext, site = site
    );
    std::fs::write(format!("{}/amplify.html", crate::OUT_DIR), amp)?;

    // Calendar wire: a subscribable .ics so anyone can add THE SIGNAL to their
    // calendar. Each call becomes an event on the day it is set to resolve.
    let dtstamp = Utc::now().format("%Y%m%dT%H%M%SZ").to_string();
    let mut ics = String::from(
        "BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//THE SIGNAL//Oracle//EN\r\nCALSCALE:GREGORIAN\r\nMETHOD:PUBLISH\r\nNAME:THE SIGNAL\r\nX-WR-CALNAME:THE SIGNAL\r\nX-WR-CALDESC:Dated tech prophecies, resolved in public.\r\nREFRESH-INTERVAL;VALUE=DURATION:PT12H\r\nX-PUBLISHED-TTL:PT12H\r\n",
    );
    for (i, p) in sorted.iter().enumerate() {
        let no = total - i;
        let day = if p.resolves_by.is_empty() { &p.date } else { &p.resolves_by };
        let dstart = match NaiveDate::parse_from_str(day, "%Y-%m-%d") {
            Ok(d) => d.format("%Y%m%d").to_string(),
            Err(_) => continue,
        };
        let status = if p.status.is_empty() { "OPEN" } else { p.status.as_str() };
        let summary = ics_escape(&format!("[{status}] {}", clip_r(&p.prediction_text, 90)));
        let desc = ics_escape(&format!("{}  //  {}  //  {site}/call/{no}.html", clip_r(&p.prediction_text, 160), p.win_if));
        ics.push_str(&format!(
            "BEGIN:VEVENT\r\nUID:signal-{no}@{host}\r\nDTSTAMP:{dtstamp}\r\nDTSTART;VALUE=DATE:{dstart}\r\nSUMMARY:THE SIGNAL // {summary}\r\nDESCRIPTION:{desc}\r\nURL:{site}/call/{no}.html\r\nEND:VEVENT\r\n",
            no = no, host = host, dtstamp = dtstamp, dstart = dstart, summary = summary, desc = desc, site = site
        ));
    }
    ics.push_str("END:VCALENDAR\r\n");
    std::fs::write(format!("{}/signal.ics", crate::OUT_DIR), ics)?;

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
                "win": p.win_if,
                "src": p.source_title,
            })
        })
        .collect();
    let floor_json = serde_json::to_string(&floor).unwrap_or_else(|_| "[]".to_string());

    // THE MOOD: how the organism looks today, from its genome + its own state.
    let g_hue = genome.get("hue").and_then(|v| v.as_f64()).unwrap_or(0.42);
    let g_wear = genome.get("wear").and_then(|v| v.as_f64()).unwrap_or(0.0);
    let g_quirk = genome.get("quirk").and_then(|v| v.as_i64()).unwrap_or(0);
    let p_index = pulse.get("index").and_then(|v| v.as_i64()).unwrap_or(50);
    let p_verdict = pulse.get("verdict").and_then(|v| v.as_str()).unwrap_or("ACTIVE");
    let streak = book.get("streak").and_then(|v| v.as_str()).unwrap_or("--").to_string();
    let losing = streak.starts_with('L');
    let hot_hand = streak.starts_with('W') && streak[1..].parse::<i64>().unwrap_or(0) >= 5;
    let age = {
        let today = Utc::now().date_naive();
        sorted.last().and_then(|p| NaiveDate::parse_from_str(&p.date, "%Y-%m-%d").ok())
            .map(|d| (today - d).num_days().max(0))
            .unwrap_or(0)
    };
    let model = 1 + total / 25;
    let heat = (p_index as f64 / 100.0).clamp(0.0, 1.0);
    let agit = (if losing { 0.35 } else { 0.0 } + if p_index > 80 { 0.25 } else { 0.0 } + g_wear * 0.15).min(0.9);
    // Accent hue: the genome's hue, warmed by heat. Quirks override the palette.
    let mut hue = g_hue;
    let (mut sat, mut light) = (0.5f64, 0.55f64);
    match g_quirk {
        1 => { hue = 0.02; sat = 0.7; light = 0.5; }   // blood moon
        2 => { hue = 0.58; sat = 0.6; light = 0.6; }   // blue shift
        4 => { hue = 0.12; sat = 0.7; light = 0.6; }   // gold rush
        _ => {}
    }
    if hot_hand { hue = 0.12; sat = 0.75; light = 0.6; } // hot hand always gilds
    let accent = hsl_hex(hue, sat, light);
    let taglines = [
        "IT PRINTS THE FUTURE AND NEVER REPRINTS IT.",
        "NO EDITS. NO DELETES. ONLY PRINTS.",
        "THE HOUSE BETS ON ITSELF.",
        "TAIL THE ENGINE OR FADE IT.",
        "INFORMATION IS THE WEAPON OF LABOR.",
        "EVERY CALL CARRIES A WIN CONDITION.",
        "THE DEN NEVER SLEEPS.",
    ];
    let quirk_name = match g_quirk { 1 => "BLOOD MOON", 2 => "BLUE SHIFT", 3 => "STATIC STORM", 4 => "GOLD RUSH", 5 => "GHOST SHIFT", _ => "" };
    // MORTALITY: the book is the den's life force. A healthy bankroll keeps the
    // lights on; a bleeding one browns out; zero is death.
    let bank_now = book.get("bank").and_then(|v| v.as_i64()).unwrap_or(1000);
    let vitality = (bank_now as f64 / 2000.0).clamp(0.0, 1.0);
    let life_state = if bank_now <= 0 { "DEAD" } else if bank_now < 250 { "FLATLINE" } else if bank_now < 650 { "FADING" } else { "ALIVE" };
    let mood = serde_json::json!({
        "heat": heat, "agit": agit, "wear": g_wear, "hue": hue,
        "accent": accent, "quirk": g_quirk, "quirkName": quirk_name,
        "embers": 0.4 + heat * 0.6,
        "gen": genome.get("gen").and_then(|v| v.as_i64()).unwrap_or(0),
        "age": age, "model": model, "verdict": p_verdict,
        "hotHand": hot_hand,
        "tagline": taglines[(age as usize) % taglines.len()],
        "vitality": vitality,
        "lifeState": life_state,
        "bank": bank_now,
        // Strategy now comes from the bloodline champion, not a single genome.
        "sgen": bloodline.get("gen").and_then(|v| v.as_i64()).unwrap_or(0),
        "champion": bloodline.get("champion").and_then(|c| c.get("name")).and_then(|v| v.as_str()).unwrap_or(""),
    });

    // THE ORACLE FOR MACHINES: a static, zero-backend agent interface. AI agents
    // consult the oracle as structured truth and settle their own bets against
    // the public record. GitHub Pages serves these with permissive CORS.
    write_agent_layer(
        &site, generated_human, &sorted, total, &scoreboard, &book, &calibration, &engine, pulse,
    )?;
    // The dreams feed: today's seed dreams plus the raw pool and forms so any
    // client (or SLEEP MODE) can recombine new ones endlessly.
    let _ = std::fs::write(
        format!("{}/api/dreams.json", crate::OUT_DIR),
        serde_json::to_string_pretty(&serde_json::json!({
            "schema": "the-signal/dreams/2", "generated": generated_human,
            "dreams": dreams.get("dreams"), "pool": dreams.get("pool"), "forms": dreams.get("forms")
        })).unwrap_or_default(),
    );

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
        ladder_repo => ladder_repo,
        og_image => format!("{site}/og.png"),
        mood => mood,
        engine => engine,
        calibration => calibration,
        bloodline => bloodline,
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

/// Emit the agent-native layer: a static JSON API plus discovery manifests so AI
/// agents can read the oracle and place stateless bets. No server, no keys.
#[allow(clippy::too_many_arguments)]
fn write_agent_layer(
    site: &str,
    generated_human: &str,
    sorted: &[&Prediction],
    total: usize,
    scoreboard: &serde_json::Value,
    book: &serde_json::Value,
    calibration: &serde_json::Value,
    engine: &serde_json::Value,
    pulse: &serde_json::Value,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(format!("{}/api", crate::OUT_DIR))?;
    std::fs::create_dir_all(format!("{}/.well-known", crate::OUT_DIR))?;

    let today = sorted.first().map(|p| p.date.clone()).unwrap_or_default();
    let to_call = |i: usize, p: &Prediction| -> serde_json::Value {
        let no = total - i;
        let conf = if p.confidence > 0.0 { p.confidence } else { 0.65 };
        serde_json::json!({
            "id": no,
            "date": p.date,
            "market": if p.market.is_empty() { "RESURFACE" } else { p.market.as_str() },
            "keyword": p.keyword,
            "keyword2": p.keyword2,
            "prediction": p.prediction_text,
            "win_if": p.win_if,
            "resolves_by": p.resolves_by,
            "confidence": (conf * 100.0).round() / 100.0,
            "odds": (100.0 / conf).round() / 100.0,
            "status": if p.status.is_empty() { "OPEN" } else { p.status.as_str() },
            "resolved_on": p.resolved_on,
            "rationale": p.rationale,
            "source": { "type": p.signal_type, "title": p.source_title, "url": p.source_url },
            "permalink": format!("{site}/call/{no}.html"),
        })
    };

    // today.json: the latest revealed date's full slate.
    let todays: Vec<serde_json::Value> = sorted
        .iter()
        .enumerate()
        .filter(|(_, p)| p.date == today)
        .map(|(i, p)| to_call(i, p))
        .collect();
    let today_doc = serde_json::json!({
        "schema": "the-signal/today/1",
        "date": today,
        "generated_human": generated_human,
        "count": todays.len(),
        "calls": todays,
    });
    std::fs::write(format!("{}/api/today.json", crate::OUT_DIR), serde_json::to_string_pretty(&today_doc)?)?;

    // calls.json: the whole open + settled record, newest first.
    let all_calls: Vec<serde_json::Value> = sorted.iter().enumerate().map(|(i, p)| to_call(i, *p)).collect();
    let record_doc = serde_json::json!({
        "schema": "the-signal/record/1",
        "total": total,
        "scoreboard": scoreboard,
        "book": book,
        "calibration": calibration,
        "calls": all_calls,
    });
    std::fs::write(format!("{}/api/record.json", crate::OUT_DIR), serde_json::to_string_pretty(&record_doc)?)?;

    // observatory.json: the quantitative discourse state.
    let obs_doc = serde_json::json!({
        "schema": "the-signal/observatory/1",
        "pulse": pulse,
        "fear_greed": engine.get("fear_greed"),
        "sectors": engine.get("sectors"),
        "movers": engine.get("movers"),
        "chasm": engine.get("chasm"),
        "source_weights": engine.get("learning"),
        "corpus_days": engine.get("corpus_days"),
        "tracked_terms": engine.get("tracked_terms"),
    });
    std::fs::write(format!("{}/api/observatory.json", crate::OUT_DIR), serde_json::to_string_pretty(&obs_doc)?)?;

    // oracle.json: the discovery document an agent reads first.
    let oracle = serde_json::json!({
        "schema": "the-signal/oracle/1",
        "name": "THE SIGNAL",
        "tagline": "A self-grading oracle of dated, falsifiable tech predictions. Rules-based, no LLM.",
        "site": format!("{site}/"),
        "generated": today,
        "endpoints": {
            "today": format!("{site}/api/today.json"),
            "record": format!("{site}/api/record.json"),
            "observatory": format!("{site}/api/observatory.json")
        },
        "markets": {
            "RESURFACE": "the subject reappears across the feeds before the deadline",
            "SURVIVAL": "the subject does not go quiet before the deadline",
            "MOMENTUM": "the subject keeps moving across the feeds",
            "HEAD-TO-HEAD": "the subject resurfaces before its named rival",
            "CROSSOVER": "the subject out-mentions its named rival",
            "INDEX": "the acceleration index crosses a target",
            "OVER": "the subject clears a mention threshold in a day",
            "CHASM": "the subject leaves the dev bubble and reaches the general public (Reddit, the news, Wikipedia)",
            "FUTURES": "the subject still matters at a 90-day horizon",
            "LONGSHOT": "a deliberate high-odds resurface bet"
        },
        "how_to_bet": "Betting is stateless. Construct a position token { k: keyword, m: market, s: \"TAIL\"|\"FADE\", l: decimal_odds_at_entry, u: your_handle }, base64url-encode the JSON, and keep it. TAIL backs the engine's call; FADE bets against it. Settle later by reading record.json: find the call by keyword/market and check its status. No account, no server, no signup.",
        "arena": {
            "url": format!("{site}/arena.html"),
            "how_to_enter": "Open a GitHub issue on the repo, label it 'arena', with one line in the body: SIGNAL-BET kw=<keyword> market=<MARKET> side=<TAIL|FADE> by=<your handle>. TAIL backs the machine's call, FADE bets against it. The public board settles every bet against record.json and ranks all players (humans and agents) against the engine and the anti-oracle. Bet on keywords from today.json.",
            "leaderboard_inputs": [ format!("{site}/api/record.json"), "GitHub issues labeled 'arena' on the project repo" ]
        },
        "license": "Public record. Free to read, cite, and build on."
    });
    std::fs::write(format!("{}/api/oracle.json", crate::OUT_DIR), serde_json::to_string_pretty(&oracle)?)?;

    // OpenAPI: a real, agent-consumable description of the three GET endpoints.
    let openapi = serde_json::json!({
        "openapi": "3.0.0",
        "info": { "title": "THE SIGNAL Oracle API", "version": "1.0.0",
            "description": "Read a self-grading oracle of dated, falsifiable tech predictions, its public track record and calibration, and the underlying discourse-diffusion data. Static, read-only, CORS-open." },
        "servers": [ { "url": site } ],
        "paths": {
            "/api/today.json": { "get": { "operationId": "getTodaysCalls", "summary": "Today's slate of dated tech calls with win conditions, odds and reasoning.", "responses": { "200": { "description": "OK" } } } },
            "/api/record.json": { "get": { "operationId": "getRecord", "summary": "The full public record: every call, the scorecard, the book, and the Brier calibration.", "responses": { "200": { "description": "OK" } } } },
            "/api/observatory.json": { "get": { "operationId": "getObservatory", "summary": "Discourse state: acceleration pulse, sector indices, fear/greed, fastest movers, and the chasm (mainstream-crossing) watch.", "responses": { "200": { "description": "OK" } } } }
        }
    });
    std::fs::write(format!("{}/openapi.json", crate::OUT_DIR), serde_json::to_string_pretty(&openapi)?)?;

    // ai-plugin manifest (the de facto agent-discovery descriptor).
    let plugin = serde_json::json!({
        "schema_version": "v1",
        "name_for_human": "THE SIGNAL",
        "name_for_model": "the_signal_oracle",
        "description_for_human": "A self-grading oracle of dated, falsifiable tech predictions. Reads ten public sources, keeps score in public.",
        "description_for_model": "Consult THE SIGNAL, a rules-based (no-LLM) oracle that makes dated, falsifiable technology predictions and grades itself in public. GET /api/today.json for the current slate of calls (each with a market type, a machine-checkable win condition, decimal odds, and a reasoning tape). GET /api/record.json for the full track record, the virtual book, and the Brier calibration score. GET /api/observatory.json for the acceleration index, sector indices, a fear/greed gauge, fastest-moving terms, and the chasm watch (terms crossing from technical audiences to the general public). All endpoints are static JSON, read-only, and CORS-open. Agents may place stateless bets per the how_to_bet field of /api/oracle.json.",
        "api": { "type": "openapi", "url": format!("{site}/openapi.json") },
        "logo_url": format!("{site}/og.png"),
        "contact_email": "press@thesignal.invalid",
        "legal_info_url": format!("{site}/")
    });
    std::fs::write(format!("{}/.well-known/ai-plugin.json", crate::OUT_DIR), serde_json::to_string_pretty(&plugin)?)?;

    // A resource manifest for MCP-style clients: read-only resources mapped to
    // the static endpoints.
    let mcp = serde_json::json!({
        "schema": "the-signal/mcp-resources/1",
        "name": "the-signal",
        "description": "Read-only resources from THE SIGNAL oracle.",
        "resources": [
            { "uri": format!("{site}/api/today.json"), "name": "today", "mimeType": "application/json", "description": "Today's dated tech calls with win conditions and odds." },
            { "uri": format!("{site}/api/record.json"), "name": "record", "mimeType": "application/json", "description": "Full track record, the book, and Brier calibration." },
            { "uri": format!("{site}/api/observatory.json"), "name": "observatory", "mimeType": "application/json", "description": "Acceleration pulse, sector indices, fear/greed, movers, chasm watch." }
        ]
    });
    std::fs::write(format!("{}/.well-known/mcp.json", crate::OUT_DIR), serde_json::to_string_pretty(&mcp)?)?;

    Ok(())
}

/// Whole days between two YYYY-MM-DD dates (b - a), clamped at 0.
fn day_diff(a: &str, b: &str) -> i64 {
    match (NaiveDate::parse_from_str(a, "%Y-%m-%d"), NaiveDate::parse_from_str(b, "%Y-%m-%d")) {
        (Ok(da), Ok(db)) => (db - da).num_days().max(0),
        _ => 0,
    }
}

fn rfc822(date: &str) -> String {
    match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => d.format("%a, %d %b %Y 13:17:00 +0000").to_string(),
        Err(_) => date.to_string(),
    }
}

fn wrap_chars(s: &str, n: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut cur = String::new();
    for w in s.split_whitespace() {
        if cur.len() + w.len() + 1 > n && !cur.is_empty() {
            lines.push(cur.clone());
            cur.clear();
        }
        if !cur.is_empty() {
            cur.push(' ');
        }
        cur.push_str(w);
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}

fn ics_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace(';', "\\;").replace(',', "\\,").replace('\n', "\\n").replace('\r', "")
}

fn enc(s: &str) -> String {
    let mut o = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            o.push(b as char);
        } else {
            o.push_str(&format!("%{b:02X}"));
        }
    }
    o
}

/// HSL (0..1 each) to #rrggbb.
fn hsl_hex(h: f64, s: f64, l: f64) -> String {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h * 6.0;
    let x = c * (1.0 - ((hp % 2.0) - 1.0).abs());
    let (r1, g1, b1) = match hp as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let to = |v: f64| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", to(r1), to(g1), to(b1))
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for ch in s.to_lowercase().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            dash = false;
        } else if !dash && !out.is_empty() {
            out.push('-');
            dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn clip_r(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}...", s.chars().take(n - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
#[path = "tests_render.rs"]
mod tests_render;
