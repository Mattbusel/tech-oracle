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

## The paid tier (Stripe + Cloudflare Worker)

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
