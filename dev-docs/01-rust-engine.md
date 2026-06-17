# 01: The Rust engine

The crate is a single binary (`tech-oracle`) plus a separate `wasm/` crate. This
file documents every module, type, function and constant. Signatures are exact.

Crate manifest (`Cargo.toml`): deps are `reqwest` (blocking, json, rustls-tls,
no OpenSSL), `serde`/`serde_json`, `feed-rs` (Atom/RSS), `regex` (GitHub scrape;
`scraper` is deliberately avoided because its codegen pulled a `rand` that fails
on rustc 1.91), `minijinja`, `chrono`, `anyhow`, `aes-gcm` + `pbkdf2` + `sha2` +
`base64` + `getrandom` (access codes), `png` (share cards). Release profile is
`opt-level = 2`.

Module graph (declared in `main.rs`): `access`, `bloodline`, `card`, `fetch`,
`generate`, `model`, `observatory`, `rank`, `render`.

## `bloodline.rs` (the breeding population / genetic algorithm)

The oracle's strategy is evolved by a population, not a single hill-climb.
- `struct Genes { aggr, risk, conf, select, press, fade }` - six heritable
  instincts: line aggressiveness, stake variance, confidence bias, selectivity
  (how high a bar before it bets), conviction (pressing a hot streak), and
  tail-vs-fade (>0.5 = contrarian, bets against the oracle).
- `struct Organism { id, name, born, parents, genes, fitness, best, age, alive,
  died, bets, wins, losses, win_rate, max_streak, biggest, roi }` - identity plus
  the full stat line (career-high `best` ratchets up; the rest recompute daily).
- `struct Bloodline { next_id, gen, population, last_evolved, hall_of_fame }` -
  persisted to `data/bloodline.json`. `hall_of_fame` keeps the all-time greats by
  career-high bankroll, never pruned.
- `struct SimStats` + `fn simulate(genes, calls, seed) -> SimStats` - the betting
  sim: ONE bet per settled call (shuffled per organism by `seed`), a fraction of
  bankroll staked, selectivity skipping marginal calls, tail/fade, pressing a
  streak, busting to zero. One pass (no replayed laps), so a runaway perfect run
  is impossible over a real record and bankrolls stay sane. Returns bank plus the
  full stat line. This is the fitness.
- `pub fn load() -> Bloodline`.
- `Bloodline::champion_genes()` - the fittest living organism's genes (drives the
  live line in `generate`).
- `Bloodline::evolve(date, resolved)` - calls `evolve_in_memory` then `save()`.
- `Bloodline::evolve_in_memory(date, resolved)` - the IO-free selection core
  (seed/score/age/cull/breed), idempotent per day via `last_evolved`. Split out
  so tests can exercise it without touching `data/bloodline.json`. Seed the
  founding population if empty;
  else score everyone via `simulate`, age them, and once there are >= 5 settled
  calls cull to `SURVIVORS` (7), breed back to `TARGET` (10) via `crossover` +
  mutation, prune the graveyard, persist.
- `Bloodline::to_json()` - champion + living (ranked, with stat lines) + recent
  dead + houses + newborns + rookies (promising young) + pros (top career) +
  hall_of_fame, for the broadcast.
The Hall of Fame updates every run (idempotent: `best` only ratchets).
Internals: a small LCG `Rng` seeded by `ghash(date)` (deterministic), a `NAMES`
word list for organism names.

---

## `model.rs` (the data model)

Two structs, both the shared vocabulary of the whole engine.

### `struct Signal`
One normalized item from any source.
- `signal_type: String` - source tag: `hn | arxiv | github | lobsters | devto | ars | reddit | news | wiki | crates`.
- `title: String` - sanitized headline (see `collapse_ws`).
- `url: String`.
- `momentum_score: f64` - source-native heat (HN score, GitHub stars-today, recency rank, downloads, pageviews). Comparable only within a source until `rank.rs` normalizes it.

