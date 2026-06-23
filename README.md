# SIGNAL · ORACLE

A self-updating tech-prediction site. Every day it fetches free signal feeds,
generates one or two specific, dated, confident calls **by rules** (no LLM, no
API keys), and publishes them to a static page with a growing public track
record. A GitHub Action runs it on a cron; GitHub Pages serves the result.

**The business:** the product's value is being *early*. Subscribers see each
call immediately; the public page reveals it a configurable number of days
later. The free archive is the proof the calls are good (marketing); the paid
tier sells the head start.

```
free public page  ──►  proves it works (full dated track record)
paid subscribers   ──►  same calls, N days sooner (the edge)
```

Nothing here costs money to run except (optionally) a Cloudflare free-tier
Worker for the paid gate. No server you operate, no database you run.

---

<!--SIGNAL:START-->
## Today on THE SIGNAL

**June 23, 2026** // Index **68 (SURGING)** // hottest **PRIME** // record **21-2**

> Watching What we call "age verification" is actually mass surveillance climb HN. This is the kind of thing that quietly becomes table-stakes infrastructure within six months.

Live: https://mattbusel.github.io/tech-oracle/ // Watch this repo for the daily dispatch.
<!--SIGNAL:END-->

## Architecture (polyglot — right tool per job)

| Layer | Language / tool | Why |
|---|---|---|
| Core engine | **Rust** (one binary) | Fetch, parse, rank, dedup, generate, render. Static binary, no runtime to install on CI. Threads for concurrent fail-soft fetches. |
| Templating | **minijinja** (Jinja2) | Template stays an editable `.html` file but is embedded via `include_str!` at compile time → still one self-contained binary. Runtime render keeps iteration easy; no askama compile-macro constraints. |
| CI / glue | **Bash** in GitHub Actions | Build, run, KV sync, commit-back. |
| Paid gate | **JavaScript** on Cloudflare Workers + KV | The only backend. Stripe-verified, free tier. |

### Why two outputs, and why the split persistence

The Rust core is **payment-agnostic**. It produces:

1. `data/predictions.json` — the **revealed** archive. Committed, public,
   renders `docs/index.html`.
2. `build/early_payload.json` — the **embargoed** calls (the edge). Gitignored,
   pushed to Cloudflare KV, served only to subscribers.

The repo is public (GitHub Pages requires it), so **embargoed call text must
never be committed.** That's why the still-secret calls live in KV, not in the
repo. Each daily run:

- pulls the embargoed pool from KV → `build/embargoed_in.json`,
- generates today's call (starts embargoed),
- **promotes** any embargoed call older than `REVEAL_DELAY_DAYS` into the public
  archive,
- writes the new embargoed pool back to KV and commits only the revealed files.

`generate()` sits behind a single function so a later **rules → model** upgrade
is a swap, not a rewrite.

---

## Signal sources (all free, no auth, fetched concurrently, fail soft)

- **Hacker News** Firebase API — top ~30 stories, ranked by `score`.
- **arXiv** Atom feed (`cs.AI` + `cs.LG`, newest first) — ranked by recency.
- **GitHub Trending** (HTML scrape) — ranked by stars-today.

A dead source is logged and skipped; the run still produces a page from the
survivors. Cross-source ranking normalizes each signal against the max within
its own source, dedups near-identical topics (Jaccard > 0.6), and picks the top
1–2 distinct items. Template variants rotate by a **date seed**, so the same day
+ same signals is deterministic and reproducible.

---

## Repository layout

```
src/            Rust core: fetch, rank, generate, render, main (orchestration)
templates/      index.html (minijinja template, embedded at build time)
data/           predictions.json — revealed, committed archive (seeded)
docs/           GitHub Pages output (index.html written here; has .nojekyll)
worker/         Cloudflare Worker (paid gate) + wrangler.toml
.github/workflows/daily.yml   cron + manual automation
build/          gitignored scratch: embargoed_in.json / early_payload.json
```

---

## Quick start

### 1. Build & run locally (no keys needed)

```sh
cargo run --release
```

Reads `data/predictions.json`, fetches signals, writes the updated archive,
`build/early_payload.json`, and `docs/index.html`. Open `docs/index.html` in a
browser.

> On Windows the binary is `target\release\tech-oracle.exe`.

**Tip:** with `REVEAL_DELAY_DAYS=0` there's no embargo — every fresh call goes
straight to the public page (acts like a free blog). Useful for seeing your own
call render immediately:

```sh
REVEAL_DELAY_DAYS=0 cargo run --release      # bash
$env:REVEAL_DELAY_DAYS=0; cargo run --release   # PowerShell
```

### 2. Create the repo & enable Pages

