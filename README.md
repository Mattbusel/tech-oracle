# THE SIGNAL (tech-oracle)

**A self-updating, self-grading tech-prediction site built by one Rust binary and a GitHub Actions cron. No LLM, no API keys, no server.**

Live: **https://mattbusel.github.io/tech-oracle/**

Most prediction content is vague and never checked. THE SIGNAL makes specific, dated calls with machine-checkable win conditions, then grades every one of them HIT or MISS against later data and keeps the whole record public, misses included. Everything is rules and arithmetic over free public feeds, so it costs nothing to run and every call can be traced back to the numbers that produced it.

<!--SIGNAL:START-->
## Today on THE SIGNAL

**September 26, 2026** // Index **72 (SURGING)** // hottest **GOOGLE** // record **599-1263**

> What If Your AI Agent Never Had to Leave the Browser? (Demo ) is trending with practitioners on dev.to. The signal says it hits job postings as a required skill within a year.

Live: https://mattbusel.github.io/tech-oracle/ // Watch this repo for the daily dispatch.
<!--SIGNAL:END-->

## What it does

Once a day the binary:

1. **Fetches** ten keyless, fail-soft sources concurrently: Hacker News, GitHub Trending, Lobsters, Dev.to, Ars Technica, Reddit r/technology, Google News (tech), Wikipedia pageviews, crates.io, and arXiv. A dead source is logged and skipped.
2. **Measures** the discourse: a growing term corpus, velocity, diffusion from niche to mainstream sources, sectors, a 0 to 100 acceleration index and a fear/greed reading.
3. **Selects and generates** a few calls. Momentum is normalized per source, weighted by learned per-source hit rates, deduplicated by topic, and turned into a `Prediction` with a keyword, a `win_if` condition, a `resolves_by` horizon and a rationale, using date-seeded templates so a re-run on the same day is deterministic.
4. **Grades** open calls against the corpus time series and updates the per-source weights from the results (simple online learning).
5. **Renders** the static site into `docs/`: the main page, a permalink and PNG share card per call, topic pages, a receipts page, RSS, sitemaps, `llms.txt`, an iCal feed, an embeddable widget, a README badge, an agent API (`docs/api/*.json` plus `openapi.json`) and an open CC-BY dataset (`docs/dataset/`: CSV, JSONL, Frictionless and Croissant metadata).
6. **Commits** the public output and state files back to the repo; GitHub Pages redeploys.

Try it from a terminal:

```bash
curl https://mattbusel.github.io/tech-oracle/cli
curl https://mattbusel.github.io/tech-oracle/api/today.json
```

## Run your own

### Download