### `struct Prediction` (Serialize + Deserialize)
The persisted source of truth for a call. All grading fields default via
`#[serde(default)]` so old records load.
- `date: String` (YYYY-MM-DD), `prediction_text`, `source_title`, `source_url`, `signal_type`.
- `status: String` - `OPEN | HIT | MISS`.
- `keyword: String` - the token watched for resurfacing.
- `win_if: String` - human-readable win condition.
- `resolves_by: String` - YYYY-MM-DD deadline.
- `resolved_on: String` - settle date or "".
- `confidence: f64` - 0.34..0.95; the line is `1/confidence`.
- `market: String` - see markets in `05-conventions-and-glossary.md`.
- `keyword2: String` - the rival, for HEAD-TO-HEAD / CROSSOVER.
- `target: i64` - index threshold (INDEX) or mention threshold (OVER).
- `rationale: String` - the machine reasoning tape.
- `live, live_prev: i64`, `live_date: String` - the mark-to-market likelihood, its
  prior reading, and the last day it rolled.
- `regime: String`, `gamma: f64`, `geodesic: i64`, `phase: String` - the manifold
  stamp at call time: the space-time regime that shaped the bet, the Lorentz
  factor, the forward geodesic forecast in percent, and the phase (RISING /
  PEAKING / FALLING / BOTTOMING / CHURNING / FLAT). Empty/`0`/`1.0` when warming up.

---

## `fetch.rs` (the ten sources)

All fetchers are independent, return `anyhow::Result<Vec<Signal>>`, and are run
concurrently by `main`. A dead source is logged and skipped.

- `pub fn client() -> reqwest::blocking::Client` - shared client, 20s timeout, a
  descriptive User-Agent (required by Wikimedia and crates.io). Cheap to clone.
- `pub fn fetch_hackernews(client)` - Firebase `topstories.json`, then the top 30
  item lookups concurrently (a nested `thread::scope`). momentum = HN score.
- `pub fn fetch_arxiv(client)` - Atom feed for `cs.AI OR cs.LG`, newest first;
  momentum = recency rank (`n - i`).
- `pub fn fetch_github(client)` - scrapes `github.com/trending?since=daily` with
  two regexes (repo anchor, "N stars today") and zips them. Defensive: shifted
  markup yields fewer signals, never a crash. momentum = stars today.
- `pub fn fetch_lobsters(client)` - `lobste.rs/hottest.json`. momentum = score.
- `pub fn fetch_devto(client)` - `dev.to/api/articles?per_page=30&top=7`.
  momentum = positive reactions.
- `pub fn fetch_ars(client)` - Ars Technica RSS. momentum = recency rank.
- `pub fn fetch_reddit(client)` - `r/technology/top.json?t=day`. (Often HTTP 403
  from datacenter IPs; works from the GitHub runner. Fails soft.)
- `pub fn fetch_news(client)` - Google News Technology RSS; trims the
  " - Publisher" tail. momentum = recency rank.
- `pub fn fetch_wikipedia(client)` - Wikimedia REST top pageviews. **Project
  string must be `en.wikipedia`** (not `en.wikipedia.org`). Walks back day-1..4
  until a day resolves (pageview data lags). Skips namespaced/meta titles. This
  is the general-public attention layer. momentum = views.
- `pub fn fetch_crates(client)` - `crates.io/api/v1/summary`,
  `most_recently_downloaded`. The real-adoption layer. momentum = recent downloads.

Helpers:
- `fn collapse_ws(s) -> String` - the house-style sanitizer. Normalizes
  typographic dashes/quotes/ellipsis to ASCII, **strips emoji and pictographic
  ranges** (0x1F000+, dingbats, arrows, regional indicators, ZWJ, variation
  selectors), collapses whitespace. This is the first line of defense for the
  no-em-dash/no-emoji rule.
- `fn parse_leading_number(s) -> f64` - "1,234 stars today" -> 1234.0.

