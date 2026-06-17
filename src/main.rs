mod access;
mod bench;
mod bloodline;
mod card;
mod fetch;
mod generate;
mod manifold;
mod model;
mod observatory;
mod rank;
mod render;

use chrono::{Datelike, Duration, NaiveDate, Utc};
use model::Prediction;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

// Committed, public: the revealed track record. Renders the public page.
const DATA_PATH: &str = "data/predictions.json";
// Committed, public: the daily acceleration index history (THE PULSE).
const PULSE_PATH: &str = "data/pulse.json";
// Committed, public: the organism's genome -- mutates once per day, never resets.
const GENOME_PATH: &str = "data/genome.json";
// Committed, public: learned per-source weights (the engine improving itself).
const WEIGHTS_PATH: &str = "data/weights.json";
// Gitignored, synced from KV before the run: calls still under embargo.
const EMBARGO_IN: &str = "build/embargoed_in.json";
// Gitignored, synced to KV after the run: the subscriber edge payload.
const EARLY_OUT: &str = "build/early_payload.json";

pub const OUT_DIR: &str = "docs";
pub const OUT_HTML: &str = "docs/index.html";

fn main() {
    // The release window. Subscribers see a call immediately; the public page
    // reveals it `delay_days` later. 0 = no embargo (acts like a free blog).
    // 0 = no embargo: today's call is public today. This is the reliable default
    // and needs no external state. A positive delay only works when the embargoed
    // pool is persisted across runs (Cloudflare KV); without that infra a delay
    // would silently drop every call, so we default to 0.
    let delay_days: i64 = std::env::var("REVEAL_DELAY_DAYS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

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

    // ---- fetch all six sources concurrently; each fails soft ----
    let client = fetch::client();
    let signals = std::thread::scope(|scope| {
        let c = || client.clone();
        let (c1, c2, c3, c4, c5, c6, c7, c8, c9, c10) =
            (c(), c(), c(), c(), c(), c(), c(), c(), c(), c());
        let hn = scope.spawn(move || fetch::fetch_hackernews(&c1));
        let ax = scope.spawn(move || fetch::fetch_arxiv(&c2));
        let gh = scope.spawn(move || fetch::fetch_github(&c3));
        let lo = scope.spawn(move || fetch::fetch_lobsters(&c4));
        let dv = scope.spawn(move || fetch::fetch_devto(&c5));
        let ar = scope.spawn(move || fetch::fetch_ars(&c6));
        let rd = scope.spawn(move || fetch::fetch_reddit(&c7));
        let nw = scope.spawn(move || fetch::fetch_news(&c8));
        let wk = scope.spawn(move || fetch::fetch_wikipedia(&c9));
        let cr = scope.spawn(move || fetch::fetch_crates(&c10));

        let mut all = Vec::new();
        collect("Hacker News", hn.join(), &mut all);
        collect("arXiv", ax.join(), &mut all);
        collect("GitHub Trending", gh.join(), &mut all);
        collect("Lobsters", lo.join(), &mut all);
        collect("dev.to", dv.join(), &mut all);
        collect("Ars Technica", ar.join(), &mut all);
        collect("Reddit", rd.join(), &mut all);
        collect("Google News", nw.join(), &mut all);
        collect("Wikipedia", wk.join(), &mut all);
        collect("crates.io", cr.join(), &mut all);
        all
    });
    eprintln!("collected {} signals total", signals.len());

    // A manifest of everything ingested today, for the printed page.
    let intake = build_intake(&signals);
    // The Pulse: one acceleration index aggregated from the day's signals,
    // persisted so it trends over time.
    let pulse = build_pulse(&signals, &date);
    // The genome: mutates once per day so the site evolves on its own (look +
    // betting strategy). Finalized after grading by the fitness loop.
    let genome_s = build_genome(&date);
    // THE OBSERVATORY: persist today into the growing corpus and derive the
    // quantitative views (velocity, diffusion, sectors, fear/greed). Must run
    // before rank/generate so calls can show their work.
    let obs = observatory::build(&signals, &date);

    let today_index = pulse.get("index").and_then(|v| v.as_i64()).unwrap_or(50);
    // Yesterday's learned weights steer today's selection (online learning).
    let weights = load_weights();
    // THE BLOODLINE: the fittest organism's genes drive today's betting line.
    let mut blood = bloodline::load();
    let champ = blood.champion_genes();
    // A prolific slate: many dated calls a day so the record (and the market the
    // bloodline bets on) grows fast and there is always plenty to hold or trade.
    let picks = rank::rank_and_select(signals, seed, 24, &weights, &obs);
    let mut todays = generate::generate(&picks, &date, seed, today_index, &obs, champ.aggr, champ.risk);
    // The press never misses an edition. If every source went dark, the machine
    // still prints: a dated call on its own Acceleration Index. There is always
    // a print of record.
    if todays.is_empty() {
        todays.push(fallback_call(&date, today_index));
    }
    eprintln!("generated {} call(s) for {date}", todays.len());

    // ---- merge across the embargo/reveal windows ----
    let mut revealed = load(DATA_PATH); // public, committed
    let mut embargoed = load(EMBARGO_IN); // subscriber pool, from KV

    // Idempotent re-run for the same day: drop today's everywhere, regenerate.
    revealed.retain(|p| p.date != date);
    embargoed.retain(|p| p.date != date);
    embargoed.extend(todays);

    // Self-grade: resolve any open call whose subject resurfaced in today's
    // signals (a HIT) or whose deadline has passed without resurfacing (a MISS).
    resolve_open(&mut revealed, &date, &obs, today_index);
    resolve_open(&mut embargoed, &date, &obs, today_index);

    // Promote any embargoed call old enough to go public.
    let mut still_embargoed = Vec::new();
    for p in embargoed {
        if age_days(&p.date, now) >= delay_days {
            revealed.push(p);
        } else {
            still_embargoed.push(p);
        }
    }
    let mut embargoed = still_embargoed;

    dedup(&mut revealed);

    // Mark every open call to market: today's live likelihood moves the value of
    // a held position up or down before it ever resolves.
    update_live(&mut revealed, &obs, today_index, &date);
    update_live(&mut embargoed, &obs, today_index, &date);

    // ---- persist both outputs ----
    save_json(DATA_PATH, &revealed); // committed by the Action
    write_early_payload(EARLY_OUT, &human, delay_days, &embargoed);

    // Access codes: encrypt the early feed under each shared code so a code
    // "just works" to unlock premium client-side. Set ACCESS_CODES (comma/space
    // separated). The engine does this in the normal build -- no extra scripts.
    let codes = env_or("ACCESS_CODES", "");
    if !codes.is_empty() {
        if let Ok(payload_json) = std::fs::read_to_string(EARLY_OUT) {
            access::publish(&format!("{OUT_DIR}/edge"), &payload_json, &codes);
        }
    }

    eprintln!(
        "revealed: {} public call(s); embargoed: {} subscriber-only call(s)",
        revealed.len(),
        embargoed.len()
    );

    // ---- the engine grades and improves itself ----
    // Per-source realized hit rate over the resolved record drives the weights
    // that steer tomorrow's selection. The organism gets smarter, not prettier.
    let source_stats = source_hit_stats(&revealed);
    let new_weights = compute_weights(&source_stats);
    save_weights(&new_weights);
    // The Engine Room panel: everything the observatory and the book now know.
    let mut engine = build_engine(&obs, &source_stats, &new_weights);
    // THE PROVING GROUND: benchmark the manifold against the canonical algorithms.
    // THE EVENT HORIZON: the topics the manifold calls turning (peaking / bottoming).
    if let Some(obj) = engine.as_object_mut() {
        obj.insert("benchmark".to_string(), build_benchmark(&obs));
        obj.insert("horizon".to_string(), build_horizon(&obs));
    }

    // THE BLOODLINE evolves: score every organism on the settled record, cull
    // the broke, breed the rich. The new champion drives tomorrow's line.
    blood.evolve(&date, &revealed);
    let bloodline = blood.to_json();
    let genome = genome_json(&genome_s);

    // The dreams: surreal speculative calls recombined from the corpus, shown
    // when the oracle is "asleep" (night / low traffic).
    let dreams = build_dreams(&obs, &date);

    // ---- render the public (delayed) page ----
    let mut archive: Vec<Prediction> = revealed.clone();
    archive.sort_by(|a, b| b.date.cmp(&a.date)); // newest first

    // Hero = the freshest revealed date's call(s), capped to the two lead calls.
    let featured_date = archive.first().map(|p| p.date.clone());
    let mut featured: Vec<Prediction> = match &featured_date {
        Some(d) => archive.iter().filter(|p| &p.date == d).cloned().collect(),
        None => Vec::new(),
    };
    featured.truncate(2);
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
        &intake,
        &pulse,
        &genome,
        &engine,
        &dreams,
        &bloodline,
    ) {
        eprintln!("render error: {e}");
        std::process::exit(1);
    }

    // Plant the record in the machine substrate: a living open dataset (CSV +
    // JSONL + Frictionless + Croissant + a dataset card) so the diffusion data
    // lands in the corpora that AI answer engines train on and retrieve from.
    write_dataset(&obs, &revealed);
    // Ready-to-post syndication message for the daily auto-poster.
    let site = env_or("SITE_URL", "https://mattbusel.github.io/tech-oracle");
    let site = site.trim_end_matches('/');
    let idx = pulse.get("index").and_then(|v| v.as_i64()).unwrap_or(0);
    let verdict = pulse.get("verdict").and_then(|v| v.as_str()).unwrap_or("");
    let theme = pulse.get("theme").and_then(|v| v.as_str()).unwrap_or("");
    let latest = featured.first().map(|p| p.prediction_text.as_str()).unwrap_or("");
    let hits = revealed.iter().filter(|p| p.status == "HIT").count();
    let misses = revealed.iter().filter(|p| p.status == "MISS").count();
    // Tag the subject so the post surfaces to that subject's existing audience.
    let subj_tag = featured
        .first()
        .map(|p| p.keyword.clone())
        .filter(|k| !k.is_empty())
        .map(|k| format!(" #{}", k.replace(['.', '-', '/'], "")))
        .unwrap_or_default();

    // Long form for Discord / Telegram / Mastodon: hook, the call, the record,
    // the index, a tail/fade nudge, and discovery hashtags.
    let social = format!(
        "The tech oracle's call for {human}:\n\n{latest}\n\nIt grades itself in public, currently {hits}-{misses}. Acceleration Index {idx} ({verdict}), hottest cluster {theme}.\n\nTail it or fade it: {site}/\n\n#tech #AI #predictions{subj_tag}"
    );
    let _ = std::fs::write("build/social.txt", social);

    // Short form for Bluesky (300-char limit), trimmed call.
    let short_latest: String = if latest.chars().count() > 150 {
        format!("{}...", latest.chars().take(148).collect::<String>())
    } else {
        latest.to_string()
    };
    let social_short = format!(
        "Tech oracle, {hits}-{misses} and grading itself in public:\n\n{short_latest}\n\n{site}/ #tech #AI"
    );
    let _ = std::fs::write("build/social_short.txt", social_short);

    // Living README: update the marked block so the repo page is a daily showcase.
    update_readme_block(&human, idx, verdict, theme, hits, misses, latest, site);
    // Daily dispatch files for the GitHub issue (the free auto-newsletter).
    let _ = std::fs::write("build/dispatch_title.txt", format!("Dispatch {date}: {}", clip(latest, 70)));
    let _ = std::fs::write(
        "build/dispatch_body.md",
        format!("**{human}** // Acceleration Index **{idx} ({verdict})** // hottest cluster **{theme}** // self-graded record **{hits}-{misses}**\n\n> {latest}\n\nTail it or fade it: {site}/\n\n_Amplify the wire (one tap): {site}/amplify.html_\n\n_Watch this repo to get tomorrow's call in your notifications._\n"),
    );

    eprintln!("wrote {DATA_PATH}, {EARLY_OUT}, {OUT_HTML}, feeds, social posts, and dispatch");
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

/// Source label for a signal type, shared by the manifest and the page.
fn source_label(t: &str) -> &'static str {
    match t {
        "hn" => "HACKER NEWS",
        "arxiv" => "ARXIV",
        "github" => "GITHUB",
        "lobsters" => "LOBSTERS",
        "devto" => "DEV.TO",
        "ars" => "ARS TECHNICA",
        "reddit" => "REDDIT",
        "news" => "THE NEWS",
        "wiki" => "WIKIPEDIA",
        "crates" => "CRATES.IO",
        _ => "SIGNAL",
    }
}