Get the latest build from
[GitHub Releases](https://github.com/Mattbusel/tech-oracle/releases/latest)
and pick the file for your computer:

| You have | Download |
| --- | --- |
| Windows 10 or 11 | `tech-oracle-vX.Y.Z-x86_64-pc-windows-msvc.zip` |
| Mac with Apple Silicon (M1 and later) | `tech-oracle-vX.Y.Z-aarch64-apple-darwin.tar.gz` |
| Mac with an Intel chip | `tech-oracle-vX.Y.Z-x86_64-apple-darwin.tar.gz` |
| Linux, 64-bit | `tech-oracle-vX.Y.Z-x86_64-unknown-linux-gnu.tar.gz` |

Unzip it, put `tech-oracle` (`tech-oracle.exe` on Windows) in an empty folder
and run it there. It fetches today's public signals, makes and grades its
calls, and writes `data/`, `build/` and `docs/` into that folder. Open
`docs/index.html` to see your site. Run it again each day (a scheduled task or
cron job works) and it keeps grading itself. `tech-oracle --help` lists the
other commands.

The downloads are not signed, so your computer will be cautious the first
time. On Windows, SmartScreen may say "unknown publisher": click **More info**,
then **Run anyway**. On a Mac, right-click the program and choose **Open**.
`SHA256SUMS.txt` on the release page lists every file's checksum.

### With Cargo

```bash
cargo install tech-oracle
```

### From source

See the quick start below.

## Quick start

Requires a stable Rust toolchain. No keys are needed to run it.

```bash
git clone https://github.com/Mattbusel/tech-oracle
cd tech-oracle
REVEAL_DELAY_DAYS=0 cargo run --release     # PowerShell: $env:REVEAL_DELAY_DAYS=0; cargo run --release
cargo test --release
```

A run fetches live signals, updates `data/*.json`, writes `build/early_payload.json`, and regenerates `docs/`. Open `docs/index.html` in a browser. With `REVEAL_DELAY_DAYS=0` every new call is public immediately; the default in the binary is 1 day.

Other subcommands: `tech-oracle backfill [days]` rebuilds history, `tech-oracle harvest` reads leaderboard submissions from GitHub issues.

`templates/index.html` is embedded at compile time with `include_str!`, so template edits need a rebuild before they show up.

## Architecture

| Layer | Tool | Role |
| --- | --- | --- |
| Engine | Rust, one binary | fetch, measure, rank, generate, grade, render |
| Templating | minijinja | the single-page app template, embedded at build time |
| Share cards | `png` crate, hand-coded 5x7 font | dot-matrix PNG cards and the ASCII banner |
| Particles | Rust to `wasm32` (`wasm/`) | the page's background effect, `docs/signal.wasm` |
| Automation | GitHub Actions (`daily.yml`) | build, run, commit, syndicate, IndexNow ping |
| Paid gate (optional) | Cloudflare Worker + KV (`worker/`) | Stripe-verified early feed |

```
src/
  main.rs          orchestration: fetch -> measure -> generate -> grade -> persist -> render
  fetch.rs         the ten source fetchers
  rank.rs          cross-source normalization and weighted selection
  generate.rs      rules and templates -> dated calls with win conditions
  observatory.rs   corpus, velocity, diffusion, sectors, fear/greed
  manifold.rs      trajectory regime and confidence model
  render.rs        the site and every generated artifact
  card.rs          PNG share cards
  access.rs        AES-GCM encryption of the early feed per access code
  tests_*.rs       unit tests per module
templates/index.html   the whole front end (HTML, CSS, JS)
wasm/                  no_std particle engine
worker/                optional Cloudflare Worker
data/                  committed state that compounds daily (predictions, pulse, genome, weights, corpus, bloodline)
docs/                  the published site (mostly generated)
dev-docs/              full developer reference, not published
```

Full module-by-module documentation, data schemas, ops and testing are in [`dev-docs/`](dev-docs/README.md).

## Free record, paid head start

Every call starts in an embargoed pool. Calls older than `REVEAL_DELAY_DAYS` move into the public record (`data/predictions.json`); the rest are written to `build/early_payload.json`, which is gitignored and never committed in plaintext. The early feed reaches subscribers one of two ways:

- **Access codes (no server).** Set the repo secret `ACCESS_CODES` to a list of codes. Each build encrypts the early feed once per code (PBKDF2 to AES-256-GCM) into `docs/edge/<sha256(code)>.json`; the browser decrypts it when the user enters the code. Revocation takes effect on the next daily build.
- **Cloudflare Worker + Stripe (optional).** `worker/worker.js` verifies Stripe webhooks and Checkout sessions, issues a passwordless "press credential" derived from the Stripe customer id, and serves the early payload from KV. Setup:

  ```sh
  cd worker
  wrangler kv namespace create EDGE          # paste the id into wrangler.toml
  wrangler secret put STRIPE_SECRET_KEY
  wrangler secret put STRIPE_WEBHOOK_SECRET
  wrangler secret put COOKIE_SECRET
  wrangler deploy
  ```

  Then point a Stripe webhook at `https://<worker>/webhook` for `customer.subscription.*` and `invoice.payment_failed`, and set the repo secrets `CF_API_TOKEN`, `CF_ACCOUNT_ID`, `CF_KV_NAMESPACE_ID` so the Action syncs the payload to KV. Without them the KV steps are skipped.

The public workflow currently runs with `REVEAL_DELAY_DAYS: "0"`, so the live site shows every call the day it is made.

## Configuration

| Setting | Where | Purpose |
| --- | --- | --- |
| `REVEAL_DELAY_DAYS` | workflow env / shell | days the public page trails the early feed (binary default 1, workflow sets 0) |
| `SITE_URL` | repo variable | absolute URLs for feeds, cards and links |
| `STRIPE_PAYMENT_LINK`, `STRIPE_PORTAL_URL`, `EARLY_ACCESS_URL` | repo variables | subscribe, manage and sign-in buttons |
| `ACCESS_CODES` | repo secret | codes that unlock the encrypted early feed |
| `CF_API_TOKEN`, `CF_ACCOUNT_ID`, `CF_KV_NAMESPACE_ID` | repo secrets | KV sync for the Worker |
| `DISCORD_WEBHOOK_URL`, `TELEGRAM_BOT_TOKEN` + `TELEGRAM_CHAT_ID`, `MASTODON_BASE` + `MASTODON_TOKEN`, `BLUESKY_HANDLE` + `BLUESKY_APP_PASSWORD` | secrets / variables | optional daily syndication; unset channels are skipped |

The cron fires four times a day (13:17, 17:17, 21:17 and 01:47 UTC) because GitHub's scheduler is best effort. The run is idempotent per calendar day, so only the first one to land changes anything. Scheduled runs also post the day's call as a GitHub issue labeled `dispatch`, so watching the repo works as a newsletter, and they refresh the "Today" block above.

## Distribution built in

RSS (`docs/feed.xml`), sitemaps and JSON-LD, per-call and per-topic pages, IndexNow pings, `llms.txt`, an embeddable widget, a daily SVG badge and the curl banner are all generated by the same run:

```html
<script src="https://mattbusel.github.io/tech-oracle/widget.js" async></script>
```

```md
[![THE SIGNAL](https://mattbusel.github.io/tech-oracle/badge.svg)](https://mattbusel.github.io/tech-oracle/)
```

## Notes

- The "Today" block between the `SIGNAL` markers in this README is rewritten by the daily run. Edit around it, not inside it.
- Not financial advice. Calls are generated by rules from public signals, and the public record shows how often they miss.