To add a source: write `fetch_x`, add a `signal_type` tag, spawn it in `main`'s
scope and `collect()` it, give it a stage in `observatory::stage_of`, a label in
`main::source_label` and the template `labels` map, and a color in the template
CSS (`.s-x` and `i.s-x`/`.sm-dot.s-x`).

---

## `rank.rs` (selection)

- `pub fn rank_and_select(signals: Vec<Signal>, seed: i64, max_picks: usize, weights: &HashMap<String,f64>) -> Vec<Signal>`
  - Computes per-source max momentum, normalizes each signal to `0..1` within its
    source (cross-source comparability).
  - Multiplies the normalized score by the learned source `weight` (default 1.0):
    sources whose past calls landed get amplified, chronic missers shrink.
  - Adds a tiny date-seeded per-source nudge so ties rotate which source leads.
  - Greedily takes the top `max_picks`, skipping near-duplicate topics.
- `fn near_duplicate(a, b) -> bool` - Jaccard token overlap > 0.6.
- `fn tokens(s) -> HashSet<String>` - lowercased words len>2 minus a small stoplist.

---

## `generate.rs` (rules -> dated calls)

- `const HORIZON_DAYS: i64 = 30`, `const FUTURES_DAYS: i64 = 90`.
- `const MARKETS: &[&str]` - the rotation pool: RESURFACE, CHASM, MOMENTUM, OVER,
  HEAD-TO-HEAD, SURVIVAL, CROSSOVER, INDEX, FUTURES, LONGSHOT.
- `pub fn generate(signals, date, seed, index, obs: &Observatory, aggr: f64, risk: f64) -> Vec<Prediction>`
  (`aggr` shifts confidence, `risk` turns some calls into longshots; both are the
  current bloodline champion's genes)
  - For each pick: derive `subject` and `keyword`; pull features from the
    observatory (`velocity_pct`, `cross_source`, `is_crossing`, `rationale`).
  - **The manifold shapes the bet.** `manifold::analyze(obs.trajectory(keyword))`
    reads the topic's attention trajectory; once it is defined (3+ days of
    history) the local space-time regime selects the market from
    `manifold::market_pool` (TIMELIKE -> trend bets MOMENTUM/SURVIVAL/RESURFACE/
    FUTURES; LIGHTLIKE -> transition bets CHASM/HEAD-TO-HEAD/CROSSOVER; SPACELIKE
    -> spike bets OVER/LONGSHOT/INDEX). Before it is defined, the plain
    `(seed + i) % MARKETS.len()` rotation runs (early days stay varied). Then
    validate against the data:
    - CHASM only survives if the term has a technical origin (room to cross),
      else falls back to RESURFACE.
    - HEAD-TO-HEAD / CROSSOVER need a distinct `keyword2` from the next pick.
    - OVER sets `target` = today's mention count + small jitter.
    - INDEX sets `target` = current index + 5 + jitter.
    - FUTURES uses the 90-day horizon.
  - Build `win_if` per market. Compute `confidence`: once the manifold is defined
    the geodesic forecast leads (`0.35 * data_conf + 0.65 * manifold.confidence()`),
    otherwise the data heuristic stands alone (cross-source confirmation and
    positive velocity raise it; later picks, LONGSHOT and CHASM lower it; clamped
    0.34..0.9). The manifold readout is appended to the `rationale` tape.
  - `prediction_text`: a challenge frame (`challenge_template`) when the call is
    CHASM or the term is crossing, else a source-specific `fill_template`.
- `fn pick_keyword(subject) -> String` - the longest non-trivial token (the
  watched keyword).
- `fn horizon(date, days) -> String` - date + days.
- `fn fill_template(signal_type, subject, seed, idx)` and
  `fn variants_for(signal_type) -> &[&str]` - the per-source narrative templates
  (4 variants each for all ten sources, plus a default). Variant chosen by seed+idx.
- `fn challenge_template(subject, seed, idx)` - the provocative, dated,
  falsifiable framing for crossing topics ("tail it or fade it").
- `fn subject_of(s) -> String` and `fn shorten(t, max)` - trim a title to a word
  boundary.

