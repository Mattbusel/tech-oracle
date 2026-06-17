# 02: The frontend (templates/index.html)

The entire client is one file: `templates/index.html`. It is HTML + CSS + all
JavaScript inline, embedded into the binary at compile time via `include_str!`
and rendered with minijinja. **Edit it, then `cargo build`, or your change does
not appear in `docs/index.html`.**

Progressive enhancement throughout: with JS off you get real text and the static
painting; every live/animated layer is additive and fail-soft.

---

## Template plumbing (minijinja)

- Line ~3: `{%- set labels = { "hn": "HACKER NEWS", ... "wiki": "WIKIPEDIA", "crates": "CRATES.IO" } -%}` maps `signal_type` to display labels. Keep in sync with new sources.
- `{{ ... }}` interpolations and `{% for/if %}` blocks pull from the render
  context (see `01-rust-engine.md` render section): `generated_human`, `total`,
  `featured`, `pages`, `record`, `intake`, `pulse`, `scoreboard`, `book`,
  `calibration`, `mood`, `engine`, `jsonld`, `floor_json`, `ladder_repo`,
  `payment_link`, `portal_url`, `early_access_url`, `og_image`, `reveal_delay_days`.
- Three values are injected into JS as globals near the end:
  `var __FLOOR = {{ floor_json | safe }};`, `var __LADREPO = "{{ ladder_repo }}";`,
  `var __MOOD = {{ mood | safe }};`. Immediately after, the accent is applied:
  `--ac`, `--phos`, `--led` are set from `__MOOD.accent` so the whole UI recolors
  daily.

---

## Document structure (body)

```
<svg class="filter-defs">           #ink turbulence/displacement filter
<section id="scene" class="scene">   THE WEBGL DEN (the hero you "enter")
  <canvas id="glcanvas">             WebGL surface
  <div class="scene-grad">           dark surround gradient
  <div id="ghosts">                  live presence: other visitors' cursors/chips
  <div id="den-count">               "IN THE DEN: N"
  <div class="stage-ui" id="stage-ui">   operable overlays composited on the painting
    #ov-wire                         live wire panel (on the painting's screen)
    .ov-station (#ov-clock,#ov-cd,#ov-den)  clock / next-print / den count
    .ov-sheet (#ov-call + TAIL/FADE) the printed call on the painting's sheet
    #ov-tiles                        bet tiles from __FLOOR
  <div class="scene-ui">             #scene-kicker (the GEN/DAY/MODEL readout), #scene-enter

<div class="deck">                   THE PRINTOUT (the paper world)
  .machine                          the printer chrome (.plate, .panel LEDs, .carriage, #machine-status)
  .console
    aside.wing-l                    INTAKE: live HN wire (#wire, #wire-status)
    .center > main.sheet
      header.head                   wordmark (dot-matrix), metaline, barcode, acct-entry buttons
      section.ask                   ASK THE ORACLE (#ask-in, #ask-go, #ask-out)
      section.scoreboard            THE SCORECARD (+ SEE THE RECEIPTS, SHARE)
      section.book                  THE BOOK: the engine's own virtual bankroll
      section.floor                 LIVE PIT: #ticker, #pit-stats, #board (tiles)
      section.ladder-pit            THE LADDER (#ladder, public GitHub-issue runs)
      section.pulse                 ACCELERATION INDEX (#seismo canvas, history bars)
      section.manifest.engine-room  THE ENGINE ROOM (fear/greed, calibration, sectors, movers, chasm, learning)
      section.latest                LATEST PRINT (featured calls, dot-matrix)
      section.manifest              INTAKE MANIFEST (today's signals by source)
      section.coupon                FOUNDING ACCESS (subscribe + credential)
      section.record                THE RECORD (by source, signal map)
      section.ledger                THE LEDGER (every call; entry shows win_if + ENGINE TAPE)
      footer.foot                   record links: receipts, dataset, agent api, calendar, rss, curl, amplify
    aside.wing-r                    STATION: #utc clock, #scope oscilloscope, #countdown, #log

<div id="acct-modal">                press-credential / early-feed modal
<div id="bet-modal">                 place-a-trade modal (#bet-body)
<div id="toast">, <div id="bigstamp"> transient feedback
```

---

## CSS (the <style> block in <head>)

- `:root` palette: `--paper #efede4`, `--ink #1b1a14`, `--soft`, `--stamp
  #b23a2e`, `--led`, `--phos`, `--amber`, `--ac` (the daily accent, overwritten
  by JS), `--mono` (IBM Plex Mono). The whole look is ink-on-paper plus the den.
- Source colors: `.s-hn .s-arxiv .s-github .s-lobsters .s-devto .s-ars .s-reddit
  .s-news .s-wiki .s-crates` (text) and `i.s-* , .sm-dot.s-*` (bar/dot fills).
  Add both when adding a source or its bars are invisible.