1. Push this repo to GitHub.
2. **Settings → Pages → Source: Deploy from a branch**, branch `main`, folder
   **`/docs`**. Save. Your site is at `https://<user>.github.io/<repo>/`.

### 3. Schedule

The cron lives in `.github/workflows/daily.yml`:

```yaml
schedule:
  - cron: "17 13 * * *"   # 13:17 UTC — edit this line
```

Run it by hand anytime: **Actions → daily-prediction → Run workflow**
(`workflow_dispatch`).

---

## Configuration

| Setting | Where | Default | Meaning |
|---|---|---|---|
| `REVEAL_DELAY_DAYS` | workflow `env` / shell | `1` | Days the public reveal trails the subscriber feed. `0` = no embargo. |
| `STRIPE_PAYMENT_LINK` | repo **Variable** | `#` | Subscribe button target. |
| `STRIPE_PORTAL_URL` | repo **Variable** | `#` | Stripe Customer Portal (cancel/manage). |
| `EARLY_ACCESS_URL` | repo **Variable** | `#` | Worker URL (subscriber sign-in). |
| `CF_API_TOKEN` / `CF_ACCOUNT_ID` / `CF_KV_NAMESPACE_ID` | repo **Secrets** | — | KV sync. If unset, KV steps no-op (manual fallback). |

Repo **Variables** (Settings → Secrets and variables → Actions → Variables) are
for public URLs; **Secrets** are for the Cloudflare token.

---

## Accounts + the paid tier

### Accounts are a PRESS CREDENTIAL, and they need no server

The account is a credential like `RIBBON-COPPER-VECTOR-7F3A`. There is no email,
username, or password.

- **Free accounts are 100% client-side.** "Get a free press credential" mints a
  high-entropy credential in the browser with `crypto.getRandomValues` and stores
  it in `localStorage`. Nothing is sent anywhere. The credential is the identity.
- **Premium (early feed) is unlocked by ACCESS CODES, no server.** The engine
  (the Rust core, in its normal daily build) encrypts today's embargoed payload
  once per code in `ACCESS_CODES` with a key derived from that code (PBKDF2 →
  AES-256-GCM) and writes `docs/edge/<sha256(code)>.json`. The file is committed
  to the public repo but is **ciphertext** — only someone with the code can
  decrypt it in the browser. No Cloudflare, no database, no extra scripts.

### Access codes / god passes (premium that just works)

A premium code (a "god pass") is simply a string the engine encrypts the early
feed for. Set a repo **Secret** `ACCESS_CODES` to a comma/newline-separated list,
e.g. `GOLDEN-TICKET-9X2, APRIL-FRIENDS`. On the next daily build each code gets
its encrypted feed published.

Give someone a code; they click **EARLY FEED**, type it, and it decrypts — full
premium, instantly, no payment and no account needed. Revoke by removing the code
from `ACCESS_CODES` (its file stops publishing on the next run). Codes live in a
Secret, never exposed publicly. A code may contain spaces (normalized to hyphens),
so `GOLDEN TICKET` and `golden-ticket` resolve to the same thing.

This is also how you grant **paying subscribers** access for now: give each a code
(or one shared code you rotate). It works with no Stripe and no Cloudflare at all —
`ACCESS_CODES` is the only thing to set.

Tradeoffs: revocation is daily (not instant), and anyone a code is forwarded to can
use it (rotate codes for a clean cut). Fine for a daily $15 edge. The Cloudflare
Worker below remains an **optional** alternative for real-time, per-user gating.

## The paid tier (optional Cloudflare Worker alternative)

### Gating approach chosen: serverless function (Cloudflare Worker + KV)

The addendum offered two paths; this ships the **clean** one. The Worker is the
front door for subscribers: it verifies Stripe subscription state and serves the
early payload from KV. No payment server of our own, no self-hosted database, and
the embargoed content **never** touches the public repo or Pages.

**Rejected alternative — unguessable URL.** Publishing the early payload at a
secret path on Pages is truly zero-server, but the file is still technically
reachable on a public host if the path leaks, and rotating it per billing period
is manual. Fine for a first $10–20/mo "edge"; we went with the Worker because the
gate is real and the cost is still $0 (free tier). The core is unchanged either
way — only the access layer differs, so you can switch without touching Rust.

### The account system: the PRESS CREDENTIAL (no email, no password)

Accounts are deliberately *not* username/email/password. On a successful Stripe
checkout the press **issues a unique credential** — three themed words plus a hex
suffix, e.g. `RIBBON-COPPER-VECTOR-7F3A` — derived deterministically from the
Stripe customer id (`HMAC(customer)` → wordlist). That credential **is** the
account. Subscribers log in by entering it; a signed cookie remembers them after.

- Deterministic, so it can always be **re-issued** from a later checkout or the
  customer portal (no "forgot password" flow needed).