---

## `manifold.rs` (the prediction core)

Repurposed from SRFM (Special Relativity in Financial Modeling): a topic's daily
attention is treated as the "price" of an asset and its time series as a
trajectory through a curved relativistic space-time. Where it goes next is a
geodesic on that manifold. Dependency-free, deterministic, pure arithmetic.

- Constants: `BETA_MAX` (0.9999, the velocity ceiling), `LIGHTLIKE_EPS` (0.15,
  the transition band on the normalized interval), `HORIZON` (7, geodesic
  forecast steps), `WINDOW` (20, lookback), `MIN_POINTS` (3, below which the
  reading is neutral).
- `enum Regime { Timelike, Lightlike, Spacelike }` with `.label()` and
  `.certainty()` (1.0 / 0.6 / 0.4: how far a directional forecast is trusted).
- `struct Reading { points, beta, gamma, rel_return, ds2, regime, curvature, trend }`
  - `beta`: relativistic velocity of attention (move / local light speed),
  - `gamma`: Lorentz factor `1/sqrt(1 - beta^2)` (fast moves weigh more),
  - `rel_return`: `gamma * last_return`, the relativistic momentum,
  - `ds2`: normalized space-time interval `-1 + |v|^2 + |a|^2 + sigma^2` (its sign
    is the regime),
  - `curvature`: the geodesic-deviation z-score `(d gamma / gamma) * sign(beta)`,
  - `trend`: the forward geodesic projection over the horizon, tanh-squashed to
    [-1, 1].
  - `drift` (mean recent log-return, the causal velocity) and `accel` (its change,
    the curvature).
  - `.defined()` (>= MIN_POINTS), `.prob_rising()` (0..1 P(attention rises),
    `0.5 + 0.5*trend*certainty`), `.confidence()` (0.34..0.92 from speed + regime
    + trend), `.tag()` (the readout for the rationale tape).
  - `.forward_velocity()` (`drift + accel`, where it is actually headed next),
    `.forecast_path(steps)` (the integrated geodesic curve for charts),
    `.peak_in()` (days until the projected turn, the manifold's signature timing
    capability), and `.phase() -> Phase`.
- `enum Phase { Rising, Peaking, Falling, Bottoming, Churning, Flat }` with
  `.label()` and `.is_turn()`. The reversal states (PEAKING = rising now but
  forward velocity already negative; BOTTOMING = the mirror) are what the manifold
  does best and the trend-followers structurally miss.
- `pub fn analyze(series: &[f64]) -> Reading` - the whole pipeline: log-attention
  returns `ln(1+count)`, a local light speed = rolling max move, betas/gammas,
  the interval and regime, the curvature z-score, and the geodesic forecast.
- `fn forecast_trend(v0, a0, regime)` - integrates the geodesic forward HORIZON
  steps: momentum carries velocity (persistence by regime), curvature bends and
  decays it, and the metric resists travel through high-velocity space
  (`/ sqrt(1 + |v|)`).
- `pub fn market_pool(regime) -> &[&str]` - the regime's preferred markets (how
  topology shapes the bet). Surfaced in `api/observatory.json` as `manifold` (one
  reading per top mover) and tested in `tests_manifold.rs`.

Also wired into `rank.rs` (selection multiplies a topic's score by
`1 + 0.6 * conviction`, where conviction is `|trend| * regime.certainty()`, so the
engine selects topics it reads with strong, trustworthy conviction).

---

## `bench.rs` (the proving ground)

A head-to-head forecasting benchmark of the manifold against the canonical
algorithms it implicitly competes with. Same task for all: given a topic's
attention series up to day t, output P(attention is higher H=7 days later). Scored
on a controlled, deterministic suite of synthetic topics across six regimes
(TREND, DECLINE, MEAN-REVERT, REGIME-SWITCH, VIRAL, NOISE) with walk-forward
evaluation. Dependency-free; xorshift RNG; fixed seeds so results are reproducible.

