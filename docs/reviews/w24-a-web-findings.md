# Wave 24 — Lane A (browser matrix, issue #49) audit findings

- **Auditor:** W24-R8 (review scout, audit lane A).
- **Audited state:** local branch `w24/a` @ `9a13c39` — **NOT pushed** (`origin/w24/a` does not
  exist; `git fetch origin w24/a` → "couldn't find remote ref"). The branch IS reachable in the
  local repo, so the audit covers the full **committed** delta `c339ee8..9a13c39` (6 commits:
  `70ed69c`, `771259c`, `ff6e6aa`, `231598f`, `d1cba99`, `9a13c39`), checked out read-only into
  worktree `wt/w24r8` (branch `w24/r8`). Lane A is still running — **uncommitted work in `wt/w24a`
  was NOT audited** (per scout rules I never entered that worktree).
- **Verdict headline:** P0 fix verified closed; settings regen verified in sync; harness is real,
  config-driven and honest (no stubs/fabricated evidence); Rust touched by the lane is
  fmt/clippy/lint-literals green on native AND wasm32. No P0/P1 findings. Two P2s (both
  "finish-line" items, not defects in committed code) and five P3s.

## What was actually run (evidence for every claim below)

- `npx tsc --noEmit` (ui/) — exit 0.
- `npm run lint` (ui/, includes the new eslint blocks for `e2e/web/site/**`, `scripts/web-matrix-run.mjs`) — exit 0, **0 errors**, 28 warnings, all in files lane A did not touch.
- `npm run build` (ui/) — exit 0 (tsc + vite; only the pre-existing >500 kB chunk warning).
- `npm run test` (vitest) — **718/718 pass, 80 files** — matches ff6e6aa's coherence claim exactly.
- `npm run settings:check` — OK (5 ui areas, 21 ui entries, 32 total).
- `cargo fmt -p playback-engine -- --check` — clean.
- `cargo clippy -p playback-engine --all-targets --locked` — clean (exit 0).
- `cargo check` / `cargo clippy -p lumen-web --target wasm32-unknown-unknown --locked` — clean.
- `cargo test -p playback-engine --lib the_default_policy` — 1 passed (the wasm32-trap regression test).
- `cargo run -p xtask --locked -- lint-literals` — PASS (0 violations).
- Live probes: `npx playwright test --list` (smoke lane still finds 38 tests / 3 files with
  `e2e/web/**` excluded); `npx playwright test --config playwright.web.config.ts --list` without
  env → the intended fail-fast error, with env → per-mode URL validation error (both correct).

## 1. The P0 fix (`70ed69c`) — VERIFIED CLOSED

- `ui/src/App.tsx` merge-marker count: **0** (`grep -cE '^(<<<<<<<|=======|>>>>>>>)'` → 0); a
  repo-wide `ui/src` scan for marker lines is also clean.
- Both routes preserved per the wave-23 merge intent: `/discovery` (App.tsx:97) AND `/convert`
  (App.tsx:98), with both screen imports present (App.tsx:17, :21). The diff removes exactly the
  three marker lines + two duplicated comment blocks — no logic dropped (6 deletions total).
- Typecheck/lint/build/test all green on the fixed tree (see battery above). The commit's
  "unblocks UI typecheck/build" claim reproduces.

## 2. Settings regen (`771259c`) — VERIFIED IN SYNC

- `npm run settings:check` passes (byte-identical regeneration — authoritative).
- Cross-checked counts against `schemas/settings.toml`: **32 `[[setting]]` entries**; **14 known
  areas** in `[areas].known`; generated `schema.ts` carries entries on **7 screens**
  (engine 6, extensions 5, global 1, interface 7, performance 3, playback 6, privacy 4 = 32).
- The 5 new `extensions.crash-backoff-*` / `crash-restart-limit` / `crash-disabled-reenable-ms`
  rows match the toml values (spot-check `crash-backoff-initial-ms`: default 5000, range 0–600000,
  screen `extensions`, section `crash-isolation`, order 10, `--extensions-crash-backoff-initial-ms`,
  since 0.1.0 — schema.ts:603-621 vs settings.toml:393-404).

## 3. Harness quality (`ff6e6aa` + `231598f` + `d1cba99` + `9a13c39`) — SOUND