- KV stores `cred:<CREDENTIAL>` → `customer` (for login lookup) and
  `sub:<customer>` → active (from webhooks, for real-time revocation).
- Nothing personal is stored. No card data, no database, no email.

### What the Worker does (`worker/worker.js`)

- `POST /webhook` — verifies the Stripe signature; on `customer.subscription.*` /
  `invoice.payment_failed` writes/deletes `sub:<customer>` in KV (real-time gating).
- `GET /?session_id=...` — back from Checkout: verifies the session, mints the
  credential, stores it, sets a cookie, and **prints the credential** for the user
  to save.
- `GET /?cred=...` — logs in with a credential (KV lookup → active check → cookie).
- `GET /` — cookie → early feed; otherwise the credential gate (enter-credential
  form + subscribe button).
- `GET /logout` — clears the cookie.

We store **no card data** and run **no database** — KV holds only `sub:<customer>`,
`cred:<CREDENTIAL>`, and the daily `edge_payload`.

### Sharing

Every call, the scorecard, the book, and the pulse have **share controls**: native
share sheet (mobile) / X intent (desktop) / copy link, plus **SAVE AS IMAGE**,
which draws an on-brand receipt-style PNG card to canvas client-side (no backend,
no image service) for screenshot-native sharing. Link previews (`og:`/`twitter:`)
are rendered with the live hit rate, bankroll, and index so a pasted URL already
reads like a headline.

### Stripe setup (test mode)

1. **Product + recurring price** in the Stripe Dashboard (test mode).
2. **Payment Link** for that price → set as repo Variable `STRIPE_PAYMENT_LINK`.
   In the Payment Link's *After payment* settings, redirect to
   `https://<your-worker>/?session_id={CHECKOUT_SESSION_ID}`.
3. **Customer Portal**: enable it, copy its link → `STRIPE_PORTAL_URL`.

### Cloudflare setup

```sh
cd worker
wrangler kv namespace create EDGE          # paste the id into wrangler.toml
wrangler secret put STRIPE_SECRET_KEY      # sk_test_...
wrangler secret put STRIPE_WEBHOOK_SECRET  # whsec_... (from the webhook endpoint)
wrangler secret put COOKIE_SECRET          # any long random string
# set PAYMENT_LINK_URL and PORTAL_URL in wrangler.toml [vars]
wrangler deploy
```

Then in Stripe, add a **webhook endpoint** → `https://<your-worker>/webhook`,
subscribe to `customer.subscription.*` and `invoice.payment_failed`, and copy its
signing secret into `STRIPE_WEBHOOK_SECRET`.

Finally, set repo Secrets `CF_API_TOKEN` (KV edit permission), `CF_ACCOUNT_ID`,
`CF_KV_NAMESPACE_ID`, and repo Variable `EARLY_ACCESS_URL = https://<your-worker>/`.

### Test a subscription end-to-end (Stripe test mode)

1. Click **Subscribe** → pay with test card `4242 4242 4242 4242`, any future
   expiry/CVC.
2. You're redirected to the Worker with `?session_id=…`; it confirms the session,
   sets the cookie, and you see **today's embargoed call** — before the public
   page shows it.
3. Trigger the daily Action (`workflow_dispatch`). The public page still only
   shows calls older than `REVEAL_DELAY_DAYS`; today's call stays subscriber-only.
4. Cancel via the Customer Portal. The `customer.subscription.deleted` webhook
   deletes the KV flag → access is revoked by the next visit. (With the cookie
   still present, the KV check is the source of truth.)

### Manual fallback (no Worker / webhooks yet)

If `CF_*` secrets are unset, the KV steps no-op and the site runs as a free,
delayed public page. To sell early access before wiring the Worker, you can
manually share `build/early_payload.json` with paying users and rotate access
each billing period by hand. Document the tradeoff to subscribers; upgrade to the
Worker path when ready (no change to the Rust core).

---

## Distribution (automated, $0, layered)

Every daily run produces a syndication payload and the Action fans it out. Layers,
cheapest/most-automatic first:

1. **RSS feed** (`docs/feed.xml`) — the universal source. Anyone can subscribe, and
   it's the input for free automation tools: pipe it through **IFTTT** or **Zapier**
   (free tiers) to auto-post to almost any network without writing code. One feed,
   many destinations.
2. **SEO** — `sitemap.xml`, JSON-LD structured data, and rich `og:`/`twitter:`
   previews rendered with the live hit rate / bankroll / index, so a pasted link
   reads like a headline and each call is indexable.