- Contenders: MANIFOLD (this engine), MOMENTUM (EWMA, the hot-feed-ranking
  reflex), MA-CROSS (classic technical analysis), POPULARITY (the recommender
  reflex), PAGERANK (centrality on the co-movement graph, importance-by-structure),
  and RANDOM WALK (the efficient-market null, scored as a true coin flip).
- Metrics: directional accuracy, information coefficient (IC, the correlation of
  the score with the realized forward move), and Brier score (calibration, lower
  better), plus per-regime accuracy.
- `pub fn run(real_eligible) -> BenchReport` - builds the suite, runs every
  contender walk-forward, returns the sorted leaderboard. `main::build_benchmark`
  shapes it into the engine doc; `render.rs` writes `api/benchmark.json` and the
  `manifold.html` page reads both. Tested in `tests_bench.rs` (determinism, the
  null holding no edge, the manifold clearing a coin flip and finishing top-tier).
  The manifold leads on IC and Brier and ties for top directional accuracy; the
  full numbers live on the page. `real_eligible` counts tracked topics with 30+
  days of history, so a live benchmark on the real corpus activates as it matures.

---

## `observatory.rs` (memory + quant core)

The engine's memory. Persists `data/corpus.json` and derives every quantitative
view. Pure arithmetic.

### Stage funnel (diffusion axis)
`pub fn stage_of(src) -> i64`: arxiv 0, github 1, crates 2, lobsters 3, hn 4,
devto 5, reddit 6, ars 7, news 8, wiki 9. `is_technical` <= 4, `is_general` >= 6.
A term born technical that reaches a general stage has "crossed the chasm".

### Lexicons / constants
- `STOP` - tokenizer stoplist (includes discourse-noise words).
- `SECTORS: &[(&str, &[&str])]` - AI, CHIPS, RUST, CRYPTO, CLOUD, SECURITY,
  ROBOTS, PLATFORM, each a set of member tokens.
- `GREED_WORDS`, `FEAR_WORDS` - the sentiment lexicons.
- `CORPUS_PATH`, `KEEP_DAYS` (180), `PRUNE_AFTER_DAYS` (120).

### Persisted types (Serialize/Deserialize, all pub)
- `struct CorpusDay { date, total, by_source: BTreeMap, terms: BTreeMap, sectors: BTreeMap, greed: i64 }` - one day's snapshot (terms kept only if count>=2).
- `struct TermRec { first_seen, first_stage, stages: BTreeMap<source,date>, peak, peak_date, days, last_seen, crossed, crossed_on }` - one term's longitudinal ledger.
- `struct Corpus { days: Vec<CorpusDay>, terms: BTreeMap<String, TermRec> }`.

### Live type
- `struct Observatory { today, corpus, today_terms: HashMap, today_sources: HashMap<term, HashSet<source>>, greed: i64, sectors: Vec<(name, index, delta)> }`.

### `pub fn build(signals, date) -> Observatory`
Reads the corpus (fail-soft to empty), counts today's terms and which sources
carry each, computes today's sector counts and the greed index, updates each
`TermRec` (first stage, per-source first-seen dates, peak, active days, chasm
crossing), prunes terms unseen for 120 days, snapshots today (idempotent),
trims to 180 days, persists, and computes the sector ticker with deltas vs
yesterday.

### Methods on `Observatory`
- `today_count(term)`, `cross_source(term)` (distinct sources today).
- `velocity_pct(term)` - today vs trailing 7-day average, as a percent (large for
  a fresh spike off zero baseline).
- `accel_pct(term)` - change in velocity (curvature).
- `reach_stage(term)` / `origin_stage(term)` and `reach_label` / `origin_label`.
- `is_crossing(term)` - technical origin and touching a general source now.
- `half_life(term) -> Option<f64>` - exponential-decay estimate from the peak.
- `rationale(term) -> String` - the reasoning tape: `VEL +x%/7d // n/10 FEEDS //
  REACH X [// ACCEL // CROSSING FROM Y // HALF-LIFE Nd]`.