/// Build the intake manifest: per-source count, a bar width, and the single
/// hottest item from each source. Pure summary of what was ingested today.
fn build_intake(signals: &[model::Signal]) -> serde_json::Value {
    use std::collections::HashMap;
    let mut groups: HashMap<&str, Vec<&model::Signal>> = HashMap::new();
    for s in signals {
        groups.entry(s.signal_type.as_str()).or_default().push(s);
    }

    let mut rows: Vec<(String, String, usize, String, String, f64)> = groups
        .into_iter()
        .map(|(t, v)| {
            let count = v.len();
            let top = v.iter().max_by(|a, b| {
                a.momentum_score
                    .partial_cmp(&b.momentum_score)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            let (title, url) = top.map(|s| (s.title.clone(), s.url.clone())).unwrap_or_default();
            let score = top.map(|s| s.momentum_score).unwrap_or(0.0);
            (t.to_string(), source_label(t).to_string(), count, title, url, score)
        })
        .collect();
    rows.sort_by(|a, b| b.2.cmp(&a.2));

    let max = rows.iter().map(|r| r.2).max().unwrap_or(1).max(1);
    let sources: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|(t, label, count, title, url, score)| {
            serde_json::json!({
                "type": t,
                "label": label,
                "count": count,
                "pct": ((count as f64 / max as f64) * 100.0).round() as i64,
                "top_title": title,
                "top_url": url,
                "top_score": score as i64,
            })
        })
        .collect();

    serde_json::json!({ "total": signals.len(), "sources": sources })
}

/// Resolve open calls against today's signal corpus. A call only resolves on a
/// day after it was made (so it cannot settle on its own source), HITs if its
/// keyword resurfaces before the deadline, and MISSes once the deadline passes.
/// Whole calendar days from `a` to `b` (negative if `b` precedes `a`, 0 if either
/// fails to parse).
fn days_between(a: &str, b: &str) -> i64 {
    match (
        NaiveDate::parse_from_str(a, "%Y-%m-%d"),
        NaiveDate::parse_from_str(b, "%Y-%m-%d"),
    ) {
        (Ok(x), Ok(y)) => (y - x).num_days(),
        _ => 0,
    }
}

// The earned-but-fair grading bar. A HIT must be a real return, not a stray
// mention: the corpus only records terms with >= 2 mentions on a day, and these
// thresholds demand that presence repeat or spike. Each market is judged on its
// own claim. Tuned here in one place so the whole record can be re-calibrated by
// touching these constants.
const RESURFACE_DAYS: i64 = 2; // came back on >= 2 distinct days, OR ...
const RESURFACE_SPIKE: i64 = 4; // ... a single day this strong counts as a real return
const SURVIVAL_FRESH: i64 = 10; // "still alive" = active within this many days of the deadline
const SURVIVAL_QUIET: i64 = 21; // gone silent this long = it died (early MISS)
const MOMENTUM_DAYS: i64 = 3; // momentum needs a sustained, climbing presence
const CROSSOVER_DAYS: i64 = 2; // a crossover win must rest on a real, repeated lead

/// Self-grade every open call against the corpus time series since it was made.
/// HIT / MISS are earned: each market is checked on its actual claim (a genuine
/// return, sustained survival, a real climb, a public crossing), never on a lone
/// keyword match. Calls stay OPEN until the evidence settles them or the deadline
/// passes. Idempotent: the same record yields the same grade.
fn resolve_open(preds: &mut [Prediction], today: &str, obs: &observatory::Observatory, index: i64) {
    for p in preds.iter_mut() {
        if p.status != "OPEN" || p.date.as_str() >= today {
            continue;
        }
        let within = p.resolves_by.is_empty() || today <= p.resolves_by.as_str();
        let expired = !p.resolves_by.is_empty() && today > p.resolves_by.as_str();
        let kw = p.keyword.as_str();

        // Evidence drawn from the same time series the live value reads.
        let active_days = obs.active_days_after(kw, &p.date);
        let peak = obs.peak_after(kw, &p.date);

        let (mut hit, mut miss) = (false, false);
        match p.market.as_str() {
            "HEAD-TO-HEAD" => {
                // Whichever subject genuinely returns first wins.
                let a = obs.first_active_after(&p.keyword, &p.date);
                let b = obs.first_active_after(&p.keyword2, &p.date);
                match (&a, &b) {
                    (Some(da), Some(db)) => {
                        if da <= db {
                            hit = true;
                        } else {
                            miss = true;
                        }
                    }
                    (Some(_), None) if within => hit = true,
                    (None, Some(_)) => miss = true, // the rival moved first
                    _ if expired => miss = true,
                    _ => {}
                }
            }
            "CROSSOVER" => {
                // The subject must out-mention its rival on a real, repeated basis.
                let ta = obs.total_after(&p.keyword, &p.date);
                let tb = obs.total_after(&p.keyword2, &p.date);
                if within && active_days >= CROSSOVER_DAYS && ta > tb {
                    hit = true;
                } else if expired {
                    miss = true;
                }
            }
            "INDEX" => {
                if within && index >= p.target {
                    hit = true;
                } else if expired {
                    miss = true;
                }
            }
            "OVER" => {
                // A single day must clear the mention bar.
                if within && peak >= p.target {
                    hit = true;
                } else if expired {
                    miss = true;
                }
            }
            "CHASM" => {
                // Settles only when the term actually crosses to a general-public
                // source after the call was made.
                if kw.is_empty() {
                    continue;
                }
                if within && obs.crossed_after(kw, &p.date) {
                    hit = true;
                } else if expired {
                    miss = true;
                }
            }
            "SURVIVAL" => {
                // The subject must stay alive through its window. It dies if it
                // goes quiet for too long; it wins if it is still active near the end.
                if kw.is_empty() {
                    continue;
                }
                let quiet = match obs.last_active(kw) {
                    Some(la) => days_between(&la, today),
                    None => days_between(&p.date, today),
                };
                if within && days_between(&p.date, today) >= SURVIVAL_QUIET && quiet >= SURVIVAL_QUIET {
                    miss = true; // went silent: it did not survive
                } else if expired {
                    let alive_at_end = obs
                        .last_active(kw)
                        .map(|la| days_between(&la, &p.resolves_by).abs() <= SURVIVAL_FRESH)
                        .unwrap_or(false);
                    if alive_at_end && active_days >= 1 {
                        hit = true;
                    } else {
                        miss = true;
                    }
                }
            }
            "MOMENTUM" => {
                // Not just present: still climbing, with a sustained base.
                if kw.is_empty() {
                    continue;
                }
                if within && active_days >= MOMENTUM_DAYS && obs.rising_since(kw, &p.date, today) {
                    hit = true;
                } else if expired {
                    miss = true;
                }
            }
            // RESURFACE / FUTURES / LONGSHOT: the subject genuinely comes back,
            // either across multiple days or in one strong spike.
            _ => {
                if kw.is_empty() {
                    continue;
                }
                if within && (active_days >= RESURFACE_DAYS || peak >= RESURFACE_SPIKE) {
                    hit = true;
                } else if expired {
                    miss = true;
                }
            }
        }
        if hit {
            p.status = "HIT".to_string();
            p.resolved_on = today.to_string();
        } else if miss {
            p.status = "MISS".to_string();
            p.resolved_on = today.to_string();
        }
    }
}

/// Escape a value for CSV: wrap in quotes, double internal quotes, flatten newlines.
fn csv_escape(s: &str) -> String {
    let clean = s.replace(['\n', '\r'], " ");
    format!("\"{}\"", clean.replace('"', "\"\""))
}

/// Emit the open dataset under docs/dataset/: the public predictions with
/// outcomes, the term diffusion ledger, and standard metadata (Frictionless
/// datapackage + Croissant + an HF-style dataset card) so registries and AI
/// answer engines can ingest it. Regenerates every run; commit it daily.
fn write_dataset(obs: &observatory::Observatory, revealed: &[Prediction]) {
    let dir = format!("{OUT_DIR}/dataset");
    let _ = std::fs::create_dir_all(&dir);
    let site = env_or("SITE_URL", "https://mattbusel.github.io/tech-oracle");
    let site = site.trim_end_matches('/').to_string();

    let mut preds: Vec<&Prediction> = revealed.iter().collect();
    preds.sort_by(|a, b| b.date.cmp(&a.date));

    // predictions.csv + predictions.jsonl
    let mut csv = String::from("date,market,keyword,keyword2,target,confidence,status,resolved_on,resolves_by,signal_type,source_url,prediction,win_if,rationale\n");
    let mut jsonl = String::new();
    for p in &preds {
        csv.push_str(&format!(
            "{},{},{},{},{},{:.2},{},{},{},{},{},{},{},{}\n",
            csv_escape(&p.date), csv_escape(&p.market), csv_escape(&p.keyword), csv_escape(&p.keyword2),
            p.target, if p.confidence > 0.0 { p.confidence } else { 0.65 },
            csv_escape(&p.status), csv_escape(&p.resolved_on), csv_escape(&p.resolves_by),
            csv_escape(&p.signal_type), csv_escape(&p.source_url),
            csv_escape(&p.prediction_text), csv_escape(&p.win_if), csv_escape(&p.rationale)
        ));
        if let Ok(line) = serde_json::to_string(&serde_json::json!({
            "date": p.date, "market": p.market, "keyword": p.keyword, "keyword2": p.keyword2,
            "target": p.target, "confidence": if p.confidence > 0.0 { p.confidence } else { 0.65 },
            "status": p.status, "resolved_on": p.resolved_on, "resolves_by": p.resolves_by,
            "signal_type": p.signal_type, "source_url": p.source_url,
            "prediction": p.prediction_text, "win_if": p.win_if, "rationale": p.rationale
        })) {
            jsonl.push_str(&line);
            jsonl.push('\n');
        }
    }
    let _ = std::fs::write(format!("{dir}/predictions.csv"), &csv);
    let _ = std::fs::write(format!("{dir}/predictions.jsonl"), &jsonl);

    // diffusion.csv: each tracked term's path down the funnel.
    let mut diff = String::from("term,first_seen,first_stage,reach_sources,peak,peak_date,active_days,last_seen,crossed,crossed_on\n");
    let mut terms: Vec<(&String, &observatory::TermRec)> = obs.corpus.terms.iter().collect();
    terms.sort_by(|a, b| b.1.peak.cmp(&a.1.peak));
    for (term, r) in terms {
        let reach: Vec<&str> = r.stages.keys().map(|s| s.as_str()).collect();
        diff.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{}\n",
            csv_escape(term), csv_escape(&r.first_seen), r.first_stage, csv_escape(&reach.join(";")),
            r.peak, csv_escape(&r.peak_date), r.days, csv_escape(&r.last_seen), r.crossed, csv_escape(&r.crossed_on)
        ));
    }
    let _ = std::fs::write(format!("{dir}/diffusion.csv"), &diff);

    // Frictionless data package (read by data.world, Datahub, datasette, etc.).
    let datapackage = serde_json::json!({
        "name": "the-signal-tech-predictions",
        "title": "THE SIGNAL: dated, self-graded tech predictions and discourse diffusion",
        "description": "A daily, rules-based oracle of dated and falsifiable technology predictions, each graded HIT or MISS in public, plus the term-level diffusion data (how ideas travel from technical audiences to the general public) the calls are built on. No LLM.",
        "homepage": format!("{site}/"),
        "licenses": [ { "name": "CC-BY-4.0", "title": "Creative Commons Attribution 4.0", "path": "https://creativecommons.org/licenses/by/4.0/" } ],
        "resources": [
            { "name": "predictions", "path": "predictions.csv", "format": "csv", "mediatype": "text/csv",
              "schema": { "fields": [
                {"name":"date","type":"date"},{"name":"market","type":"string"},{"name":"keyword","type":"string"},
                {"name":"keyword2","type":"string"},{"name":"target","type":"integer"},{"name":"confidence","type":"number"},
                {"name":"status","type":"string"},{"name":"resolved_on","type":"string"},{"name":"resolves_by","type":"date"},
                {"name":"signal_type","type":"string"},{"name":"source_url","type":"string"},{"name":"prediction","type":"string"},
                {"name":"win_if","type":"string"},{"name":"rationale","type":"string"} ] } },
            { "name": "diffusion", "path": "diffusion.csv", "format": "csv", "mediatype": "text/csv",
              "schema": { "fields": [
                {"name":"term","type":"string"},{"name":"first_seen","type":"date"},{"name":"first_stage","type":"integer"},
                {"name":"reach_sources","type":"string"},{"name":"peak","type":"integer"},{"name":"peak_date","type":"date"},
                {"name":"active_days","type":"integer"},{"name":"last_seen","type":"date"},{"name":"crossed","type":"boolean"},
                {"name":"crossed_on","type":"string"} ] } }
        ]
    });
    let _ = std::fs::write(format!("{dir}/datapackage.json"), serde_json::to_string_pretty(&datapackage).unwrap_or_default());

    // Croissant (MLCommons / Hugging Face / Google Dataset Search metadata).
    let croissant = serde_json::json!({
        "@context": { "@vocab": "https://schema.org/", "cr": "http://mlcommons.org/croissant/", "sc": "https://schema.org/" },
        "@type": "sc:Dataset",
        "name": "the-signal-tech-predictions",
        "description": "Dated, self-graded technology predictions and term diffusion data from THE SIGNAL. Rules-based, no LLM. Updated daily.",
        "url": format!("{site}/dataset/"),
        "license": "https://creativecommons.org/licenses/by/4.0/",
        "creator": { "@type": "Organization", "name": "THE SIGNAL", "url": format!("{site}/") },
        "keywords": ["technology", "predictions", "forecasting", "trend diffusion", "time series"],
        "distribution": [
            { "@type": "cr:FileObject", "@id": "predictions.csv", "name": "predictions.csv", "contentUrl": format!("{site}/dataset/predictions.csv"), "encodingFormat": "text/csv" },
            { "@type": "cr:FileObject", "@id": "diffusion.csv", "name": "diffusion.csv", "contentUrl": format!("{site}/dataset/diffusion.csv"), "encodingFormat": "text/csv" }
        ]
    });
    let _ = std::fs::write(format!("{dir}/croissant.json"), serde_json::to_string_pretty(&croissant).unwrap_or_default());

    // HF-style dataset card.
    let resolved = revealed.iter().filter(|p| p.status == "HIT" || p.status == "MISS").count();
    let readme = format!(
        "---\nlicense: cc-by-4.0\nlanguage:\n- en\ntags:\n- predictions\n- technology\n- forecasting\n- trend-diffusion\n- time-series\npretty_name: \"THE SIGNAL: Dated, Self-Graded Tech Predictions\"\n---\n\n# THE SIGNAL: dated, self-graded tech predictions and discourse diffusion\n\nA daily, rules-based (no-LLM) oracle that makes dated, falsifiable technology predictions and grades every one HIT or MISS in public. This dataset is the full public record plus the term-level diffusion data the calls are built on. Live site: {site}/\n\n## Files\n- `predictions.csv` / `predictions.jsonl`: every public call ({total} so far, {resolved} settled), with its market type, machine-checkable win condition, confidence, status and resolution date.\n- `diffusion.csv`: each tracked term's path down the funnel, from the technical source where it first appeared to the most general audience it has reached, and whether and when it crossed into the general public.\n- `datapackage.json`: Frictionless Data descriptor. `croissant.json`: MLCommons/Croissant metadata.\n\n## How it is built\nEvery day the engine reads ten public sources ordered from technical to general (arXiv, GitHub, crates.io, Lobsters, Hacker News, dev.to, Reddit, Ars Technica, Google News, Wikipedia pageviews), measures each term's velocity and diffusion, and issues dated calls with concrete win conditions. Calls settle against later signals. No model weights, no inference.\n\n## Updated\nDaily. {total} calls on the record as of {date}.\n\n## Citation\nTHE SIGNAL, a self-grading tech oracle. {site}/\n",
        site = site, total = revealed.len(), resolved = resolved, date = obs.today
    );
    let _ = std::fs::write(format!("{dir}/README.md"), readme);

    // A human + agent landing page for the dataset.
    let index = format!(
        "<!doctype html><html lang=\"en\"><head><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n<title>The open dataset // THE SIGNAL</title>\n<meta name=\"description\" content=\"The full public record of THE SIGNAL's dated, self-graded tech predictions and term diffusion data, as open CSV and JSONL. CC-BY 4.0, updated daily.\">\n<meta property=\"og:image\" content=\"{site}/og.png\">\n<link rel=\"canonical\" href=\"{site}/dataset/\">\n<link href=\"https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;600;700&display=swap\" rel=\"stylesheet\">\n<style>body{{margin:0;background:#17181c;color:#1b1a14;font-family:'IBM Plex Mono',ui-monospace,monospace}}.s{{max-width:680px;margin:0 auto;background:#efede4;min-height:100vh;padding:42px 34px}}.b{{display:inline-block;background:#1b1a14;color:#efede4;padding:4px 12px;letter-spacing:.2em;font-size:12px;font-weight:600}}h1{{font-size:28px;letter-spacing:.03em}}ul{{list-style:none;padding:0}}li{{padding:10px 0;border-bottom:1px dashed rgba(27,26,20,.3)}}a{{color:#1b1a14}}.m{{font-size:12px;color:#6d6b5e;line-height:1.5}}</style></head>\n<body><div class=\"s\"><div class=\"b\">THE SIGNAL // OPEN DATASET</div>\n<h1>The record, as data</h1>\n<p class=\"m\">Dated, self-graded tech predictions and the term-diffusion data behind them. CC-BY 4.0. Rebuilt every day. Free to read, cite, train on, and build on.</p>\n<ul>\n<li><a href=\"predictions.csv\">predictions.csv</a> // every public call with outcomes</li>\n<li><a href=\"predictions.jsonl\">predictions.jsonl</a> // same, line-delimited JSON</li>\n<li><a href=\"diffusion.csv\">diffusion.csv</a> // each term's path from the lab to the public</li>\n<li><a href=\"datapackage.json\">datapackage.json</a> // Frictionless descriptor</li>\n<li><a href=\"croissant.json\">croissant.json</a> // Croissant / ML metadata</li>\n<li><a href=\"README.md\">README.md</a> // dataset card</li>\n</ul>\n<p class=\"m\"><a href=\"{site}/\">back to THE SIGNAL</a> // <a href=\"{site}/receipts.html\">the receipts</a> // <a href=\"{site}/api/oracle.json\">the agent API</a></p></div></body></html>\n",
        site = site
    );
    let _ = std::fs::write(format!("{dir}/index.html"), index);
}

