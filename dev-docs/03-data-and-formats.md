# 03: Data and formats

Three kinds of files: **committed state** (`data/*.json`, compounds daily),
**generated public artifacts** (`docs/*`, rebuilt every run), and **gitignored
scratch** (`build/*`, including the paid edge). Schemas below are exact.

---

## Committed state (`data/`)

These MUST be committed by the Action every run or memory/learning/evolution
resets. The daily commit stages all six explicitly (predictions, pulse, genome,
weights, corpus, bloodline).

### `data/predictions.json` (the public record, source of truth)
A JSON array of `Prediction` (see `01-rust-engine.md::model.rs`):
```json
[{
  "date":"2026-06-15","prediction_text":"...","source_title":"...","source_url":"...",
  "signal_type":"hn","status":"HIT","keyword":"local","win_if":"WIN IF \"LOCAL\" ...",
  "resolves_by":"2026-07-15","resolved_on":"2026-06-16","confidence":0.58,
  "market":"RESURFACE","keyword2":"","target":0,"rationale":"VEL +x%/7d // ...",
  "live":64,"live_prev":56,"live_date":"2026-06-17",
  "regime":"TIMELIKE","gamma":1.84,"geodesic":41,"phase":"PEAKING"
}]
```
`live`/`live_prev`/`live_date` are the mark-to-market likelihood (0-100), its
prior reading, and the last day it was rolled. `regime`/`gamma`/`geodesic` are the
manifold stamp at call time (the space-time regime, the Lorentz factor, the
forward geodesic forecast in percent, and the `phase` (RISING / PEAKING / FALLING /
BOTTOMING / CHURNING / FLAT); empty/`0`/`1.0` while the manifold warms up). The engine now generates ~24 calls
a day (a prolific slate) so the record and the betting market grow fast.
Only revealed (age >= `REVEAL_DELAY_DAYS`) calls live here. Idempotent per day:
a re-run drops today's and regenerates.

### `data/pulse.json`
Array of daily readings: `[{ "date":"YYYY-MM-DD", "index":0..100, "theme":"WORD" }]`.
Drives the seismograph and the index history (last 14 shown).

### `data/genome.json`
The organism's DNA, mutated once per calendar day. Look genes plus strategy genes:
```json
{ "gen": 1, "hue": 0.445, "wear": 0.006, "quirk": 0, "last": "2026-06-17",
  "aggr": 0.0, "risk": 0.0, "sgen": 1, "fit": 0.8, "p_aggr": 0.0, "p_risk": 0.0 }
```
Look: `gen` days lived, `hue` 0..1 (random walk), `wear` 0..1 (accumulates),
`quirk` 0..5 (rare), `last` idempotency guard. Strategy (self-evolution):
`aggr` confidence shift, `risk` longshot appetite, `sgen` accepted strategies,
`fit` best realized hit rate (the bar), `p_aggr`/`p_risk` the pre-mutation
values to revert to. Feeds `mood` and generation.

### `data/weights.json`
Learned per-source selection weights (online learning):
```json
{ "github": 1.4, "hn": 1.0, "arxiv": 1.0, ... }
```
0.6..1.5, computed from realized hit rate (needs >= 3 settled calls to move off
1.0). Read by `rank.rs` before selection; recomputed and saved after grading.

### `data/bloodline.json`
The breeding population of strategy-organisms (the genetic algorithm). `{ next_id,
gen, last_evolved, population: [{ id, name, born, parents,
genes{aggr,risk,conf,select,press,fade}, fitness, best, age, alive, died, bets,
wins, losses, win_rate, max_streak, biggest, roi }], hall_of_fame: [...] }`. The
fittest living organism's genes drive the live betting line; the Hall of Fame
keeps the all-time greats. Must be committed every run or the species resets.

### `data/corpus.json` (the moat)
The growing discourse time series. Two parts:
- `days`: array of `CorpusDay` `{ date, total, by_source{src:n}, terms{term:n},
  sectors{name:n}, greed }` (terms kept only if count>=2; last 180 days).
- `terms`: map `term -> TermRec { first_seen, first_stage, stages{src:date},
  peak, peak_date, days, last_seen, crossed, crossed_on }` (pruned after 120
  days unseen).
This file powers velocity, diffusion/CHASM, sectors, the dataset, and the
manifold's trajectories. It can be seeded past the warmup phase with
`tech-oracle backfill` (see `01-rust-engine.md::backfill.rs`), which reconstructs
~120 days of history by blending arXiv, Hacker News and Wikipedia archives so the
manifold is defined immediately.

---

## Generated public artifacts (`docs/`)

### The page and per-call pages
- `index.html` - the rendered single page.
- `call/N.html` - per-call permalink. Carries **ClaimReview** JSON-LD when
  settled (the call date proves "we called it first") or a dated **Claim** when
  open, plus a HIT/MISS receipt stamp with lead time.