- `top_movers(n)` - fastest terms today (count>=2), by velocity.
- `chasm_watch(n)` - terms crossing now or crossed within 14 days, with origin,
  reach, age.
- `greed_label()` - EXTREME FEAR .. EXTREME GREED.

---

## `main.rs` (orchestration)

Constants: `DATA_PATH`, `PULSE_PATH`, `GENOME_PATH`, `WEIGHTS_PATH` (committed
state), `EMBARGO_IN`/`EARLY_OUT` (gitignored), `OUT_DIR`/`OUT_HTML`.

`fn main()` is the flow in `README.md`. Key helpers:

- `fn collect(name, joined, out)` - drains one fetcher's thread result into the
  signal list, logging failures/panics.
- `fn source_label(t) -> &'static str` - the uppercase display label per source.
- `fn build_intake(signals) -> Value` - per-source count, bar pct, hottest item.
- `fn resolve_open(preds, today, obs, index)` - the **earned-but-fair grader**.
  Only settles calls older than today, and judges each against the corpus *time
  series since the call was made* (via `Observatory::active_days_after`,
  `peak_after`, `total_after`, `first_active_after`, `last_active`,
  `crossed_after`, `rising_since`) rather than a substring scan of one day. A
  corpus-day snapshot only keeps terms with >= 2 mentions, so "active" already
  means real discourse. Per market: RESURFACE/FUTURES/LONGSHOT (active >= 2 days
  OR a >= 4 spike), SURVIVAL (alive near the deadline; early MISS if silent 21+
  days), MOMENTUM (>= 3 active days AND climbing in the back half), HEAD-TO-HEAD
  (subject returns on/before the rival), CROSSOVER (>= 2 active days AND
  out-mentions keyword2 over the window), INDEX (index >= target), OVER (a single
  day >= target), CHASM (the diffusion ledger marks it `crossed`). MISS when the
  deadline passes. The bar constants (`RESURFACE_DAYS`, `RESURFACE_SPIKE`,
  `SURVIVAL_FRESH`, `SURVIVAL_QUIET`, `MOMENTUM_DAYS`, `CROSSOVER_DAYS`) sit at the
  top of the fn so the whole record can be re-calibrated in one place.
  Deterministic and idempotent. `fn days_between(a, b)` is the date helper.
  Tested per market in the inline `main.rs` tests.
- `fn live_likelihood(p, obs, index, today) -> i64` and `fn update_live(preds,
  obs, index, today)` - the mark-to-market: a running estimate of the eventual
  outcome computed from the *same evidence the grader settles on*, so when a HIT
  bar is already met the value sits near certainty (~96) and it decays toward
  zero as the deadline nears unmet. Rolled once per day (idempotent via
  `live_date`, `live_prev` records the move). The daily slate is ~24 calls
  (`rank_and_select(.., 24, ..)`).
- `fn fallback_call(date, index) -> Prediction` - the guaranteed daily print when
  every source fails: a dated, self-checkable INDEX bet. The press never prints a
  blank edition. (`REVEAL_DELAY_DAYS` defaults to 0 so the call is public the day
  it is made; see `04-distribution-and-ops.md`.)