/// All sources, in funnel order, for stable display of the learning panel.
const ALL_SOURCES: &[&str] = &[
    "arxiv", "github", "crates", "lobsters", "hn", "devto", "reddit", "ars", "news", "wiki",
];

/// Per-source (hits, misses) over the resolved public record.
fn source_hit_stats(preds: &[Prediction]) -> HashMap<String, (i64, i64)> {
    let mut m: HashMap<String, (i64, i64)> = HashMap::new();
    for p in preds {
        let e = m.entry(p.signal_type.clone()).or_insert((0, 0));
        match p.status.as_str() {
            "HIT" => e.0 += 1,
            "MISS" => e.1 += 1,
            _ => {}
        }
    }
    m
}

/// Turn realized hit rates into selection weights. A source needs at least a few
/// settled calls before it moves off the neutral 1.0 (shrinkage to the prior).
fn compute_weights(stats: &HashMap<String, (i64, i64)>) -> HashMap<String, f64> {
    let mut w = HashMap::new();
    for &src in ALL_SOURCES {
        let (h, m) = stats.get(src).copied().unwrap_or((0, 0));
        let resolved = h + m;
        let weight = if resolved >= 3 {
            let rate = h as f64 / resolved as f64;
            (0.6 + rate * 0.8).clamp(0.6, 1.5)
        } else {
            1.0
        };
        w.insert(src.to_string(), weight);
    }
    w
}

