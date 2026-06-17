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
        let page = format!(
            "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>Call No. {no}: {tt} // THE SIGNAL</title>\n<meta name=\"description\" content=\"{desc}\">\n<meta property=\"og:title\" content=\"THE SIGNAL // Call No. {no} [{status}]\">\n<meta property=\"og:description\" content=\"{desc}\">\n<meta name=\"twitter:card\" content=\"summary_large_image\">\n<meta property=\"og:image\" content=\"{site}/call/{no}.png\">\n<meta name=\"twitter:image\" content=\"{site}/call/{no}.png\">\n<link rel=\"canonical\" href=\"{site}/call/{no}.html\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:620px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}.c{{font-size:25px;font-weight:600;line-height:1.35;margin:18px 0}}.m{{font-size:11px;letter-spacing:.1em;color:#6d6b5e}}.w{{font-size:12px;color:#6d6b5e;margin:14px 0}}a{{color:#1b1a14}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // CALL No. {no}</div>\n<div class=\"m\">{date} // {market} // {status}</div>\n<p class=\"c\">{t}</p>\n<div class=\"w\">{win}</div>\n<p class=\"m\"><a href=\"{src}\" rel=\"noopener\">source signal</a> // <a href=\"{site}/#call-{no}\">on the public record</a> // <a href=\"{site}/\">THE SIGNAL</a></p>\n</div></body></html>\n",
            no = no, tt = tt, t = xml(&p.prediction_text), desc = desc, status = status, market = market,
            date = xml(&p.date), win = xml(&p.win_if), src = xml(&p.source_url), site = site
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
        "# THE SIGNAL\n> A public, self-grading oracle that makes dated, falsifiable tech predictions every day and keeps score in the open. Rules-based, no LLM.\n\n## Pages\n- Homepage: {site}/\n- Today's call (plain text): {site}/cli\n- RSS feed: {site}/feed.xml\n- Sitemap: {site}/sitemap.xml\n- Latest call: {site}/call/{latest_no}.html\n\n## How it works\nReads Hacker News, arXiv, GitHub, Lobsters, dev.to and Ars Technica. Each call carries a concrete win condition and is settled HIT or MISS against later signals. Current record: {hits}-{misses}. Tech Acceleration Index today: {idx} ({verdict}).\n",
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
    let mood = serde_json::json!({
        "heat": heat, "agit": agit, "wear": g_wear, "hue": hue,
        "accent": accent, "quirk": g_quirk, "quirkName": quirk_name,
        "embers": 0.4 + heat * 0.6,
        "gen": genome.get("gen").and_then(|v| v.as_i64()).unwrap_or(0),
        "age": age, "model": model, "verdict": p_verdict,
        "hotHand": hot_hand,
        "tagline": taglines[(age as usize) % taglines.len()],
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
        jsonld => jsonld,
        floor_json => floor_json,
        ladder_repo => ladder_repo,
        og_image => format!("{site}/og.png"),
        mood => mood,
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