- **Config-driven:** `ui/e2e/web/matrix.config.json` is the single knob source (ports, artifact
  paths + fallback, isolation modes, pump budgets, play-to-end tolerance, browser evidence tiers,
  launch args, report names); `web-matrix-run.mjs:43-99` validates the shape loudly (missing knob /
  bad mode / non-`--` launch flag → exit 2 before any server starts). Runtime facts flow
  env-injected (`WEB_MATRIX_BASE_URLS/OUT_DIR/FIXTURE_BYTES`), never literals.
- **No stubs / no fabricated results:** the in-page harness (site/harness.js, 735 lines) drives the
  REAL engine Worker (real wasm instantiation, real fetch, real postMessage ordering); the
  Firefox/WebKit/Safari/Safari-iOS lanes are committed as `evidence: "not-runnable-in-sandbox"` +
  BCD-cited expectations + next steps; the chromium lane merges only fragments that were actually
  written and warns on missing ones. No TODO/FIXME/stub markers.
- **Playwright usage correct:** `PLAYWRIGHT_SKIP_BROWSER_DOWNLOAD=1` (web-matrix-run.mjs:302);
  executable resolution env → config path under browsers root → pinned (mjs:284-294), recorded
  into the report `tooling` block; `--no-sandbox`/`--disable-dev-shm-usage` come from config with a
  written container rationale; `playwright.web.config.ts` has no webServer/baseURL on purpose and
  the spec fails fast with an actionable message when run outside the driver (verified live).
- **Server-lifecycle fix (`d1cba99`) is correct:** `spawn` (async) instead of `spawnSync`, with an
  accurate root-cause comment (spawnSync starved the event loop → connections sat in the accept
  backlog → `net::ERR_ABORTED` at goto). Exit code propagates verbatim.
- **wasm32-safe throttle fix (`9a13c39`) is sound:** `EventPolicy::default_policy` now builds
  `default_throttle_source()` — cfg-split: native keeps the real `InstantSource`, wasm32 gets a
  static zero-reading `ZeroMonotonic` (events.rs:562-590). Rationale is documented in-code:
  `SessionConfig::new` constructs the default transiently before hosts inject, and std
  `Instant::now` panics on wasm32-unknown-unknown, so every wasm `session.open` trapped. The
  `throttle` field is `Arc<dyn MonotonicSource>` (events.rs:520) so both arms coerce; I verified no
  other reachable `InstantSource` construction remains on the wasm path (`TimeTickPolicy` uses
  media position). Regression test `the_default_policy_is_constructible_and_readable` passes
  natively; wasm32 check/clippy green.

## 4. Rust strictness (lane A's only Rust file: `crates/playback-engine/src/events.rs`)

fmt clean, clippy `--all-targets` clean, wasm32-target clippy clean, lint-literals PASS, new unit
test green. (Note: a plain `grep` of the file can display `#ust_use]` artifacts — that is a
terminal/ANSI display artifact of `#[must_use]`, not file corruption; octal dump confirms
`# [ m u s t _ u s e ]`.)

## 5. Scope discipline — PASS