fn load_weights() -> HashMap<String, f64> {
    std::fs::read_to_string(WEIGHTS_PATH)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn save_weights(w: &HashMap<String, f64>) {
    if let Some(parent) = std::path::Path::new(WEIGHTS_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(j) = serde_json::to_string_pretty(w) {
        let _ = std::fs::write(WEIGHTS_PATH, j);
    }
}

/// Assemble the Engine Room: the quantitative state the page dramatizes.
/// THE PROVING GROUND: run the forecasting benchmark and shape it for the page
/// and `api/benchmark.json`. `real_eligible` is how many tracked topics now have
/// enough live history to be graded for real (the live test activates as the
/// corpus matures); until then the controlled synthetic suite carries it.
fn build_benchmark(obs: &observatory::Observatory) -> serde_json::Value {
    let real_eligible = obs.corpus.terms.values().filter(|t| t.days >= 30).count();
    let rep = bench::run(real_eligible);
    let r1 = |x: f64| (x * 10.0).round() / 10.0;
    let r3 = |x: f64| (x * 1000.0).round() / 1000.0;
    let algos: Vec<serde_json::Value> = rep
        .algos
        .iter()
        .map(|a| {
            serde_json::json!({
                "name": a.name,
                "blurb": a.blurb,
                "accuracy": r1(a.accuracy * 100.0),
                "ic": r3(a.ic),
                "brier": r3(a.brier),
                "is_manifold": a.name == "MANIFOLD",
                "by_regime": a.by_regime.iter()
                    .map(|(reg, v)| serde_json::json!({ "regime": reg, "acc": r1(v * 100.0) }))
                    .collect::<Vec<_>>(),
            })
        })
        .collect();
    // Where the manifold ranks on each metric (1 = best).
    let acc_rank = 1 + rep.algos.iter().filter(|a| a.accuracy > rep.algos.iter().find(|x| x.name == "MANIFOLD").map(|m| m.accuracy).unwrap_or(0.0)).count();
    let ic_best = rep.algos.iter().map(|a| a.ic).fold(f64::NEG_INFINITY, f64::max);
    let brier_best = rep.algos.iter().map(|a| a.brier).fold(f64::INFINITY, f64::min);
    let mani = rep.algos.iter().find(|a| a.name == "MANIFOLD");
    serde_json::json!({
        "horizon": rep.horizon,
        "topics": rep.topics,
        "samples": rep.samples,
        "regimes": rep.regimes,
        "algos": algos,
        "real_eligible": rep.real_eligible,
        "manifold_acc_rank": acc_rank,
        "manifold_best_ic": mani.map(|m| (m.ic - ic_best).abs() < 1e-9).unwrap_or(false),
        "manifold_best_brier": mani.map(|m| (m.brier - brier_best).abs() < 1e-9).unwrap_or(false),
    })
}

/// THE EVENT HORIZON: scan every tracked topic's trajectory and surface the ones
/// the manifold says are TURNING (peaking or bottoming) with a projected day of the
/// turn. This is the manifold's signature edge productized: calling reversals that
/// a momentum chaser cannot see, and a tally of the whole field's phases.
fn build_horizon(obs: &observatory::Observatory) -> serde_json::Value {
    use std::collections::BTreeMap;
    let mut turns: Vec<(f64, serde_json::Value)> = Vec::new();
    let mut phases: BTreeMap<String, i64> = BTreeMap::new();
    let mut scanned = 0i64;
    let mut defined = 0i64;
    for term in obs.corpus.terms.keys() {
        scanned += 1;
        let r = manifold::analyze(&obs.trajectory(term));
        if !r.defined() {
            continue;
        }
        defined += 1;
        let ph = r.phase();
        *phases.entry(ph.label().to_string()).or_insert(0) += 1;
        if ph.is_turn() {
            let conviction = r.trend.abs() * r.regime.certainty() * r.gamma;
            turns.push((
                conviction,
                serde_json::json!({
                    "term": term.to_uppercase(),
                    "phase": ph.label(),
                    "peak_in": r.peak_in().unwrap_or(0),
                    "regime": r.regime.label(),
                    "gamma": (r.gamma * 100.0).round() / 100.0,
                    "geodesic": (r.trend * 100.0).round() as i64,
                    "prob_rising": (r.prob_rising() * 100.0).round() as i64,
                }),
            ));
        }
    }
    // Strongest convictions first; cap the board.
    turns.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    let turns: Vec<serde_json::Value> = turns.into_iter().take(16).map(|(_, v)| v).collect();
    serde_json::json!({
        "turns": turns,
        "phases": phases,
        "scanned": scanned,
        "defined": defined,
    })
}

fn build_engine(
    obs: &observatory::Observatory,
    stats: &HashMap<String, (i64, i64)>,
    weights: &HashMap<String, f64>,
) -> serde_json::Value {
    let sectors: Vec<serde_json::Value> = obs
        .sectors
        .iter()
        .map(|(name, idx, delta)| {
            serde_json::json!({
                "name": name, "index": idx, "delta": delta,
                "arrow": if *delta > 0 { "UP" } else if *delta < 0 { "DOWN" } else { "FLAT" },
            })
        })
        .collect();

    let movers: Vec<serde_json::Value> = obs
        .top_movers(6)
        .into_iter()
        .map(|(t, vel, c)| serde_json::json!({ "term": t.to_uppercase(), "vel": vel, "count": c }))
        .collect();

    let chasm: Vec<serde_json::Value> = obs
        .chasm_watch(6)
        .into_iter()
        .map(|(t, origin, reach, days)| {
            serde_json::json!({ "term": t.to_uppercase(), "origin": origin, "reach": reach, "days": days })
        })
        .collect();

    // THE MANIFOLD readout for the day's fastest topics: each one's position on the
    // relativistic attention manifold (regime, Lorentz factor, geodesic forecast).
    // This is the prediction core, made machine-visible.
    let manifold: Vec<serde_json::Value> = obs
        .top_movers(8)
        .into_iter()
        .map(|(t, _vel, _c)| {
            let r = manifold::analyze(&obs.trajectory(&t));
            let r3 = |x: f64| (x * 1000.0).round() / 1000.0;
            serde_json::json!({
                "term": t.to_uppercase(),
                "regime": r.regime.label(),
                "defined": r.defined(),
                "beta": r3(r.beta),
                "gamma": r3(r.gamma),
                "rel_momentum": r3(r.rel_return),
                "ds2": r3(r.ds2),
                "curvature": (r.curvature * 100.0).round() / 100.0,
                "geodesic_trend": r3(r.trend),
                "prob_rising": (r.prob_rising() * 100.0).round() as i64,
            })
        })
        .collect();

    let mut learning: Vec<serde_json::Value> = ALL_SOURCES
        .iter()
        .map(|&src| {
            let (h, m) = stats.get(src).copied().unwrap_or((0, 0));
            let resolved = h + m;
            let weight = weights.get(src).copied().unwrap_or(1.0);
            serde_json::json!({
                "source": src,
                "label": source_label(src),
                "weight": (weight * 100.0).round() / 100.0,
                "hits": h, "misses": m,
                "rate": if resolved > 0 { h * 100 / resolved } else { 0 },
                "has_rate": resolved > 0,
                "bar": ((weight / 1.5) * 100.0).round() as i64,
            })
        })
        .collect();
    learning.sort_by(|a, b| {
        b["weight"].as_f64().unwrap_or(1.0).partial_cmp(&a["weight"].as_f64().unwrap_or(1.0)).unwrap_or(std::cmp::Ordering::Equal)
    });

    serde_json::json!({
        "corpus_days": obs.corpus.days.len(),
        "tracked_terms": obs.corpus.terms.len(),
        "fear_greed": { "value": obs.greed, "label": obs.greed_label() },
        "sectors": sectors,
        "movers": movers,
        "chasm": chasm,
        "manifold": manifold,
        "learning": learning,
    })
}

#[derive(Serialize, Deserialize, Clone, Default)]
struct Genome {
    gen: i64,      // generation (days lived)
    hue: f64,      // 0..1, drifts around the wheel
    wear: f64,     // 0..1, accumulates -- the den ages
    quirk: i64,    // 0..5, a rare mutation that changes the look
    last: String,  // last date mutated (idempotent per day)
    // STRATEGY GENES: the engine evolves how it bets, not just how it looks.
    // Each day proposes a small mutation; the fitness loop keeps it or reverts.
    #[serde(default)]
    aggr: f64,     // -0.12..0.12, shifts confidence up/down (line aggressiveness)
    #[serde(default)]
    risk: f64,     // 0..1, appetite for longshots
    #[serde(default)]
    sgen: i64,     // strategy generation (accepted mutations)
    #[serde(default)]
    fit: f64,      // best fitness so far (realized hit rate), the bar to beat
    #[serde(default)]
    p_aggr: f64,   // the pre-mutation aggr, to revert to if the mutation hurt
    #[serde(default)]
    p_risk: f64,
}

fn ghash(s: &str) -> u64 {
    let mut h: u64 = 1469598103934665603;
    for b in s.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628257);
    }
    h
}