- `call/N.png` - per-call og:image (dot-matrix card).
- `topic/<slug>.html` - per-keyword SEO page grouping all calls on a subject.
- `receipts.html` - the credibility wall: every settled call, HITs then MISSes,
  newest first, with "N days on the record".
- `arena.html` - the prediction tournament board (client-side): reads GitHub
  issues labeled `arena` plus `api/record.json`, settles every `SIGNAL-BET`, and
  ranks all players against the machine and the anti-oracle.
- `horizon.html` + `api/horizon.json` - THE EVENT HORIZON: the reversals the
  manifold is calling (topics PEAKING or BOTTOMING) with a projected day of the
  turn, plus a phase tally of the whole field. The manifold's signature edge
  (timing reversals) productized.
- `manifold.js` - the prediction core ported to the browser (faithful port of
  `manifold.rs::analyze`); any page or third party runs `window.Manifold.analyze`
  client-side. The keystone for live-without-a-server.
- `api/trajectories.json` - daily attention series for the top ~1500 tracked
  topics (`{schema, dates, days, count, series:{TERM:[counts...]}}`). Powers THE
  ORACLE BOX (type any topic, forecast it live) and the watchlist on `manifold.html`.
- `manifold.html` - THE MANIFOLD: the prediction core made visible (topics plotted
  as points on the relativistic attention manifold by regime, conviction and
  geodesic forecast) plus THE PROVING GROUND (the algorithm benchmark). Reads
  `api/observatory.json` and `api/benchmark.json` at runtime.
- `sleep.html` - SLEEP MODE: a standalone, always-running dreamscape destination
  (the term pool and forms are baked in; the client recombines new dreams
  forever). Not a takeover; reached from the footer.
- `bloodline.html` + `api/bloodline.json` - THE BLOODLINE, LIVE: the living
  population with full stat lines, the rival houses, births/deaths wire, the Hall
  of Fame, and rolling commentary with a voice. Plus **THE LIVE FLOOR**: a
  never-stopping client-side betting pit (injected into `bloodline.html`). Every
  round (~20s, FASTER on demand) the organisms gamble on a batch of `bet_pool`
  topics, settled against the manifold's true odds, so tailers grow and faders
  bleed. It runs as **seasons** (100 rounds): at the bell the season resolves, the
  champion is enshrined in **THE RAFTERS** with a one-of-one foil card, the banks
  reset, and a new season opens. Tracks the headline stats (top net worth, biggest
  win, longest streak, most misses). You can **claim a rookie card** of any live
  organism; when its season resolves, if it finished #1 the card becomes a LEGEND
  (1-of-1) attached to that historic run. Banks are uncapped within a season (runs
  can go astronomical, abbreviated K/M/B/T in the UI) and only reset at the bell.
  **Comeback + anti-runaway math**: bets are even money (no minting), underdogs
  lever up to climb, and rising friction (above ~100k) slows big banks hard so
  growth never overflows and the truly ridiculous is rare and earned. Numeric
  guards clamp banks (and sanitize any glitched saved state). A lead change fires an
  UPSET banner. If a run touches the ceiling the card is **ASCENDED** (infinite
  value, the rarest flex).
  **Currency + cards (kept deliberately simple).** Everyone starts with 1,000,000
  CRED and earns more just by watching. **SELL** has the house buy a card instantly
  for CRED (the card is destroyed) at a value scaled by its rank, stats, finish, and
  how long it was held, so holding pays. That keeps every CRED move exact and
  atomic against the house, never an honor-system peer transfer. **GIVE** hands a
  card to a friend by code (it leaves your collection; they REDEEM the code to
  receive it), no accounts, no server. One rookie claim per season; organisms are
  freshly named every season; each claim rolls a random finish (SHINY / GOLD /
  EMERALD / SAPPHIRE / DIAMOND) that tints the art and scales value. **SHARE**
  renders a card to a downloadable PNG (emblem + stat line) to post as a flex. A
  HOW IT WORKS panel explains it inline. The collection, rafters, and wallet persist in
  `localStorage`; the live game reseeds from each day's population.
- `bloodline/cards/<kind>-<id>.png` - collectible rookie / pro / hall-of-fame
  trading cards per top organism.
- `api/dreams.json` - schema `the-signal/dreams/2`: today's seed `dreams` plus
  the raw `pool` (top terms) and `forms` so any client can recombine endlessly.

### Feeds, SEO, discovery
- `feed.xml` (RSS), `sitemap.xml`, `sitemap-images.xml`, `robots.txt`,
  `<INDEXNOW_KEY>.txt` (ownership proof; the const lives in `render.rs`).
- `llms.txt` - the human/agent site map: pages, the API map, and the stateless
  betting scheme.