- Subsystem styles: `.dm-fallback/.dm-canvas/.ready` (dot-matrix), `.ov-*`
  (overlays), `.ghost/.ghost-chip/.den-count` (presence), `.er-*` (engine room),
  plus the den, machine, pit, book, scoreboard, ledger, modal styles.
- Honors `prefers-reduced-motion` (disables print animation, ghost motion, etc.).

---

## JavaScript subsystems (the IIFEs at the bottom)

Each is a self-contained IIFE, defensively coded, fail-soft. They communicate
through a few `window.__*` globals (catalogued at the end).

### 1. The WebGL den (`#scene`)
- Fragment shader samples the painting (`docs/assets/scene.jpg`) with
  contain-fit, dark surround, breathing zoom, mouse/tilt parallax, chromatic
  aberration, scanlines, grain, vignette, and a cursor lamp.
- **Mood uniforms** `heat, agit, wear, tint` come from `__MOOD`: the den runs
  hotter/calmer, flickers more when agitated, darkens with accumulated wear, and
  takes the daily accent tint.
- Particles: loads `docs/signal.wasm`, calls `init/step`, reads the buffer from
  wasm memory, draws additive points mapped onto the contained image rect; warmth
  and draw-count scale with `__MOOD.heat/embers`.
- Texture `onload` gates drawing (`ready`); a CSS background-image is the static
  fallback. Degrades to the still painting with no WebGL.

### 2. The pit / floor (the betting core)
- State: `bets` (live lines), `W` wallet, `POS` open positions; persisted to
  localStorage `signal_wallet`, `signal_pos`, `signal_snd`.
- Renders `#board` tiles, `#ticker`, `#pit-stats`; lines drift on a timer
  (`move`/`tick`); marked to market live.
- `openBet`/`placeBet`/`cashOut`/`closeBet`; `posPnl`/`exposure`/`openPnl`.
- WebAudio `beep`/`chord`, `toast`, `bigStamp`.
- Challenge links: `challengeShare`, `decChal`, `showIncoming` (reads `?vs=`),
  the head-to-head share mechanic.
- Exposes `window.__openBet(kw)`. Calls `window.__ghostBet` on place (presence).
  Reads/writes `window.__incoming`.

### 3. Activity log + machine events
- `window.__log(msg)` writes to `#log`. Periodic carriage `sweep` and `#machine-status`
  changes; calls `window.__spike` on events.

### 4. Live wire (HN front page)
- Fetches `hacker-news.firebaseio.com` top stories (CORS-ok, no key) into
  `#wire`, every 5 min; updates `#wire-status`; calls `__spike`/`__log`. Fails to
  a clear offline message; the daily print is unaffected.

### 5. Ask the oracle
- Deterministic fortune from a hash of (question + today). Renders a dated
  "prophecy" with odds/confidence; `data-pshare` (X intent / native share) and
  `data-pcard` (via `__signalCard`).

### 6. The ladder
- Reads public GitHub issues labeled `ladder` on `__LADREPO`, parses
  `SIGNAL-RUN pnl=.. streak=.. record=.. by=..` lines, ranks by pnl into
  `#ladder`. `lad-post` opens a prefilled new-issue link (no server). `lad-refresh` reloads.

### 7. `<dot-matrix>` custom element
- `DM` class: re-rasterizes its text into impact dots on a canvas; a print head
  strikes them column by column. `data-size/-pitch/-weight/-fit/-print` attrs.
  Keeps a `.dm-fallback` real-text span for accessibility/no-JS. `boot()` runs
  on `document.fonts.ready`; pointer/key press skips the animation; relayouts on
  resize.

### 8. Accounts (the Press Credential, client-side)
- Credential = three WORDS + a 4-hex suffix, minted in-browser, stored in
  localStorage `signal_cred`. No server for the free tier.
- `viewNew` (mint), `viewEarly`/`doEarly` (unlock), `subscribe`, `pressPass`.
- Early-feed unlock: `sha256(cred)` -> fetch `edge/<hash>.json` ->
  `decryptBlob` (WebCrypto PBKDF2 100k -> AES-GCM), matching `access.rs`
  byte-for-byte. Renders the embargoed calls. `data-acct` / `data-go` / `data-cp`
  / `data-pass` handlers; uses `#acct-modal`.
- `subscribe` appends `client_reference_id=<cred>` to the payment link so Stripe
  ties the credential to the customer (the Worker reads it on return).

### 9. Sharing + PNG cards
- `window.__signalCard(opts, text)` draws an on-brand card to a canvas
  (`card(...)`) and offers download/native-share. Click handlers: `data-share`
  (native share / X intent / copy), `data-card` (image), `data-hero-act`.

### 10. Station / scope / clock (`wing-r`)
- `#utc` live clock, `#countdown` to the next daily print, `#scope` oscilloscope
  canvas and the `#seismo` energy line (pulse). Defines `window.__spike()` and
  `window.__pulseSpike()` that the other subsystems call to kick the scope.