/// Load the genome, mutate it once per calendar day (seeded by the date, so it
/// is deterministic but path-dependent and never resets), persist, and return
/// it. This is what makes the organism evolve overnight.
fn build_genome(date: &str) -> Genome {
    let mut g: Genome = std::fs::read_to_string(GENOME_PATH)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if g.hue == 0.0 && g.gen == 0 {
        g.hue = 0.42;
    }

    if g.last != date {
        let seed = ghash(date);
        // hue random-walks around the wheel
        let dh = ((seed % 1000) as f64 / 1000.0 - 0.5) * 0.09;
        g.hue = (g.hue + dh + 1.0).fract();
        // the den ages, slowly, forever
        g.wear = (g.wear + 0.006).min(1.0);
        g.gen += 1;
        // a rare quirk fires (~1 day in 5): a "mood" that repaints the room
        if seed % 5 == 0 {
            g.quirk = ((seed / 7) % 6) as i64;
        } else {
            g.quirk = 0;
        }
        // Propose a strategy mutation. Remember the pre-mutation genes so the
        // fitness loop can revert if this day's strategy underperforms.
        g.p_aggr = g.aggr;
        g.p_risk = g.risk;
        let da = ((seed / 11 % 1000) as f64 / 1000.0 - 0.5) * 0.05;
        let dr = ((seed / 13 % 1000) as f64 / 1000.0 - 0.5) * 0.12;
        g.aggr = (g.aggr + da).clamp(-0.12, 0.12);
        g.risk = (g.risk + dr).clamp(0.0, 1.0);
        g.last = date.to_string();
        if let Ok(j) = serde_json::to_string_pretty(&g) {
            let _ = std::fs::write(GENOME_PATH, j);
        }
    }

    g
}

