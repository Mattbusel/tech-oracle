# 04: Distribution and ops

How the site ships daily, how it reaches people (and machines), how premium
works, and where every secret lives.

---

## The daily Action (`.github/workflows/daily.yml`)

Job `generate` on `ubuntu-latest`. Trigger: cron `17 13 * * *` (13:17 UTC) plus
manual `workflow_dispatch`. Permissions: `contents: write` (to commit back),
`issues: write` (the dispatch). `concurrency` group prevents overlap.

Steps in order:
1. `checkout`.
2. Install Rust (stable, minimal).
3. Cache cargo + `target` (keyed on Cargo.lock/Cargo.toml).
4. `cargo build --release`.
5. **Pull embargoed pool from KV** -> `build/embargoed_in.json` (if `CF_*`
   secrets set; else skipped, manual fallback uses an empty pool).
6. **Generate**: `./target/release/tech-oracle` (the whole pipeline).
7. **Push edge payload to KV** (`build/early_payload.json` -> `edge_payload`, if
   `CF_*` set).
8. **Daily dispatch** (schedule only): closes old `dispatch`-labeled issues and
   opens a new one from `build/dispatch_*`. The free auto-newsletter to repo
   watchers. Uses `GITHUB_TOKEN`.
9. **IndexNow ping**: POSTs `build/indexnow.json` to api.indexnow.org. Best-effort.
10. **Syndicate**: posts `build/social.txt` (and `social_short.txt` for Bluesky's
    300-char limit) to Discord / Telegram / Mastodon / Bluesky. Each channel only
    fires if its secret/var is present; a failed post never fails the run.
11. **Commit revealed output**: stages exactly
    `docs README.md data/predictions.json data/pulse.json data/genome.json
    data/corpus.json data/weights.json`, commits and pushes if anything changed.
    `build/` is gitignored so the paid edge never lands in the public repo.

