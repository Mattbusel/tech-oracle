# 05: Conventions and glossary

Quick reference for the rules, the domain vocabulary, and the commands.

---

## House style (enforced)

- **No em dashes, no en dashes, no emojis. Anywhere.** Site, generated text,
  social posts, share cards, commit-visible output. `fetch.rs::collapse_ws`
  strips them from borrowed source titles; verify zero after every change.
- Voice: terse, dot-matrix, "continuous-form printout", betting-den. "No edits,
  no deletes, only prints." Calls are dated and falsifiable.
- ASCII only in UI affordances (use `->`, `[ X ]`, `//`, not arrows/glyphs).

## Architectural rules

- No LLM, no API keys, zero recurring cost. Every source keyless and fail-soft.
- Deterministic for a given (date, signals): the date seeds template rotation,
  market rotation, genome mutation, and the ask-oracle fortune.
- Append-only public record: calls are never edited or deleted, only graded.
- Static in the hot path: the daily binary writes files; the optional Worker is
  only for the premium account flow.
- State compounds: `data/*.json` is committed every run.

---

## The markets (a call's bet type and how it settles)

Defined in `generate.rs::MARKETS`, graded in `main.rs::resolve_open`.

| Market | The bet | Settles HIT when (before `resolves_by`) |
| --- | --- | --- |
| RESURFACE | the subject comes back | keyword appears in the full corpus |
| SURVIVAL | it doesn't go quiet | keyword resurfaces (same check, framed as survival) |
| MOMENTUM | it keeps moving | keyword resurfaces |
| HEAD-TO-HEAD | it beats a rival | keyword appears and keyword2 does not |
| CROSSOVER | it out-mentions a rival | count(keyword) > count(keyword2) |
| INDEX | the field heats up | acceleration index >= target |
| OVER | a mention surge | count(keyword) >= target in a day |
| CHASM | it leaves the dev bubble | keyword appears in the general-public corpus (reddit/ars/news/wiki) |
| FUTURES | it still matters long term | keyword resurfaces, 90-day horizon |
| LONGSHOT | a high-odds resurface | keyword resurfaces (low confidence -> long odds) |

Any market MISSes once `resolves_by` passes without the condition. CHASM and
crossing terms get the provocative `challenge_template` text.

## Other domain terms

- **Signal**: one normalized source item (`model::Signal`).
- **Call / Prediction**: a dated bet (`model::Prediction`).
- **The Pulse / Acceleration Index**: 0-100 daily heat from volume, breadth, and
  the dominant cluster's share (`build_pulse`).
- **The Genome**: persisted DNA (`gen, hue, wear, quirk`) mutated once/day.
- **The Mood**: the daily look derived from genome + state (heat, agit, wear,
  accent, quirk, age, model, hotHand, tagline). Drives shaders and `--ac`.
- **Quirks**: rare genome mutations -> palette overrides: 1 BLOOD MOON, 2 BLUE
  SHIFT, 3 STATIC STORM, 4 GOLD RUSH, 5 GHOST SHIFT.
- **The Observatory / Corpus**: the growing discourse time series and the quant
  views built on it.
- **The Chasm**: the point a term born technical reaches a general-public source.
  The CHASM market and the chasm watch track it.
- **Diffusion funnel / stage**: sources ordered technical(0) to general(9); see
  `observatory::stage_of`.
- **Sectors**: AI, CHIPS, RUST, CRYPTO, CLOUD, SECURITY, ROBOTS, PLATFORM
  (lexicon in `observatory::SECTORS`).
- **Fear / Greed**: lexicon sentiment 0-100 over the day's headlines.
- **The Book**: the engine's own flat-stake virtual bankroll on its calls (line
  = 1/confidence), computed in `render.rs`.
- **Calibration / Brier**: how well stated confidence matches realized hit rate.
- **Source weights**: learned per-source multipliers from realized hit rate
  (`data/weights.json`), fed back into ranking.
- **The Receipts**: the dated "we called it first" wall (`receipts.html`).
- **Press Credential**: the passwordless, emailless account (three WORDS + hex).
- **The Edge / Early feed**: embargoed calls subscribers see before reveal.
- **Access code / god pass**: a code that unlocks the encrypted early feed
  client-side (`access.rs` + `docs/edge/`).
- **Presence / ghosts**: live multiplayer cursors and chip-drops over MQTT.
- **Mortality / Vitality**: the book is the den's life force. `render.rs` derives
  `vitality` (0..1 from the bankroll) and `lifeState` (ALIVE / FADING / FLATLINE
  / DEAD). The world dims (`html.fading`) as it bleeds and shows a death screen
  (`html.dead` -> `#flatline`) at zero.