- `og.png` (daily homepage card), `badge.svg` (daily README badge),
  `widget.js` (one-line embeddable wire), `amplify.html` (prefilled submit
  links), `signal.ics` (subscribable calendar, one VEVENT per call resolve day),
  `cli` + `cli.txt` (curl-able ASCII printout).

### The agent-native interface (the "oracle for machines")
Static, read-only, CORS-open (GitHub Pages sets `access-control-allow-origin: *`).
- `api/oracle.json` - discovery doc: name, endpoints, the `markets` glossary, and
  `how_to_bet` (the stateless position-token scheme).
- `api/today.json` - the latest revealed date's slate. Each call: `id, date,
  market, keyword, keyword2, prediction, win_if, resolves_by, confidence, odds,
  status, resolved_on, rationale, source{type,title,url}, permalink`.
- `api/record.json` - `total, scoreboard, book, calibration, calls[]` (the whole
  record).
- `api/observatory.json` - `pulse, fear_greed, sectors, movers, chasm, manifold,
  bet_pool, source_weights, corpus_days, tracked_terms`. `bet_pool` is ~60 topics
  `{term, p (manifold P of rising), dir (+1/-1), phase, regime}` that feed the
  always-on client-side LIVE FLOOR on `bloodline.html`. `manifold` is one
  relativistic reading per prominent defined topic: `term, regime, defined, beta, gamma, rel_momentum, ds2,
  curvature, geodesic_trend, prob_rising`.
- `api/benchmark.json` - the proving ground: the manifold vs the canonical
  algorithms (momentum, MA-cross, popularity, PageRank, random walk) on the
  forecasting task, with accuracy / IC / Brier and per-regime accuracy. Built by
  `main::build_benchmark` from `bench::run`.
- `openapi.json` - a real OpenAPI 3.0 spec for the three GET endpoints
  (operationIds getTodaysCalls / getRecord / getObservatory).
- `.well-known/ai-plugin.json` - the de facto agent-discovery manifest, points to
  `openapi.json`.
- `.well-known/mcp.json` - a read-only resource manifest (today/record/observatory).

Schemas are tagged with a `schema` field (`the-signal/today/1`, etc.) so
consumers can version against them.

### The open dataset (`docs/dataset/`)
Regenerated every run by `main::write_dataset`. CC-BY.
- `predictions.csv` / `predictions.jsonl` - every public call with outcomes.
  Columns: `date, market, keyword, keyword2, target, confidence, status,
  resolved_on, resolves_by, signal_type, source_url, prediction, win_if,
  rationale`.
- `diffusion.csv` - each tracked term's path: `term, first_seen, first_stage,
  reach_sources (semicolon-joined), peak, peak_date, active_days, last_seen,
  crossed, crossed_on`.
- `datapackage.json` - Frictionless Data descriptor with field schemas (read by
  data.world, Datahub, datasette).
- `croissant.json` - MLCommons/Croissant metadata (Hugging Face, Google Dataset
  Search).
- `README.md` - HF-style dataset card (YAML frontmatter + description).
- `index.html` - the human/agent landing page.

### Encrypted early feed (`docs/edge/`)
Only generated when `ACCESS_CODES` is set. One file per code:
`docs/edge/<sha256(normalized_code)>.json` = `{ v, salt, iv, ct }` (base64),
AES-256-GCM under PBKDF2(code, 100k). Public files, useless without the code. The
browser decrypts with WebCrypto using the identical parameters.

### Static assets (committed, not generated)
`assets/scene.jpg`, `assets/scene2.jpg` (the den paintings), `.nojekyll` (so
`.well-known/` and `/api/` serve), `signal.wasm` (the particle engine, rebuilt
manually from `wasm/`).

---

## Gitignored scratch (`build/`)

Never committed (the `.gitignore` excludes `/build`):
- `early_payload.json` - the paid edge: embargoed calls + `public_reveal_date`,
  plus `market`, `win_if`, `rationale`. Pushed to KV by the Action / encrypted
  to `docs/edge/`.
- `embargoed_in.json` - the embargoed pool pulled from KV before a run (`EMBARGO_IN`).
- `indexnow.json` - the IndexNow ping payload the Action POSTs.
- `social.txt`, `social_short.txt` - ready-to-post syndication text.
- `dispatch_title.txt`, `dispatch_body.md` - the GitHub-issue dispatch.
- `commit_msg.txt` - scratch for multi-line commit messages.

Also gitignored: `/target`, `/.cargo-home`, `/_*.png`, `/_*.html` (local
screenshot/preview scratch), `/worker/node_modules`.

---

## Schema-change checklist

When you add or change a `Prediction` field, touch all of:
`model.rs` (the struct), `generate.rs` (set it), `main.rs::resolve_open` (grade
it) and `write_early_payload`/`write_dataset` (export it), `render.rs` (pages
item + `call/N.html` + `api/*` + receipts), and the template (display). Then
extend the relevant `tests_*.rs` and re-run the validation in `06-testing.md`.