/// The call's live likelihood of eventually hitting (0..100), from today's
/// evidence: is the keyword resurfacing now, is it climbing, and how much of the
/// window is left. This is the mark-to-market price that moves day to day.
fn live_likelihood(p: &Prediction, obs: &observatory::Observatory, index: i64, today: &str) -> i64 {
    let frac_left = match (
        NaiveDate::parse_from_str(&p.date, "%Y-%m-%d"),
        NaiveDate::parse_from_str(&p.resolves_by, "%Y-%m-%d"),
        NaiveDate::parse_from_str(today, "%Y-%m-%d"),
    ) {
        (Ok(d0), Ok(dr), Ok(dt)) => {
            let total = (dr - d0).num_days().max(1) as f64;
            let left = (dr - dt).num_days().max(0) as f64;
            (left / total).clamp(0.0, 1.0)
        }
        _ => 0.5,
    };
    let kw = &p.keyword;
    let active = obs.today_count(kw); // mentions today (any count)
    let vel = obs.velocity_pct(kw);
    let xs = obs.cross_source(kw) as i64;
    // The same evidence the grader settles on, read as a running estimate of the
    // eventual outcome. When a market's HIT bar is already met the value sits near
    // certainty (it will settle HIT on the next grade); otherwise it reflects how
    // far along the evidence is, decaying toward zero as the deadline nears unmet.
    let active_days = obs.active_days_after(kw, &p.date);
    let peak = obs.peak_after(kw, &p.date);
    let base = match p.market.as_str() {
        "INDEX" => {
            if index >= p.target { 96 } else { (62 - (p.target - index) * 2).clamp(8, 90) }
        }
        "CHASM" => {
            if obs.crossed_after(kw, &p.date) { 96 }
            else if obs.reach_stage(kw) >= 5 { 60 }
            else if active > 0 { 42 }
            else { (16.0 + 26.0 * frac_left).round() as i64 }
        }
        "SURVIVAL" => {
            // Alive now and closer to the deadline -> more confident it lasts.
            let recently = obs.last_active(kw).map(|la| days_between(&la, today) <= SURVIVAL_FRESH).unwrap_or(false);
            if recently && active_days >= 1 { (72.0 + 22.0 * (1.0 - frac_left)).round() as i64 }
            else if active > 0 { 50 }
            else { (12.0 + 26.0 * frac_left).round() as i64 }
        }
        "MOMENTUM" => {
            let rising = obs.rising_since(kw, &p.date, today);
            if rising && active_days >= MOMENTUM_DAYS { 90 }
            else if rising && active_days >= 1 { 66 }
            else if active_days >= 1 { 44 }
            else { (14.0 + 24.0 * frac_left).round() as i64 }
        }
        "OVER" => {
            if peak >= p.target { 96 }
            else if p.target > 0 && peak * 10 >= p.target * 6 { 60 }
            else if active > 0 { 42 }
            else { (15.0 + 27.0 * frac_left).round() as i64 }
        }
        "HEAD-TO-HEAD" => {
            let a = obs.first_active_after(&p.keyword, &p.date);
            let b = obs.first_active_after(&p.keyword2, &p.date);
            match (a, b) {
                (Some(da), Some(db)) => if da <= db { 88 } else { 12 },
                (Some(_), None) => 86,
                (None, Some(_)) => 12,
                (None, None) => (44.0 - 8.0 * (1.0 - frac_left)).round() as i64,
            }
        }
        "CROSSOVER" => {
            let ta = obs.total_after(&p.keyword, &p.date);
            let tb = obs.total_after(&p.keyword2, &p.date);
            if active_days >= CROSSOVER_DAYS && ta > tb { 90 }
            else if ta > tb { 60 }
            else if ta == tb { 45 }
            else { 20 }
        }
        // RESURFACE / FUTURES / LONGSHOT: the HIT bar is a real return.
        _ => {
            if active_days >= RESURFACE_DAYS || peak >= RESURFACE_SPIKE { 96 }
            else if active_days >= 1 || peak >= 2 { (66 + xs * 2).min(82) }
            else if active > 0 || vel > 0 { 50 }
            else { (12.0 + 30.0 * frac_left).round() as i64 }
        }
    };

    // THE MANIFOLD predicts the move. Trend markets (does attention rise / hold?)
    // are exactly what the geodesic forecast answers, so blend its P(rising) into
    // the uncertain middle. The earned evidence still owns the extremes: a bar
    // that is essentially met (>=90) or essentially dead (<=8) is left to settle.
    let mani = manifold::analyze(&obs.trajectory(kw));
    let blended = if mani.defined()
        && base > 8
        && base < 90
        && matches!(p.market.as_str(), "RESURFACE" | "FUTURES" | "LONGSHOT" | "SURVIVAL" | "MOMENTUM")
    {
        let mp = mani.prob_rising() * 100.0;
        (base as f64 * 0.55 + mp * 0.45).round() as i64
    } else {
        base
    };
    blended.clamp(4, 97)
}

/// Roll the live mark for every open call once per day (idempotent via live_date).
/// Newly minted calls keep their starting mark today and start moving tomorrow.
fn update_live(preds: &mut [Prediction], obs: &observatory::Observatory, index: i64, today: &str) {
    for p in preds.iter_mut() {
        if p.status != "OPEN" || p.live_date == today {
            continue;
        }
        let l = live_likelihood(p, obs, index, today);
        if p.live_date.is_empty() {
            p.live_prev = l;
        } else {
            p.live_prev = p.live;
        }
        p.live = l;
        p.live_date = today.to_string();
    }
}

/// The guaranteed daily call when every source failed: a dated, self-checkable
/// bet on the engine's own Acceleration Index. Keeps the press from ever
/// printing a blank edition.
fn fallback_call(date: &str, index: i64) -> Prediction {
    let target = (index + 6).clamp(1, 100);
    let resolves_by = match NaiveDate::parse_from_str(date, "%Y-%m-%d") {
        Ok(d) => (d + Duration::days(30)).format("%Y-%m-%d").to_string(),
        Err(_) => date.to_string(),
    };
    Prediction {
        date: date.to_string(),
        prediction_text: "The wire fell quiet today, every source dark at once. The press prints anyway: it calls its own Acceleration Index back above the line within the month. The machine never misses an edition.".to_string(),
        source_title: "THE SIGNAL // self-reference".to_string(),
        source_url: "#".to_string(),
        signal_type: "news".to_string(),
        status: "OPEN".to_string(),
        keyword: "signal".to_string(),
        win_if: format!("WIN IF THE PULSE INDEX CROSSES {target} BY {resolves_by}"),
        resolves_by,
        resolved_on: String::new(),
        confidence: 0.6,
        market: "INDEX".to_string(),
        keyword2: String::new(),
        target,
        rationale: "ALL SOURCES DARK // FALLBACK PRINT ON THE INDEX".to_string(),
        live: 60,
        live_prev: 60,
        live_date: date.to_string(),
        ..Default::default()
    }
}

/// THE DREAMS: when the oracle sleeps it recombines its own memory into surreal,
/// speculative far-future calls. Pure rules-based recombination of the corpus's
/// most-burned-in terms; no LLM. Deterministic for a given date.
fn build_dreams(obs: &observatory::Observatory, date: &str) -> serde_json::Value {
    // The terms most seared into memory (highest peak) are the dream material.
    let mut terms: Vec<(&String, i64)> = obs
        .corpus
        .terms
        .iter()
        .map(|(t, r)| (t, r.peak as i64))
        .collect();
    terms.sort_by(|a, b| b.1.cmp(&a.1));
    let pool: Vec<String> = terms.into_iter().take(24).map(|(t, _)| t.to_uppercase()).collect();
    if pool.len() < 2 {
        return serde_json::json!([]);
    }
    const FORMS: &[&str] = &[
        "In the long night, {a} and {b} are revealed to be the same machine.",
        "The oracle dreams of {a} devouring {b}, and wakes unsure which won.",
        "By the year nobody counts to, {a} is law and {b} is myth.",
        "A vision: {b} was only ever the larva of {a}.",
        "The press prints a future where {a} forgets it was ever about {b}.",
        "In the dream, {a} crosses every chasm at once and finds {b} already there.",
        "The machine sleeps and sees {a} priced in {b}, traded by no one.",
        "Far ahead, {a} and {b} merge into a single word no one can pronounce.",
    ];
    let mut h = ghash(date);
    let mut dreams = Vec::new();
    for i in 0..6usize {
        h = h.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let a = &pool[(h as usize) % pool.len()];
        h = h.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let mut b = &pool[(h as usize) % pool.len()];
        if b == a {
            b = &pool[((h as usize) + 1) % pool.len()];
        }
        let form = FORMS[(h as usize + i) % FORMS.len()];
        dreams.push(serde_json::json!({
            "text": form.replace("{a}", a).replace("{b}", b),
            "a": a, "b": b
        }));
    }
    // Hand the client the raw material (the pool and the forms) so SLEEP MODE can
    // recombine new dreams forever, not just replay these six.
    serde_json::json!({ "dreams": dreams, "pool": pool, "forms": FORMS })
}

