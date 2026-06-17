# THE SIGNAL: full codebase reference

This directory is the complete, scoped documentation of the project. It is for
developers (and agents) working on the code. It is not published: GitHub Pages
serves only `docs/`, so everything in `dev-docs/` stays internal.

Read the files in this order:

1. `README.md` (this file): what the project is, the repo map, the end-to-end
   data flow, and the hard conventions.
2. `01-rust-engine.md`: every Rust module, struct, function and constant.
3. `02-frontend.md`: the single-page template, every section, every JS
   subsystem, the WebGL/WASM/presence layers, and all `window.__*` globals.
4. `03-data-and-formats.md`: every persisted file and generated artifact, with
   schemas (predictions, pulse, genome, weights, corpus, the agent API, the
   open dataset).
5. `04-distribution-and-ops.md`: the GitHub Action, syndication, SEO/GEO, the
   Cloudflare Worker, secrets, and the premium/credential flows.
6. `05-conventions-and-glossary.md`: house style, markets, genome/mood, build
   and run commands, and known gotchas.
7. `06-testing.md`: the test suite and the four verification layers (unit tests,
   artifact validation, idempotency, the client-JS console sweep) plus the
   crypto interop check, with exact commands.

---

## What it is

THE SIGNAL is a self-updating, self-grading tech-prediction website. Once a day
a single Rust binary:

1. fetches free public signals from ten sources (no API keys),
2. measures the discourse (a growing corpus, velocity, diffusion, sectors,
   sentiment),
3. issues a few dated, falsifiable predictions ("calls") with concrete win
   conditions,
4. grades earlier open calls HIT or MISS against the new signals,
5. renders a static site (`docs/`) plus a large family of artifacts (feeds,
   share cards, an agent API, an open dataset, SEO pages),
6. and is committed back to the repo so GitHub Pages redeploys it.

There is no server in the hot path and no LLM anywhere. Everything is rules and
arithmetic. The site is a dot-matrix "betting den": users tail or fade the
oracle's calls with paper chips, accounts are passwordless "Press Credentials",
the page evolves visually each day (the genome), it has live multiplayer
presence, and it is readable by AI agents.

Live: https://mattbusel.github.io/tech-oracle/

---

## Repo map

```
Cargo.toml              Rust crate manifest (deps + release profile)
Cargo.lock
src/
  main.rs               Orchestration: fetch -> measure -> generate -> grade -> persist -> render
  fetch.rs              The 10 source fetchers (all fail-soft)
  model.rs              Signal and Prediction structs (the data model)
  rank.rs               Cross-source normalization + weighted selection of the day's picks
  generate.rs           Rules + date-seeded templates -> dated calls with win conditions
  observatory.rs        Memory and quant core: corpus, velocity, diffusion/CHASM, sectors, fear/greed
  render.rs             minijinja render of docs/index.html + every other artifact + agent API
  card.rs               Server-side PNG dot-matrix share cards + ASCII banner (hand-coded 5x7 font)
  access.rs             Per-code AES-GCM encryption of the early feed (the "god pass")
  tests_*.rs            #[cfg(test)] companions (one per module; main.rs has inline tests). See 06-testing.md
templates/
  index.html            The entire single-page app: HTML + CSS + all client JS (embedded at compile time)
wasm/
  Cargo.toml            no_std cdylib crate
  src/lib.rs            Particle engine compiled to docs/signal.wasm (1400 dust/embers)
worker/
  worker.js             Optional Cloudflare Worker: credential accounts + Stripe + KV early feed
  wrangler.toml         Worker config (KV binding, public vars; secrets set out-of-band)
data/                   Committed state that compounds day over day
  predictions.json      The revealed public record (source of truth for calls)
  pulse.json            Daily acceleration index history
  genome.json           The organism's evolving visual DNA (mutates once/day)
  weights.json          Learned per-source selection weights (online learning)
  corpus.json           The growing discourse time series (the moat)
docs/                   The published site (GitHub Pages root). Mostly generated; commit as-is.
  index.html            The rendered page
  call/N.html, N.png    Per-call permalink + ClaimReview + og card
  topic/<slug>.html     Per-subject SEO pages
  receipts.html         The "we called it" credibility wall
  dataset/              The open dataset (csv, jsonl, datapackage, croissant, README, index)
  api/, openapi.json, .well-known/   The agent-native interface
  feed.xml, sitemap*.xml, robots.txt, llms.txt, signal.ics, badge.svg, widget.js, amplify.html, cli, og.png, signal.wasm
  assets/scene.jpg, scene2.jpg        The den paintings (WebGL textures)
  edge/<sha256(code)>.json            Encrypted early feed per access code (generated when ACCESS_CODES set)
.github/workflows/daily.yml           The daily cron: build, run, commit, syndicate, ping
build/                  Gitignored scratch: early_payload.json (the paid edge), indexnow.json, social text, dispatch
```

---

## End-to-end data flow (one daily run)

`main()` in `src/main.rs` is the spine. In order:

1. **Read env**: `REVEAL_DELAY_DAYS` (default 1), `STRIPE_PAYMENT_LINK`,
   `STRIPE_PORTAL_URL`, `EARLY_ACCESS_URL`, `SITE_URL`, `ACCESS_CODES`,
   `LADDER_REPO`/`GITHUB_REPOSITORY`. The date and a deterministic `seed`
   (days-from-CE) are computed from `Utc::now()`.