### 11. Stage-UI overlays (compositing onto the painting)
- `R = { wire, station, sheet, tiles }` are rects in percent of the contained
  painting; `place`/`layout` position the overlay panels over the painting's own
  screens/sheet/felt. Hidden below 760px. `#ov-tiles` built from `__FLOOR`.
  `mirror()` copies the clock/countdown/call into the overlay. `data-ovkw` /
  `data-ovbet` clicks call `window.__openBet`.

### 12. Live presence (the ghosts)
- Connects to a public MQTT broker over WebSocket (mqtt.js from a CDN, lazy,
  fail-soft) on topic `thesignal/den/v2`. Broadcasts cursor position (throttled)
  and bet events; renders other visitors as ghost cursors in `#ghosts`, drops
  chips on bets, and updates `#den-count` / `#ov-den`. Identity is the credential
  handle or a random `GHOST-XXXX`. Exposes `window.__ghostBet(kw, side)`.
  Verified two peers see each other; if the broker is unreachable it shows one
  occupant and the den works unchanged. See `04-distribution-and-ops.md` for the
  reliability caveat.

### 13. Genome/mood accent + mortality
- Applies `__MOOD.accent` to `--ac/--phos/--led` and writes the GEN/DAY/MODEL/
  verdict/quirk/HOT HAND/VIT/STRAT readout into `#scene-kicker`.
- Mortality: sets `--vitality` and toggles `html.fading` / `html.dead` from
  `__MOOD.vitality`/`lifeState`, dimming the world as the book bleeds.

### 14. Death screen
- Fills `#flatline` (shown only when `html.dead`): the death notice. Sleep mode
  is NOT here; it is its own destination at `/sleep.html` (see below), never a
  takeover of the main page.

### 15. The arena board (in `arena.html`, not the main page)
- Client-side tournament: fetches GitHub issues labeled `arena` plus
  `api/record.json`, parses `SIGNAL-BET kw=.. market=.. side=.. by=..`, settles
  each, and ranks all players against THE MACHINE and THE ANTI-ORACLE with
  earned titles. "ENTER A BET" opens a prefilled new-issue link. Reuses the
  ladder's GitHub-issues-as-database pattern.

### 16. Reputation (accounts IIFE)
- `rapSheet()` computes a rank/title from the local wallet (net record); shown on
  the Press Credential view, linking to the arena.

### 17b. The bloodline broadcast (in `bloodline.html`) + main-page panel
- `bloodline.html` is a LIVE channel: the day's population is baked in as `var BL`;
  the client renders animated standings with **per-organism stat lines** (win
  rate, streak, ROI, tail/fade), the rival house race, a births-and-deaths wire,
  **THE CARDS** (rendered rookie/pro trading-card PNGs), a gold **HALL OF FAME**
  board, and a rolling commentary that cycles forever with an "ON AIR" indicator.
  A LISTEN button (Web Speech) reads the call aloud. Dark control-room aesthetic.
- The main page shows a bloodline panel in the Engine Room (champion, top five
  living, WATCH LIVE link) and a watch button on the scorecard, both fed from the
  `bloodline` template context.

### 17. Sleep mode (in `sleep.html`, a destination)
- A standalone living dreamscape, reached on purpose (footer link "sleep mode"),
  never auto-opened. The Rust build bakes the term `POOL` and the `FORMS` into the
  page; client JS recombines a new dream every few seconds forever (Math.random),
  streaming the last ~9, with a shifting violet field, twinkling stars, occasional
  larger "deep" dreams, an incrementing DREAM No. counter, and WAKE / DREAM
  FASTER controls. Always running and self-updating.

---

## `window.__*` globals (the bus)

| global | defined in | used by |
| --- | --- | --- |
| `__MOOD` | injected (render `mood`) | shader uniforms, particles, accent, kicker, mortality |
| `__FLOOR` | injected (render `floor_json`) | pit, overlays (#ov-tiles) |
| `__LADREPO` | injected (render `ladder_repo`) | ladder |
| `__spike()` / `__pulseSpike()` | station/scope | log, wire, presence |
| `__openBet(kw)` | pit | overlays, ask, challenge links |
| `__incoming` | pit | challenge (`?vs=`) flow |
| `__log(msg)` | log | wire, presence, machine events |
| `__signalCard(opts,text)` | sharing | ask, accounts, scoreboard, hero |
| `__ghostBet(kw,side)` | presence | pit (on place) |

## Client network calls (all keyless, all fail-soft)
HN Firebase (wire), GitHub issues API (ladder), `edge/<hash>.json` (early feed),
`signal.wasm` (particles), the MQTT broker wss (presence), `mqtt.js` + Google
Fonts CDNs, `assets/scene.jpg` (texture).

## localStorage keys
`signal_cred` (the account), `signal_wallet`, `signal_pos`, `signal_snd`.