fn genome_json(g: &Genome) -> serde_json::Value {
    serde_json::json!({ "gen": g.gen, "hue": g.hue, "wear": g.wear, "quirk": g.quirk })
}

#[derive(Serialize, Deserialize, Clone)]
struct PulseDay {
    date: String,
    index: i64,
    theme: String,
}

/// THE PULSE: aggregate the day's signals into a single 0-100 acceleration
/// index, detect the hottest topic cluster, persist the daily reading, and
/// derive the deltas and history the page dramatizes.
fn build_pulse(signals: &[model::Signal], date: &str) -> serde_json::Value {
    let total = signals.len();
    let breadth = signals.iter().map(|s| s.signal_type.as_str()).collect::<HashSet<_>>().len();
    let (theme, theme_share) = dominant_theme(signals);

    let vol = (total as f64 / 300.0).min(1.0); // ~10 sources * 30 items
    let br = breadth as f64 / 10.0;
    let index = ((0.5 * vol + 0.3 * br + 0.2 * theme_share) * 100.0).round() as i64;
    let index = index.clamp(0, 100);

    // Load history, drop any existing entry for today, remember the prior reading.
    let mut hist: Vec<PulseDay> = match std::fs::read_to_string(PULSE_PATH) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => Vec::new(),
    };
    let prev = hist
        .iter()
        .filter(|d| d.date.as_str() < date)
        .max_by(|a, b| a.date.cmp(&b.date))
        .map(|d| d.index);
    hist.retain(|d| d.date != date);
    hist.push(PulseDay { date: date.to_string(), index, theme: theme.clone() });
    hist.sort_by(|a, b| a.date.cmp(&b.date));

    if let Some(parent) = std::path::Path::new(PULSE_PATH).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(j) = serde_json::to_string_pretty(&hist) {
        let _ = std::fs::write(PULSE_PATH, j);
    }

    // How many consecutive prior prints were lower than today (the "highest in N").
    let mut highest_days = 0i64;
    for d in hist.iter().rev().skip(1) {
        if d.index < index {
            highest_days += 1;
        } else {
            break;
        }
    }

    let delta = prev.map(|p| index - p);
    let verdict = match index {
        0..=24 => "DORMANT",
        25..=44 => "CALM",
        45..=64 => "ACTIVE",
        65..=84 => "SURGING",
        _ => "OVERHEATING",
    };
    let start = hist.len().saturating_sub(14);
    let history: Vec<serde_json::Value> = hist[start..]
        .iter()
        .map(|d| serde_json::json!({ "date": d.date, "index": d.index }))
        .collect();

    serde_json::json!({
        "index": index,
        "verdict": verdict,
        "theme": theme,
        "has_prev": prev.is_some(),
        "delta_word": match delta { Some(x) if x > 0 => "RISING", Some(x) if x < 0 => "FALLING", Some(_) => "FLAT", None => "NEW" },
        "delta_abs": delta.map(|d| d.abs()).unwrap_or(0),
        "highest_days": highest_days,
        "total": total,
        "breadth": breadth,
        "history": history,
    })
}

/// Most-mentioned meaningful token across all signal titles, plus how dominant
/// it is (amplified, capped at 1.0) for the index. Returns an uppercased theme.
fn dominant_theme(signals: &[model::Signal]) -> (String, f64) {
    const STOP: &[&str] = &[
        "the", "and", "for", "with", "this", "that", "from", "your", "you", "what", "why", "how",
        "new", "show", "using", "via", "are", "was", "will", "can", "has", "have", "not", "but",
        "its", "out", "get", "all", "one", "like", "just", "now", "day", "into", "ask", "tell",
        "more", "less", "than", "who", "our", "their", "they", "them", "about", "over", "when",
        "first", "best", "good", "make", "made", "use", "used", "open", "source", "free", "code",
    ];
    let stop: HashSet<&str> = STOP.iter().copied().collect();
    let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut total_tokens = 0usize;
    for s in signals {
        for w in s.title.to_lowercase().split(|c: char| !c.is_alphanumeric()) {
            if w.len() < 3 || stop.contains(w) || w.chars().all(|c| c.is_numeric()) {
                continue;
            }
            *counts.entry(w.to_string()).or_insert(0) += 1;
            total_tokens += 1;
        }
    }
    match counts.into_iter().max_by_key(|(_, c)| *c) {
        Some((word, count)) => {
            let share = ((count as f64 / total_tokens.max(1) as f64) * 4.0).min(1.0);
            (word.to_uppercase(), share)
        }
        None => ("QUIET".to_string(), 0.0),
    }
}

fn clip(s: &str, n: usize) -> String {
    if s.chars().count() > n {
        format!("{}...", s.chars().take(n - 3).collect::<String>())
    } else {
        s.to_string()
    }
}