Full delta `c339ee8..9a13c39` touches 15 files, all inside lane A's ownership: `ui/*` (App.tsx,
settings generated schema, e2e/web/**, scripts/web-matrix-run.mjs, eslint/tsconfig/playwright
configs, package.json, .gitignore), `docs/web/browser-matrix-20261110.json`, and
`crates/playback-engine/src/events.rs` (the wasm32-safe throttle fix — explicitly part of lane A's
task). No foreign crates, no feature-matrix.toml, no xtask, no ci.yml, no other agents' files.

## Findings

### P0
None open — the P0 (committed merge markers) is verified fixed.

### P1
None found.

### P2
1. **Final browser-matrix evidence is not yet committed.**
   `docs/web/browser-matrix-20261110.json:32` — the chromium lane is `"modes": []` (zero run
   fragments); the record's git block (:5) is pinned to `231598f` and `generated` (:3) to
   19:40:50Z, i.e. it predates the `d1cba99` + `9a13c39` fixes whose commit messages describe the
   live failing run those fixes came from. The passing post-fix run + regenerated record are the
   lane's core deliverable; if the lane closes without them the evidence chain is hollow.
   **Suggested fix:** after the fix stack, re-run `npm run webmatrix` on the final tree and commit
   the regenerated JSON (and update the git/generated stamps it embeds) before lane close.
2. **`report.markdownName` is a dead knob.**
   `ui/e2e/web/matrix.config.json:42` declares `markdownName: "browser-matrix-20261110.md"` and
   `ui/scripts/web-matrix-run.mjs:65` *requires* it in the validated shape, but nothing ever reads
   it — the driver writes only the JSON (`writeFileSync` at mjs:464-465); no
   `docs/web/browser-matrix-20261110.md` exists. The config header promises "every knob … lives
   here", so a required-but-unused knob misleads the next maintainer into expecting a markdown
   artifact.
   **Suggested fix:** either emit the markdown summary lane (a small renderer over the same report
   object) or delete the knob + its validation line until it does something.

### P3
1. **One timeout literal bypasses the config single-source claim.**
   `ui/e2e/web/site/harness.js:33` — `HARNESS_TIMEOUT_MS = 30000` is the default for every
   `waitUntil` (harness.js:93); the pump timeouts are config-driven but this one is not.
   **Suggested fix:** thread `config.pump.sliceTimeoutMs` (or a new `waits.timeoutMs` knob) into
   the harness runtime config the spec already passes via `page.evaluate`.
2. **Playwright lane timeout is a literal.**
   `ui/playwright.web.config.ts:44` — `timeout: 240_000`. Same class as above; the runner timeout
   is exactly the kind of knob matrix.config.json exists for.
   **Suggested fix:** add `runner.timeoutMs` to the config and read it here.
3. **Spawn-failure path skips graceful server shutdown.**
   `ui/scripts/web-matrix-run.mjs:342-349` — if the Playwright child fails at spawn
   (`child.once("error")`), the awaited promise rejects and the `server.close()` loop never runs
   (the process exits, so the OS reclaims the sockets — cosmetic, but the file's own care about
   lifecycle argues for symmetry).
   **Suggested fix:** wrap the child await in try/finally with the close loop in `finally`.
4. **`parseRange` silently mishandles multi-range requests.**
   `ui/scripts/web-matrix-run.mjs:177-202` — `"bytes=0-1,5-6"` is served as the single window
   `0-1` (the suffix after the first range is swallowed by `parseInt`). Browsers do not issue
   multi-range fetches for media, so the harness lanes are unaffected.
   **Suggested fix for strictness:** if the spec contains a comma, ignore the Range header
   entirely (serve 200 full-body), matching the range-blind shape.
5. **Worklog entry not yet committed.** Lane A's ownership includes the worklog; nothing appended
   to repo-root `worklog.md` in the audited delta (expected: lane still running — listed so the
   merge checklist doesn't lose it).

### Informational / environment
- The committed record's `tooling.chromiumExecutable`
  (`/home/z/.cache/ms-playwright/chromium-1200/...`) **does not exist in the current sandbox**
  (`~/.cache/ms-playwright` holds only `ffmpeg-1011`; no chromium found system-wide), so this
  audit could **not re-run the browser matrix end-to-end**; the harness verdicts above rest on
  code review, the committed record, and the live-run failure narratives in the commit messages
  (which are specific and internally consistent: `net::ERR_ABORTED` at goto, the wasm32
  `unreachable` trap, `missing field 'area'`).
- The wasm32 default (`ZeroMonotonic`) freezes throttle windows for a host that never injects —
  documented in-code as the honest placeholder; the only wasm host today (apps/web) always injects
  its pump clock. A future non-injecting wasm host would be silently event-throttled-forever; a
  one-time debug log on first read of the default under `cfg(target_arch = "wasm32")` would make
  that misconfiguration visible (optional hardening, not a defect).
- Remaining lane-A ownership items (feature-matrix web column, xtask dispatch, full_gate step 10,
  ci.yml additive wiring) are **not present** in the audited delta — presumably the still-running
  lane's remaining work; flagged so the closer checks they land.

— W24-R8, 2026-11-10. Audited `w24/a` @ 9a13c39 (unpushed; local-only) from `wt/w24r8`.
