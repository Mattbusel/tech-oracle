# 06: Testing and hardening

How the codebase is verified, what is covered, and the exact commands to
reproduce every check. The goal: the core function (a correct daily print) is
provable, not hoped for.

There are four layers of verification: Rust unit tests, artifact validation,
idempotency, and a client-JS console sweep, plus a crypto interop check.

---

## 1. Rust unit tests (`cargo test`)

Every core module has a `#[cfg(test)]` companion file wired in with
`#[path = "tests_<module>.rs"] mod ...;` at the end of the module (so the tests
can reach private items via `super::*`). The binary-crate tests for `main.rs`
live in an inline `mod tests` at the bottom of `main.rs`.

Run:
```
$env:CARGO_HOME="C:\tech-oracle\.cargo-home"; $env:CARGO_TARGET_DIR="C:\tech-oracle\target"
cargo test --release
```
Current state: **45 tests, all passing.**

What each file covers:
- `main.rs` (inline) - the grader `resolve_open` for every market (RESURFACE
  hit/miss/expired, the never-settle-same-day guard, CHASM general-corpus-only,
  INDEX, OVER, HEAD-TO-HEAD both ways, CROSSOVER), `fallback_call` shape,
  `compute_weights` thresholds and bounds, `ghash` determinism, `age_days` /
  `reveal_date`, `csv_escape`, `dedup`.
- `tests_observatory.rs` - the stage funnel and technical/general bands, term
  counts and velocity, chasm-crossing detection, the rationale tape, fear/greed
  bands.
- `tests_rank.rs` - dedup and cap, learned weights changing the order, empty input.
- `tests_generate.rs` - `pick_keyword`, `challenge_template`, and that every
  generated call across many seeds is well-formed (valid market, win condition,
  confidence in 0.34..0.9, resolves_by after the call date).
- `tests_bloodline.rs` - `simulate` determinism, gene sensitivity and a full
  stat line, a fader losing to an accurate oracle, selectivity skipping marginal
  calls, house assignment by temperament, neutral champion of an empty
  population, crossover staying in gene bounds (all six genes), **evolve
  idempotency per day** (via `evolve_in_memory`, the IO-free core), and the Hall
  of Fame inducting and capping across generations.
- `tests_access.rs` - code normalization, sha256hex stability, and that
  `encrypt` emits a parseable, base64 envelope.
- `tests_card.rs` - the 5x7 font covers A-Z and 0-9, unknown chars render blank,
  the ASCII banner is seven rows.
- `tests_render.rs` - `day_diff`, `hsl_hex` primaries, `slug` / `xml` / `enc`,
  `clip_r`, `wrap_chars`.

Note: tests touch only pure logic and in-memory state. Functions that do file IO
(`observatory::build`, `bloodline::save`, `build_genome`) are not exercised by
the unit tests so they cannot clobber `data/`; their logic is covered through the
IO-free pieces (e.g. `evolve_in_memory`).

---

## 2. Artifact validation (after a real run)

Build, run, then validate every generated file:
```
cargo build --release
.\target\release\tech-oracle.exe
```
Checks (all must pass):
- Every `docs/**/*.json` parses (api/*, .well-known/*, dataset/*, openapi).
- `docs/dataset/predictions.jsonl` parses line by line.
- `docs/feed.xml`, `docs/sitemap.xml`, `docs/sitemap-images.xml` are valid XML.
- Every `docs/**/*.html` (35 pages) has zero unrendered template tags (`{{` /
  `{%`), zero em or en dashes, zero emoji.

Last run: 11 JSON + JSONL + 3 XML + 35 HTML, **0 failures**.

---

## 3. Idempotency (backstop crons must be safe)

The daily Action fires several times a day (backstops). The whole run must be
safe to repeat within a day. Verified by running the binary twice and confirming:
- the genome generation does not advance twice (date guard),
- the bloodline generation and population do not advance twice (the
  `last_evolved` guard),
- today's call count is stable and there are no duplicate `(date, text)` calls.

(Call *content* can differ between runs because sources are fetched live; that is
expected, not a regression. The structural guards are what must hold.)

---

## 4. Client-JS console sweep

Every page type is loaded headlessly and Chrome's console is scanned for uncaught
exceptions / undefined refs / type errors:
```
chrome --headless=new --enable-logging=stderr --v=1 --dump-dom file:///.../docs/<page> 2>err
# grep err for: Uncaught | is not defined | is not a function | TypeError | ReferenceError
```
Swept: index, arena, sleep, bloodline, receipts, dataset, amplify, a call page, a
topic page. Last run: **all clean.** (Network/CORS failures from `file://` are
expected and are caught in code; they are not uncaught errors.)

---

## 5. Crypto interop (the premium unlock)

The edge files are encrypted by Rust (`access.rs`) and decrypted by the browser
(WebCrypto). Verified end to end with a Node WebCrypto decryptor
(`build/rt.js`, gitignored): set `ACCESS_CODES` and a positive
`REVEAL_DELAY_DAYS`, run, then decrypt `docs/edge/<hash>.json` with the code and
confirm it yields the early payload. Last run decrypted cleanly (PBKDF2-HMAC-
SHA256 100k -> AES-256-GCM, ct = ciphertext||tag).

---

## When you change code

- Changing the grader, a market, ranking, generation, the observatory math, the
  bloodline, the crypto, the font, or a render helper: add or extend the matching
  `tests_*.rs` (or the inline `main.rs` tests) and keep `cargo test` green.
- Changing the template or any render output: re-run the artifact validation
  (Section 2) and the console sweep (Section 4); the em-dash/emoji/unrendered-tag
  counts must stay zero.
- Changing anything that runs in the daily Action: re-check idempotency (Section
  3) since backstop crons re-run the day.