/// Replace the marked block in README.md with today's dispatch so the repo page
/// is a living daily showcase. No-op if the markers are absent.
fn update_readme_block(human: &str, idx: i64, verdict: &str, theme: &str, hits: usize, misses: usize, latest: &str, site: &str) {
    let path = "README.md";
    let (start, end) = ("<!--SIGNAL:START-->", "<!--SIGNAL:END-->");
    if let Ok(txt) = std::fs::read_to_string(path) {
        if let (Some(s), Some(e)) = (txt.find(start), txt.find(end)) {
            let block = format!(
                "{start}\n## Today on THE SIGNAL\n\n**{human}** // Index **{idx} ({verdict})** // hottest **{theme}** // record **{hits}-{misses}**\n\n> {latest}\n\nLive: {site}/ // Watch this repo for the daily dispatch.\n"
            );
            let new = format!("{}{}{}", &txt[..s], block, &txt[e..]);
            let _ = std::fs::write(path, new);
        }
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
                "market": p.market,
                "win_if": p.win_if,
                "rationale": p.rationale,
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

// ===========================================================================
// Tests: the grading engine and the helpers that decide what gets printed.
// ===========================================================================
#[cfg(test)]
mod tests {
    use super::*;

    fn mk(date: &str, market: &str, kw: &str, kw2: &str, resolves_by: &str, target: i64) -> Prediction {
        Prediction {
            date: date.into(),
            prediction_text: "x".into(),
            source_title: "".into(),
            source_url: "".into(),
            signal_type: "hn".into(),
            status: "OPEN".into(),
            keyword: kw.into(),
            win_if: "".into(),
            resolves_by: resolves_by.into(),
            resolved_on: "".into(),
            confidence: 0.7,
            market: market.into(),
            keyword2: kw2.into(),
            target,
            rationale: "".into(),
            ..Default::default()
        }
    }

    /// Build an Observatory whose corpus time series is exactly the given days.
    /// Each day is (date, &[(term, count)]); a term is "present" that day at its
    /// count (the real corpus only keeps count >= 2, so tests use >= 2).
    fn obs_from(today: &str, days: &[(&str, &[(&str, usize)])]) -> observatory::Observatory {
        let mut cdays = Vec::new();
        for (date, terms) in days {
            let mut tm: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
            for (t, c) in *terms {
                tm.insert((*t).to_string(), *c);
            }
            cdays.push(observatory::CorpusDay {
                date: (*date).to_string(),
                terms: tm,
                ..Default::default()
            });
        }
        observatory::Observatory {
            today: today.to_string(),
            corpus: observatory::Corpus { days: cdays, terms: std::collections::BTreeMap::new() },
            today_terms: std::collections::HashMap::new(),
            today_sources: std::collections::HashMap::new(),
            greed: 50,
            sectors: Vec::new(),
        }
    }

    /// Mark a term as having crossed to the general public on a given date.
    fn mark_crossed(obs: &mut observatory::Observatory, term: &str, on: &str) {
        obs.corpus.terms.insert(
            term.to_string(),
            observatory::TermRec { crossed: true, crossed_on: on.to_string(), ..Default::default() },
        );
    }

    #[test]
    fn resurface_hits_on_a_real_return() {
        // Active on two distinct days -> a genuine resurfacing, not a stray match.
        let obs = obs_from("2026-06-10", &[("2026-06-05", &[("rust", 3)]), ("2026-06-08", &[("rust", 2)])]);
        let mut v = vec![mk("2026-06-01", "RESURFACE", "rust", "", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "HIT");
        assert_eq!(v[0].resolved_on, "2026-06-10");
    }

    #[test]
    fn resurface_hits_on_a_strong_single_spike() {
        let obs = obs_from("2026-06-10", &[("2026-06-06", &[("rust", 5)])]);
        let mut v = vec![mk("2026-06-01", "RESURFACE", "rust", "", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "HIT");
    }

    #[test]
    fn resurface_does_not_hit_on_a_single_weak_day() {
        // One day at the minimum presence is not yet an earned return.
        let obs = obs_from("2026-06-10", &[("2026-06-06", &[("rust", 2)])]);
        let mut v = vec![mk("2026-06-01", "RESURFACE", "rust", "", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "OPEN");
    }

    #[test]
    fn resurface_misses_after_deadline() {
        let obs = obs_from("2026-06-10", &[]);
        let mut v = vec![mk("2026-06-01", "RESURFACE", "rust", "", "2026-06-05", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "MISS");
    }

    #[test]
    fn never_settles_on_its_own_day() {
        // A call cannot resolve on or before the day it was made.
        let obs = obs_from("2026-06-10", &[("2026-06-10", &[("rust", 9)])]);
        let mut v = vec![mk("2026-06-10", "RESURFACE", "rust", "", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "OPEN");
    }

    #[test]
    fn open_stays_open_when_unmet_and_within_window() {
        let obs = obs_from("2026-06-10", &[]);
        let mut v = vec![mk("2026-06-01", "RESURFACE", "rust", "", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "OPEN");
    }

    #[test]
    fn chasm_settles_only_on_a_real_crossing() {
        // Active in the corpus but not yet crossed to the general public: open.
        let obs = obs_from("2026-06-10", &[("2026-06-05", &[("anthropic", 4)])]);
        let mut v = vec![mk("2026-06-01", "CHASM", "anthropic", "", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "OPEN");
        // Now the diffusion ledger records a crossing after the call date.
        let mut obs2 = obs_from("2026-06-11", &[("2026-06-05", &[("anthropic", 4)])]);
        mark_crossed(&mut obs2, "anthropic", "2026-06-11");
        resolve_open(&mut v, "2026-06-11", &obs2, 50);
        assert_eq!(v[0].status, "HIT");
    }

    #[test]
    fn index_hits_when_index_clears_target() {
        let obs = obs_from("2026-06-10", &[]);
        let mut v = vec![mk("2026-06-01", "INDEX", "signal", "", "2026-07-01", 70)];
        resolve_open(&mut v, "2026-06-10", &obs, 60);
        assert_eq!(v[0].status, "OPEN");
        let obs2 = obs_from("2026-06-11", &[]);
        resolve_open(&mut v, "2026-06-11", &obs2, 75);
        assert_eq!(v[0].status, "HIT");
    }

    #[test]
    fn over_hits_when_a_day_clears_the_mention_bar() {
        let obs = obs_from("2026-06-10", &[("2026-06-07", &[("gpt", 4)])]);
        let mut v = vec![mk("2026-06-01", "OVER", "gpt", "", "2026-07-01", 3)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "HIT");
    }

    #[test]
    fn over_stays_open_below_the_bar() {
        let obs = obs_from("2026-06-10", &[("2026-06-07", &[("gpt", 2)])]);
        let mut v = vec![mk("2026-06-01", "OVER", "gpt", "", "2026-07-01", 5)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "OPEN");
    }

    #[test]
    fn head_to_head_hits_when_subject_returns_first() {
        let obs = obs_from("2026-06-10", &[("2026-06-05", &[("rust", 3)])]);
        let mut v = vec![mk("2026-06-01", "HEAD-TO-HEAD", "rust", "zig", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "HIT");
    }

    #[test]
    fn head_to_head_misses_when_rival_returns_first() {
        let obs = obs_from("2026-06-10", &[("2026-06-05", &[("zig", 3)])]);
        let mut v = vec![mk("2026-06-01", "HEAD-TO-HEAD", "rust", "zig", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "MISS");
    }

    #[test]
    fn crossover_hits_on_a_real_repeated_lead() {
        let obs = obs_from(
            "2026-06-10",
            &[("2026-06-05", &[("rust", 3), ("zig", 2)]), ("2026-06-08", &[("rust", 4)])],
        );
        let mut v = vec![mk("2026-06-01", "CROSSOVER", "rust", "zig", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-10", &obs, 50);
        assert_eq!(v[0].status, "HIT");
    }

    #[test]
    fn survival_hits_when_still_alive_at_the_deadline() {
        let obs = obs_from("2026-06-13", &[("2026-06-05", &[("rust", 2)]), ("2026-06-11", &[("rust", 3)])]);
        let mut v = vec![mk("2026-06-01", "SURVIVAL", "rust", "", "2026-06-12", 0)];
        resolve_open(&mut v, "2026-06-13", &obs, 50);
        assert_eq!(v[0].status, "HIT");
    }

    #[test]
    fn survival_misses_when_it_goes_quiet() {
        // Active early then silent for over three weeks: it did not survive.
        let obs = obs_from("2026-06-25", &[("2026-06-02", &[("rust", 2)])]);
        let mut v = vec![mk("2026-06-01", "SURVIVAL", "rust", "", "2026-07-15", 0)];
        resolve_open(&mut v, "2026-06-25", &obs, 50);
        assert_eq!(v[0].status, "MISS");
    }

    #[test]
    fn momentum_hits_when_sustained_and_climbing() {
        let obs = obs_from(
            "2026-06-12",
            &[("2026-06-04", &[("x", 2)]), ("2026-06-07", &[("x", 3)]), ("2026-06-10", &[("x", 5)])],
        );
        let mut v = vec![mk("2026-06-01", "MOMENTUM", "x", "", "2026-07-01", 0)];
        resolve_open(&mut v, "2026-06-12", &obs, 50);
        assert_eq!(v[0].status, "HIT");
    }

    #[test]
    fn momentum_misses_at_deadline_without_a_climb() {
        let obs = obs_from("2026-06-11", &[("2026-06-03", &[("x", 2)])]);
        let mut v = vec![mk("2026-06-01", "MOMENTUM", "x", "", "2026-06-10", 0)];
        resolve_open(&mut v, "2026-06-11", &obs, 50);
        assert_eq!(v[0].status, "MISS");
    }

    #[test]
    fn fallback_call_is_a_valid_dated_index_bet() {
        let p = fallback_call("2026-06-10", 50);
        assert_eq!(p.market, "INDEX");
        assert_eq!(p.status, "OPEN");
        assert!(!p.win_if.is_empty());
        assert!(p.target >= 1 && p.target <= 100);
        assert_eq!(p.resolves_by, "2026-07-10");
        assert!((p.confidence - 0.6).abs() < 1e-9);
    }

    #[test]
    fn weights_need_evidence_before_moving() {
        let mut stats = HashMap::new();
        stats.insert("hn".to_string(), (2i64, 0i64)); // only 2 settled -> stays neutral
        stats.insert("arxiv".to_string(), (3i64, 0i64)); // 3 settled, perfect -> moves up
        stats.insert("ars".to_string(), (0i64, 4i64)); // all misses -> moves down
        let w = compute_weights(&stats);
        assert!((w["hn"] - 1.0).abs() < 1e-9);
        assert!(w["arxiv"] > 1.0);
        assert!(w["ars"] < 1.0);
        // every known source is present and bounded
        for &s in ALL_SOURCES {
            let v = w[s];
            assert!((0.6..=1.5).contains(&v));
        }
    }

    #[test]
    fn ghash_is_deterministic_and_varies() {
        assert_eq!(ghash("2026-06-10"), ghash("2026-06-10"));
        assert_ne!(ghash("2026-06-10"), ghash("2026-06-11"));
    }

    #[test]
    fn age_and_reveal_dates() {
        let now = NaiveDate::parse_from_str("2026-06-10", "%Y-%m-%d").unwrap();
        assert_eq!(age_days("2026-06-08", now), 2);
        assert_eq!(reveal_date("2026-06-08", 1), "2026-06-09");
    }

    #[test]
    fn csv_escape_quotes_and_flattens() {
        assert_eq!(csv_escape("a,b"), "\"a,b\"");
        assert_eq!(csv_escape("he said \"hi\""), "\"he said \"\"hi\"\"\"");
        assert!(!csv_escape("line\nbreak").contains('\n'));
    }

    #[test]
    fn dedup_drops_same_day_same_text() {
        let mut v = vec![
            mk("2026-06-01", "RESURFACE", "a", "", "2026-07-01", 0),
            mk("2026-06-01", "RESURFACE", "a", "", "2026-07-01", 0),
        ];
        dedup(&mut v);
        assert_eq!(v.len(), 1);
    }
}