3. **Direct auto-posting** from the daily Action (all free APIs, each posts only if
   its secret/var is set — unset channels are silently skipped):

   | Channel | Set (repo Secrets/Variables) |
   |---|---|
   | Discord | secret `DISCORD_WEBHOOK_URL` (Server Settings → Integrations → Webhooks) |
   | Telegram | secrets `TELEGRAM_BOT_TOKEN` (via @BotFather) + `TELEGRAM_CHAT_ID` (your channel) |
   | Mastodon | var `MASTODON_BASE` (e.g. `https://mastodon.social`) + secret `MASTODON_TOKEN` |
   | Bluesky | var `BLUESKY_HANDLE` + secret `BLUESKY_APP_PASSWORD` (Settings → App Passwords) |

   Set `SITE_URL` (repo Variable) to your Pages URL so links and feeds are absolute.

   **Turn it on in ~5 minutes:** the two channels that reach *new* people are
   **Bluesky** and **Mastodon** (their hashtag/discovery feeds) — create a free
   account, grab an app password / token, drop it in repo Secrets/Variables, done.
   Discord/Telegram are for an audience you already have. After that the daily
   build auto-posts a punchy, hashtagged call (a 300-char variant goes to Bluesky)
   with zero further effort. Everything else can be fanned out by piping the RSS
   feed through a free IFTTT/Zapier applet.

4. **Share loop** — every call/stat has share controls and a downloadable on-brand
   PNG card (see Sharing), turning every reader into a distributor.
5. **Programmatic SEO** — every revealed call is also a standalone crawlable page
   at `/call/<n>.html` with its own title/OG tags, all listed in `sitemap.xml`.
   A growing corpus of dated, linkable predictions that strangers find via search.
   No audience required; compounds over time.
   - **Topic pages** (`/topic/<x>.html`) group the archive by subject so the site
     matches real queries ("ai agents predictions"), not just one call's wording.
   - **IndexNow**: each build pings search engines to crawl the new pages now (free,
     no account; the ownership key file is served from the site). Faster cold
     search traffic without waiting for the next natural crawl.
6. **Embeddable wire** — any site/newsletter/README can show today's call with one
   line, and each embed is a backlink:
   ```html
   <script src="https://mattbusel.github.io/tech-oracle/widget.js" async></script>
   ```
   (Optionally drop a `<div id="signal-wire"></div>` where you want it.) Content is
   baked daily; no API/CORS needed because it is a script include.
6b. **Daily share images + a badge.** The engine renders a real dot-matrix PNG
   card server-side (`/og.png`, and `/call/<n>.png` per call) wired into every
   `og:image`, so every shared link unfurls as a branded visual. It also emits a
   daily-updating SVG badge for READMEs/sites (another backlink vector):
   ```md
   [![THE SIGNAL](https://mattbusel.github.io/tech-oracle/badge.svg)](https://mattbusel.github.io/tech-oracle/)
   ```
7. **GitHub-native dispatch** — each cron run posts the day's call as a GitHub
   issue (the free auto-newsletter: every repo watcher is notified) and refreshes
   the README's "Today" block. "Watch this repo" becomes "subscribe to the oracle."
8. **FADE ME challenge links** — every bet encodes into a URL that drops a friend
   into the opposite side. A peer-to-peer viral loop, no server.
9. **The ladder** — top runs are public GitHub issues (`label:ladder`) the site
   reads via the GitHub API. Competition that bootstraps on infra you already have.

10. **Curl-able wire** — `curl https://mattbusel.github.io/tech-oracle/cli` prints
   today's call as a dot-matrix ASCII banner in the terminal (wttr.in-style). Devs
   share `curl` one-liners; it's pure cold acquisition and exactly on brand.
11. **`llms.txt`** — a machine-readable map of the record so AI answer engines can
   find and cite it when people ask about tech predictions.

These layer: SEO + topic pages + IndexNow + the curl wire + the embeddable wire and
badge pull in strangers with no audience; the dispatch and challenge links convert
and retain; the ladder makes them compete.

X/Twitter is intentionally not auto-posted (their write API is no longer free);
use the per-post **share buttons** (X intent) or route the RSS feed through IFTTT.

To add a channel: create its free token, drop it into repo Secrets/Variables, done.
Nothing else changes; the next daily run starts posting there.

## Acceptance checklist

- [x] `cargo run --release` with no keys → updated `data/predictions.json`,
      `build/early_payload.json`, and a valid `docs/index.html`.
- [x] Killing any one source (e.g. block network to GitHub) still yields a valid
      page from the survivors.
- [x] Action runs green on `workflow_dispatch`; the Pages URL updates.
- [x] Test purchase grants early access (sees today's call before the public
      page); cancellation revokes by next cycle; public page always honors the
      delay regardless of subscription state.

## Out of scope (v1)

Email/SMS delivery, login/accounts beyond what Stripe provides, any
continuously-running server.

> Not financial advice. Calls are rules-generated from public signals.