- Genome: `struct Genome { gen, hue, wear, quirk, last, ... }`, `fn ghash(s)`
  (FNV-1a), `fn build_genome(date) -> Genome` (mutate the look once/day),
  `fn genome_json(g)`. Strategy evolution now lives in `bloodline.rs` (the
  champion's genes feed `generate`); the genome carries the visual DNA only.
- Dreams: `fn build_dreams(obs, date) -> Value` - returns `{ dreams, pool, forms }`:
  six seed dreams plus the raw term pool and form strings so SLEEP MODE can
  recombine endlessly client-side. `render.rs` writes `api/dreams.json` and bakes
  the pool/forms into the standalone `sleep.html` destination.
- Pulse: `struct PulseDay { date, index, theme }`, `fn build_pulse(signals, date)
  -> Value` (index from volume/breadth/theme-share, persists history, computes
  delta and "highest in N"), `fn dominant_theme(signals) -> (String, f64)`.
- Learning: `const ALL_SOURCES`, `fn source_hit_stats(preds) -> HashMap<src,(hit,miss)>`,
  `fn compute_weights(stats) -> HashMap<src,f64>` (0.6..1.5, needs >=3 settled or
  stays 1.0), `fn load_weights()`, `fn save_weights(w)`.
- `fn build_engine(obs, stats, weights) -> Value` - the Engine Room JSON:
  sectors, fear_greed, movers, chasm, manifold, learning (sorted by weight),
  corpus_days, tracked_terms. `build_benchmark` and `build_horizon` are inserted
  into it.
- `fn build_horizon(obs) -> Value` - THE EVENT HORIZON: scans every tracked
  topic's trajectory, surfaces the ones the manifold reads as turning (peaking /
  bottoming) with a projected day, sorted by conviction and capped at 16, plus a
  phase tally of the whole field. Written to `api/horizon.json`; `horizon.html`
  renders it.
- Dataset: `fn csv_escape(s)`, `fn write_dataset(obs, revealed)` - writes
  `docs/dataset/{predictions.csv, predictions.jsonl, diffusion.csv,
  datapackage.json, croissant.json, README.md, index.html}`.
- Persistence/util: `fn load(path)`, `fn save_json(path, archive)`,
  `fn write_early_payload(path, human, delay_days, embargoed)` (the paid edge,
  includes market/win_if/rationale/public_reveal_date), `fn dedup(v)`,
  `fn age_days`, `fn reveal_date`, `fn human_date`, `fn env_or`, `fn clip`,
  `fn update_readme_block(...)`.

---

## `render.rs` (artifacts)

`pub fn render(generated_human, reveal_delay_days, featured_date_human,
featured, archive, payment_link, portal_url, early_access_url, intake, pulse,
genome, engine, dreams, bloodline) -> anyhow::Result<()>`.

It builds the template context and writes everything. What it computes:

- `pages` - the ledger grouped by date; each item carries `no, prediction_text,
  status, win_if, odds, conf, market, rationale, resolved_on`.
- `calls` - the flat dot map (last 120).
- `record` / `by_source` - per-source counts across the archive.
- `scoreboard` - hits/misses/open/rate + a verdict line.
- `book` - a flat-stake virtual bankroll settled chronologically (line = 1/conf,
  conf clamped 0.34..0.95): bank, roi, streak, best run, history.
- `calibration` - Brier score over settled calls plus a predicted-vs-actual
  curve in three confidence bands, and a grade ("SHARP / HONEST / MISCALIBRATED").
- `mood` - the daily look from genome + state: heat (pulse), agit (streak/index/
  wear), wear, hue, `accent` hex, quirk + quirkName, embers, gen, age, model,
  verdict, hotHand, tagline. Consumed by the template shaders and CSS.

Artifacts written (all under `docs/` unless noted):
- `index.html` (the page), `feed.xml` (RSS).
- `call/N.html` (+ ClaimReview/Claim JSON-LD, the "called it first" date, a
  receipt stamp) and `call/N.png` (per-call og card via `card.rs`).
- `topic/<slug>.html` (per-keyword SEO pages).
- `receipts.html` (the credibility wall; HITs and MISSes with lead time).
- `arena.html` (the prediction-tournament board; client-side, settles GitHub-
  issue bets against `api/record.json`).
- `sleep.html` (SLEEP MODE: the always-running dreamscape destination, pool/forms
  baked in) and `api/dreams.json`.
- `bloodline.html` (the breeding population: living ranked, champion, graveyard)
  and `api/bloodline.json`. `mood` carries the mortality fields
  `vitality`/`lifeState`/`bank` and the bloodline `sgen` (generation) + `champion`.