2. **Fetch** (`fetch.rs`): one `thread::scope` spawns all ten fetchers
   concurrently. Each returns `anyhow::Result<Vec<Signal>>`; `collect()` logs
   and skips any that fail. Nothing here is ever fatal.
3. **Measure**:
   - `build_intake(signals)` -> per-source counts + top item (the intake manifest).
   - `build_pulse(signals, date)` -> the 0-100 acceleration index, hottest
     cluster, deltas; appends to and persists `data/pulse.json`.
   - `build_genome(date)` -> mutates `data/genome.json` once per calendar day
     (hue walk, wear, generation, rare quirk).
   - `observatory::build(signals, date)` -> updates and persists
     `data/corpus.json` and returns the live `Observatory` (today's term counts,
     sources, velocity, diffusion, sectors, fear/greed, plus the time-series
     queries the grader settles on).
4. **Select** (`rank.rs`): `load_weights()` reads `data/weights.json`;
   `rank_and_select(signals, seed, 4, &weights)` normalizes momentum per source,
   multiplies by the learned weight, dedups by topic, returns the top picks.
5. **Generate** (`generate.rs`): `generate(picks, date, seed, index, obs)`
   builds one `Prediction` per pick: a market (rotated and validated against the
   data), a keyword, a machine-checkable `win_if`, a `resolves_by` horizon,
   data-driven `confidence`, and a reasoning tape (`rationale`). Crossing/CHASM
   topics get a challenge frame.
6. **Merge across the embargo window**: load `data/predictions.json` (revealed)
   and `build/embargoed_in.json` (the subscriber pool, synced from KV). Drop
   today's from both (idempotent re-run), add today's calls to the embargoed
   pool.
7. **Grade** (`resolve_open`): for each open call older than today, settle HIT or
   MISS per its market against the corpus time series since the call was made
   (earned-but-fair: a real return, sustained survival, a true climb, a public
   crossing), or MISS if the deadline passed.
8. **Promote**: embargoed calls older than `delay_days` move into revealed.
   `dedup(revealed)`.
9. **Persist**: `save_json(data/predictions.json)` (committed) and
   `write_early_payload(build/early_payload.json)` (gitignored, the paid edge).
10. **Access codes**: if `ACCESS_CODES` is set, `access::publish` writes one
    AES-GCM-encrypted copy of the early payload per code to `docs/edge/`.
11. **Self-improve**: `source_hit_stats(revealed)` -> `compute_weights` ->
    `save_weights(data/weights.json)`; `build_engine(obs, stats, weights)`
    assembles the Engine Room JSON (sectors, fear/greed, movers, chasm, learning).
12. **Render** (`render.rs`): builds the template context (pages, calls, record,
    scoreboard, book, calibration, mood, engine, intake, pulse, genome) and
    writes `docs/index.html` plus every other artifact, including the agent layer.
13. **Dataset** (`write_dataset` in main): writes `docs/dataset/*` (the open
    dataset, regenerated every run).
14. **Syndication outputs**: `build/social.txt`, `build/social_short.txt`,
    `update_readme_block`, `build/dispatch_*`.

The GitHub Action (`04-distribution-and-ops.md`) then commits the changed public
files and data, pings IndexNow, and posts to any configured social channels.

---

## The two-output split (free vs paid)

Every call is generated into the **embargoed** pool first. Two outputs come from
it:

- `data/predictions.json`: calls whose age >= `REVEAL_DELAY_DAYS`. Public,
  committed, rendered. This is the free record.
- `build/early_payload.json`: calls still inside the embargo window. Gitignored,
  never committed in plaintext. This is the subscriber edge. It is delivered
  either by the Cloudflare Worker (from KV) or, keyless, as per-code encrypted
  files in `docs/edge/` (the access-code path).

The lead time is the product. The record is the proof.

---

## Hard conventions (do not break)

- **No em dashes, no emojis. Anywhere.** Not in the site, not in generated text,
  not in social posts. `fetch.rs::collapse_ws` even strips them from borrowed
  source titles. This is verified after every change (see commands below).
- **No LLM, no API keys, zero cost.** Every source is keyless and fail-soft.
  Adding a dependency on a paid or keyed service breaks the project's premise.
- **Never commit the paid edge in plaintext.** `build/` is gitignored. The
  embargoed payload only leaves the machine encrypted (`docs/edge/`) or via KV.
- **Never commit secrets.** Stripe keys, cookie secrets, etc. live only in
  GitHub Secrets / wrangler secrets. The live Stripe key that was once pasted in
  chat must stay rolled and unused.
- **The template is embedded at compile time** via `include_str!`. Editing
  `templates/index.html` requires a `cargo build` before the change appears in
  output. This bites often.
- **State files compound.** `data/{predictions,pulse,genome,weights,corpus}.json`
  must be committed by the Action every run or the memory and learning reset.

### Verify after any change

```
# from C:\tech-oracle, PowerShell
$env:CARGO_HOME="C:\tech-oracle\.cargo-home"; $env:CARGO_TARGET_DIR="C:\tech-oracle\target"
cargo test --release        # 41 unit tests must stay green
cargo build --release
.\target\release\tech-oracle.exe
# em-dash / emoji must be zero:
$h = Get-Content docs/index.html -Raw
([regex]::Matches($h, [char]0x2014)).Count   # em dashes
([regex]::Matches($h, '[\uD800-\uDBFF][\uDC00-\uDFFF]')).Count  # emoji
```

The full verification protocol (artifact validation, idempotency, the JS console
sweep, crypto interop) is in `06-testing.md`.