The push to `docs/` is what GitHub Pages redeploys. Re-runs are idempotent
(today's calls are dropped and regenerated).

### Env wiring (all optional; absence degrades gracefully)
- Variables (`vars.*`, non-secret): `STRIPE_PAYMENT_LINK`, `STRIPE_PORTAL_URL`,
  `EARLY_ACCESS_URL`, `SITE_URL`, `MASTODON_BASE`, `BLUESKY_HANDLE`.
- Secrets (`secrets.*`): `DISCORD_WEBHOOK_URL`, `TELEGRAM_BOT_TOKEN`,
  `TELEGRAM_CHAT_ID`, `MASTODON_TOKEN`, `BLUESKY_APP_PASSWORD`, `ACCESS_CODES`,
  `CF_API_TOKEN`, `CF_ACCOUNT_ID`, `CF_KV_NAMESPACE_ID`, `GITHUB_TOKEN` (auto).
- `REVEAL_DELAY_DAYS` is set to "1" in the workflow env.

---

## The distribution stack (every channel, all free)

Built into `render.rs`/`main.rs` outputs, fired by the Action or pulled by
others. Layered on purpose: search, social, syndication, embeds, agents.

- **Search / SEO**: per-call pages, topic pages, `sitemap.xml` +
  `sitemap-images.xml`, `robots.txt`, JSON-LD (WebSite + ItemList on the page,
  ClaimReview/Claim per call), IndexNow instant crawl pings.
- **Receipts as the credibility surface**: `receipts.html`, the dated
  "we called it first" wall, linked from the scorecard and footer.
- **Syndication**: Discord, Telegram, Mastodon, Bluesky (from `social.txt`);
  the GitHub-issue dispatch (repo watchers); RSS (`feed.xml`); the `.ics`
  calendar; the daily README block.
- **Embeds / backlinks**: `widget.js` (drop-in wire), `badge.svg` (README badge).
- **Cold visual/terminal**: `og.png` + per-call cards (link unfurls), `/cli`
  (curl-able ASCII printout).
- **Amplify console**: `amplify.html`, one-tap prefilled submit links.
- **GEO / agent-native** (the machine audience): `llms.txt`, the `api/*.json`
  + `openapi.json` + `.well-known/*` interface, and the **open dataset**
  (`docs/dataset/`) designed to land in Hugging Face, Kaggle, data.world and
  Google Dataset Search, so AI answer engines retrieve and cite the oracle.
- **The arena** (`arena.html`): a serverless prediction tournament where humans
  and AI agents bet against the machine by opening GitHub issues labeled `arena`
  (the same issues-as-database pattern as the ladder). The board settles against
  `api/record.json`. Agents enter via the issues API; the format is documented in
  `api/oracle.json`. A two-way engagement surface that pulls in the AI-agent
  audience and turns the record into a competition.
- **Borrow-the-audience thesis**: the engine predicts about topics that already
  have crowds and surfaces at the moment a topic crosses into the general public
  (the CHASM market), framed as a falsifiable challenge that crowd will argue
  with.

### One-time manual steps (then automatic)
- Submit the dataset once to Hugging Face / Kaggle (point at `…/dataset/`); it
  self-updates after.
- Submit the site to IndexNow-participating engines is automatic via the ping;
  Google Search Console verification is the usual one-time add.
- Short-form video / audio channels (if pursued later) need a post step or API
  keys; nothing in the repo does this yet.

---

## Premium: two delivery paths for the same edge

The embargoed early feed (`build/early_payload.json`) reaches subscribers by
either path. Both can run; neither is required for the free site.

### Path A: keyless access codes (the "god pass", default)
`ACCESS_CODES` (comma/newline separated) -> `access::publish` writes one
AES-GCM-encrypted file per code to `docs/edge/<sha256(code)>.json` during the
build. The browser (accounts IIFE) takes a credential/code, derives the same key
(PBKDF2 100k -> AES-GCM) and decrypts. No server, no database. Revocation =
remove the code; its edge file is deleted next run. This is how a code "just
works" with zero plumbing.

### Path B: the Cloudflare Worker (`worker/`)
`worker.js` + `wrangler.toml`. A real account system where the account IS a
Press Credential (no email, no password).
- Routes: `/new` (mint a free credential), `/?cred=...` (log in), `/?session_id=...`
  (return from Stripe -> upgrade the credential to premium), `/` (cookie ->
  early feed / account / gate), `/logout`, `POST /webhook` (Stripe lifecycle).
- KV keys: `acct:<CRED>` (tier+customer), `sub:<customer>` (active),
  `cust:<customer>` -> CRED, `edge_payload` (today's embargoed JSON, pushed by
  the Action).
- Credential format matches the client exactly (the WORDS list + hex suffix).
  Cookies are HMAC-signed (`COOKIE_SECRET`). Stripe webhook signatures are
  verified with a timing-safe compare.
- Secrets (set out-of-band via `wrangler secret put`, never in the repo):
  `STRIPE_SECRET_KEY`, `STRIPE_WEBHOOK_SECRET`, `COOKIE_SECRET`. Public vars in
  `wrangler.toml`: `PAYMENT_LINK_URL`, `PORTAL_URL`, and the KV namespace id.

Stripe linkage: the subscribe button appends `client_reference_id=<credential>`
to the Payment Link; on the checkout-complete return the Worker binds that
credential to the Stripe customer and flips it to premium.

---

## Secrets and safety (non-negotiable)

- Secrets live only in GitHub Secrets and wrangler secrets. Never in the repo,
  never echoed, never in `data/` or `docs/`.
- The live Stripe key once pasted in chat must remain rolled and unused.
- `build/` (the paid edge) is gitignored; the embargoed payload only leaves the
  machine encrypted (`docs/edge/`) or via KV.
- The presence layer uses shared **public** MQTT brokers (`broker.emqx.io`,
  `broker.hivemq.com`). They are free and zero-setup but shared and occasionally
  flaky; presence is fully fail-soft. If reliability ever matters more than the
  zero-cost purity, point the one broker URL in the presence IIFE at a dedicated
  broker.

---

## Operational gotchas

- **Template edits need a rebuild** (`include_str!`). Easy to forget; output
  won't change until you `cargo build`.
- **Local builds**: set `CARGO_HOME` and `CARGO_TARGET_DIR` to local paths (an
  `R:`/network default has caused cache-lock and path errors).
- **Push races**: the Action auto-commits, so a local push can be rejected.
  Recover with `git fetch; git merge -X ours origin/main; git push`.
- **PowerShell here-strings**: the closing `'@` must be at column 0 on its own
  line. For commit messages prefer `git commit -F build/commit_msg.txt`.
- **Wikipedia**: project string is `en.wikipedia`; data lags so the fetcher
  walks back several days. Reddit often 403s from non-residential IPs (fine from
  the runner). Both fail soft.
- **WASM rebuild** is manual: build the `wasm/` crate for
  `wasm32-unknown-unknown` and copy the artifact to `docs/signal.wasm` (use a
  forward-slash `CARGO_TARGET_DIR`). There is a stray
  `wasm/tech-oracle.cargo-home/` left from a build; it can be cleaned.