- `sitemap.xml`, `sitemap-images.xml`, `robots.txt`, `<INDEXNOW_KEY>.txt`, and
  `build/indexnow.json` (the ping payload).
- `widget.js` (embeddable wire), `og.png` (daily homepage card), `badge.svg`.
- `cli` + `cli.txt` (curl-able ASCII printout, via `card::ascii_banner`).
- `llms.txt` (the agent/site map), `amplify.html` (pre-filled submit links),
  `signal.ics` (subscribable calendar).
- The agent layer via `write_agent_layer(...)`: `api/{oracle,today,record,
  observatory}.json`, `openapi.json`, `.well-known/{ai-plugin,mcp}.json`. See
  `03-data-and-formats.md`.
- `floor_json` - the live pit positions handed to the template.

Helpers: `hsl_hex`, `day_diff`, `human_date`, `rfc822`, `wrap_chars`,
`ics_escape`, `enc` (URL), `slug`, `clip_r`, `xml`. Const `INDEXNOW_KEY`.

---

## `card.rs` (server-side PNG cards)

Draws real dot-matrix dots to an RGB buffer with a hand-coded 5x7 font, encodes
PNG. No font files, no services. Used for every shareable image.
- `struct Canvas { w, h, buf }` with `new`, `px`, `dot` (filled circle), `text`
  (dot-matrix glyph run), `save` (png crate).
- `fn glyph(ch) -> [u8;5]` - the 5x7 font (digits, A-Z, and punctuation incl.
  `- . , / : ! % + ( ) ' "`). 5 columns, bit i = row i.
- `fn text_width`, `fn wrap`, `fn sprockets`, `fn wordmark`, `fn footer`, `fn stamp`.
- `pub fn site_card(...)` - the daily homepage og:image (index, record, call).
- `pub fn call_card(...)` - per-call og:image with a HIT/MISS/OPEN stamp.
- `pub fn organism_card(...)` - a collectible bloodline trading card (PRO /
  ROOKIE / HALL OF FAME): colored kind band, name, house, a stat line, and gene
  bars. `Canvas::fill_rect` draws the bars. Written to `docs/bloodline/cards/`.
- `pub fn ascii_banner(text) -> String` - the same font as a 7-row ASCII banner
  for `/cli`.
Palette consts: PAPER, INK, SOFT, DESK, STAMP, GREEN.

---

## `access.rs` (the "god pass")

Encrypts the early payload once per code so a shared code unlocks it entirely
client-side. Format matches the browser decryptor exactly:
PBKDF2-HMAC-SHA256 (100k) -> AES-256-GCM, ciphertext is `ct||tag`, fields base64.
- `fn norm(code)` - uppercase, non-alphanumerics to single hyphens, trimmed
  (so "GOLDEN TICKET" == "GOLDEN-TICKET").
- `fn sha256hex(s)` - the file name is `sha256hex(norm(code)).json`.
- `fn encrypt(plaintext, code) -> Result<String>` - random salt+iv, returns
  `{v, salt, iv, ct}` JSON.
- `pub fn publish(edge_dir, payload_json, codes)` - writes one
  `docs/edge/<hash>.json` per code (codes separated by comma/newline), and
  revokes (deletes) any edge file whose code is no longer listed. No-op when
  `codes` is empty.

---

## `wasm/` (the particle engine)

`no_std` cdylib compiled to `docs/signal.wasm` (~666 bytes). No wasm-bindgen, raw
exports, JS reads the buffer straight from wasm memory.
- `const N: usize = 1400`; `static mut BUF: [f32; N*4]` (x, y, z-depth, phase).
- `fn rnd()` - xorshift32.
- `pub extern "C" fn count() -> usize`, `particles() -> *const f32`,
  `init(seed)`, `step(dt)` - drift up, triangle-wave sway, recycle at the top.
- Build: `cargo build --release --target wasm32-unknown-unknown` then copy the
  artifact to `docs/signal.wasm`. Use a forward-slash `CARGO_TARGET_DIR` to avoid
  a backslash path-mangling bug.