- **The Bloodline / self-evolution**: the oracle is a *breeding population* of
  gambler-organisms (`bloodline.rs`, `data/bloodline.json`), not one strategy.
  Each organism has strategy genes (`aggr` line aggressiveness, `risk` longshot
  appetite, `conf` bias). Daily, every organism shadow-bets the entire settled
  record (`simulate`); the richest survive, the broke ones die, survivors mate
  (`crossover` + mutation) to refill the population. The fittest living organism
  (the **champion**) drives `generate.rs`'s real line. A genuine genetic
  algorithm. Organisms split into emergent **houses** by temperament (THE
  PLUNGERS / THE STEADY / THE MISERS, by `risk`). Shown at `/bloodline.html` as a
  LIVE broadcast: animated standings, the house race, a births/deaths wire,
  rolling commentary, and a LISTEN voice (Web Speech). Surfaced on the main page
  in the Engine Room and on the scorecard. (This replaced the earlier
  single-genome hill-climb.)
- **Sleep mode / The Dreams**: a destination (`/sleep.html`), not a takeover.
  `main::build_dreams` exposes the corpus's most-burned-in terms (`pool`) and the
  `forms`; `sleep.html` recombines them into surreal far-future calls forever,
  client-side (always running, self-updating). Seed dreams + pool + forms are also
  at `api/dreams.json`.
- **The Arena**: a serverless prediction tournament (`arena.html`). Bets are
  GitHub issues labeled `arena` with a `SIGNAL-BET` line; the board settles them
  against `api/record.json` and ranks all players against the machine and the
  anti-oracle. The Press Credential carries a rap-sheet rank/title earned there
  and in the pit.

---

## Commands

Local build + run (PowerShell, from `C:\tech-oracle`):
```
$env:CARGO_HOME="C:\tech-oracle\.cargo-home"; $env:CARGO_TARGET_DIR="C:\tech-oracle\target"
cargo build --release
.\target\release\tech-oracle.exe
```

Verify house style (must be zero):
```
$h = Get-Content docs/index.html -Raw
([regex]::Matches($h, [char]0x2014)).Count                       # em dashes
([regex]::Matches($h, [char]0x2013)).Count                       # en dashes
([regex]::Matches($h, '[\uD800-\uDBFF][\uDC00-\uDFFF]')).Count   # emoji
([regex]::Matches($h, '\{\{|\{%')).Count                         # unrendered template tags
```

Rebuild the WASM particle engine (when `wasm/src/lib.rs` changes):
```
# uses wasm32-unknown-unknown; forward-slash target dir avoids path mangling
cargo build --release --target wasm32-unknown-unknown --manifest-path wasm/Cargo.toml
# then copy the produced .wasm to docs/signal.wasm
```

Screenshot a page for visual review (headless Chrome; isolate tall sections):
```
& "C:\Program Files\Google\Chrome\Application\chrome.exe" --headless=new --disable-gpu `
  --window-size=1280,900 --screenshot="_shot.png" "file:///C:/tech-oracle/docs/index.html"
```
(`_*.png`/`_*.html` are gitignored scratch. `--virtual-time-budget` cuts live
websockets, so it cannot prove presence; use two real instances + the DevTools
protocol for that.)

Git after a change (the Action may have advanced main):
```
git add -A
git commit -F build/commit_msg.txt        # avoids PowerShell here-string pitfalls
git fetch origin; git merge -X ours origin/main -m "merge"; git push
```

---

## Where to change what (cheat sheet)

- New source: `fetch.rs` (+spawn in `main`), `observatory::stage_of`,
  `main::source_label`, template `labels` map, CSS `.s-x`/`i.s-x`.
- New market: `generate.rs` (MARKETS + win_if + selection guard),
  `main::resolve_open` (grading), and `api/oracle.json` markets glossary.
- New `Prediction` field: see the schema-change checklist in
  `03-data-and-formats.md`.
- New page section / panel: `render.rs` (compute + add to context) and
  `templates/index.html` (markup + CSS), then rebuild.
- Visual evolution: `main::build_genome` (DNA) and `render.rs` mood + template
  shader uniforms / `--ac`.
- Any logic change: add or extend the matching `tests_*.rs` (or the inline
  `main.rs` tests) and keep `cargo test --release` green. Template/output changes:
  re-run the artifact validation and console sweep in `06-testing.md`. The
  em-dash / emoji / unrendered-tag counts must stay zero.
