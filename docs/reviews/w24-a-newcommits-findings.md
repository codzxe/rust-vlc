# Wave 24 — Lane A NEW commits audit (R11, audit lane A's post-R8 delta)

- **Auditor:** W24-R11 (review scout; one of 6 parallel agents).
- **Audited state:** local branch `w24/a` @ **`b22bff5`** (the tip when this audit started; re-verified
  unchanged at audit end). Delta audited: **`9a13c39..b22bff5`** — this is **THREE commits, not two**:
  `389bfcc` (record fragment BEFORE verdict + first real chromium capture), `ca21cb5` (human report +
  honest stamp regen + COOP/COEP knob), `b22bff5` (five R8 P3 fixes + range-blind staging fix). R8
  covered up to `9a13c39`; the task brief named only `ca21cb5`+`b22bff5`, but `389bfcc` sits inside
  the range and is covered here too. Read-only worktree `wt/w24r11` (branch `w24/r11`); lane A's
  running worktree `wt/w24a` was never entered.
- **Verdict headline:** the renderer/stamp/knob/fix work is real and mostly excellent — R8's P2-2 is
  CLOSED, P2-1 is HALF-closed, 4/5 P3s are fixed, the range-blind fix is sound on code review, the
  COOP/COEP knob matches doc 09. But the report's honesty note contains **one materially false claim
  (P1)**: it says the sandbox's Chromium was deleted and the matrix is "NOT re-runnable" — the exact
  binary the capture recorded still exists at the recorded path and runs (verified live), as does the
  hash-verified wasm artifact. The final-tree re-run that would close R8 P2-1 and exercise the
  b22bff5 fixes was skipped on a false premise.

## What was actually run (evidence for every claim below)

- **Deterministic re-render of the human report:** cloned `b22bff5` to `/tmp/w24r11-check`, ran
  `node scripts/web-matrix-run.mjs --restamp` there, diffed the regenerated
  `docs/web/browser-matrix-20261110.md` against the committed one: **byte-identical except the three
  provenance lines `--restamp` is defined to regenerate** (Record stamp date/commit, Regen reason,
  Record git block). The committed .md IS the renderer's output over the committed JSON; the renderer
  adds no facts (every table cell is read from the record or from matrix.config.json).
- **Independent re-derivation of every report number from the JSON:** 9/17 scenarios passed in BOTH
  modes; 12 `pump loop exceeded 30000ms` fails; 2 `web.decoders_absent` fails; 2
  `fetch-range-blind-server` 404 fails; 17 scenarios per mode; wasm lane 14 fetches, all
  status 200 / `application/wasm` / content-length 1294311 (both modes); Playwright 1.64.1; the
  HeadlessChrome/157 user agent; non-chromium lanes = `not-runnable-in-sandbox` + BCD expectations
  (3/2/2/2 items) + next steps. All match the .md exactly.
- **The "browser deleted" claim, tested:** `/home/z/.agent-browser/browsers/chrome-157.0.8103.50/chrome`
  — the exact path in the record's `tooling.chromiumExecutable` — **exists**, is executable, mtime
  **2026-11-10 06:34:05** (installed ~16 h before the capture; never deleted or reinstalled — a
  post-deletion reinstall would show a later mtime), and **runs**: `chrome --version` →
  "Google Chrome for Testing 157.0.8103.50", exit 0. `~/.cache/ms-playwright` now holds only
  `.links` (dir mtimes 22:50–22:52) — i.e. even the "only ffmpeg remains" half of the note is stale:
  ffmpeg-1011 (present at R8's 22:41 audit) was ALSO removed before the note was written at 23:18.
- **Re-runnability ingredients:** the wasm artifact
  `/tmp/my-project/target/wasm32-unknown-unknown/release/lumen_web.wasm` still exists — 1294311 bytes,
  sha256 `aa70ec59…4a19e2` = the record's `artifact.sha256` exactly; the fixture
  `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` = 425516 bytes = the record. Browser + artifact +
  fixture + committed harness = a re-run is possible in this sandbox today.
- **Stamp-regen honesty:** `git diff 389bfcc..ca21cb5` on the JSON touches ONLY `git.commit`
  (9a13c39 → 389bfcc) plus the appended `capture` and `recordRegenerated` blocks; `generated`,
  `tooling`, `artifact`, `fixture` and all five lanes are byte-untouched. `b22bff5` does not touch
  docs/web at all.
- **UI strictness:** `npx tsc --noEmit` (ui/) — exit 0. `npm run lint` — exit 0, **0 errors**, 28
  warnings, ALL in `src/screens/**` files this delta does not touch (identical to R8's baseline
  count); zero findings in the delta's own files. (node_modules provided by hardlink-copy from the
  main checkout — symlink creation is blocked in this sandbox; the copy is gitignored,
  `git check-ignore` confirms `/ui/node_modules`.)
- **Rust strictness: N/A** — the delta's 8 files contain no `.rs`; nothing new since R8's green
  battery on `events.rs`.
- **Display-artifact note (non-finding):** `process.env[matrix…` / `baseUrls[mode.id]` render as
  `envatrix` / `baseUrlsode` through this toolchain's output sanitizer (bare `[m` is eaten as an ANSI
  reset). Octal dump confirms both files' raw bytes are correct, and node executed web-matrix-run.mjs
  end-to-end in the re-render check. Same class of artifact R8 documented for `#[must_use]`.

## Per-item verdicts (the audit brief)

1. **The human report** (`docs/web/browser-matrix-20261110.md`) — **TRACEABLE, with one false
   claim.** Every table row traces to the JSON capture (proven by the byte-identical re-render +
   independent number re-derivation above): chromium = real `playwright-run` evidence; firefox /
   webkit / safari / safari-ios = `not-runnable-in-sandbox` + BCD-cited expectations + next steps,
   never a fabricated result. The "capture-based, browser since deleted" honesty note IS present —
   but its core factual assertions are wrong (see P1) and one prose literal has gone stale (P3-3).
2. **The stamp regen (R8 P2-1 mechanism)** — **HONEST.** Capture facts (commit/tree base, recorded
   moment) preserved verbatim in `capture`; `git` re-derived from the working tree; `recordRegenerated`
   names the pass and reason; the .md explains the record's-containing-commit-is-the-child
   relationship correctly (git block `389bfcc` → contained in `ca21cb5`). No fake fresh run. Residual
   nit: no dirty-tree flag (P3-2).
3. **The `markdownName` knob (R8 P2-2)** — **CLOSED (implemented).** `renderMatrixMarkdown()` is a
   deterministic renderer over the same report object; the driver writes
   `config.report.markdownName` on BOTH paths (live run and `--restamp`); the .md is committed and
   reproduces byte-for-byte. The knob is no longer dead.
4. **The five P3 fixes (R8)** — **4 CLOSED, 1 OPEN:**
   - P3-1 (HARNESS_TIMEOUT_MS literal) **CLOSED**: new `waits.timeoutMs` knob (config + validation),
     threaded by the spec into the page's runtime config; `runSuite` asserts it is present and
     overwrites the module-level default before any scenario runs; the 30000 literal remains only as
     the documented pre-config fallback for hand-driving the page outside the driver.
   - P3-2 (Playwright timeout literal) **CLOSED**: `runner.timeoutMs: 240000` in the config;
     `playwright.web.config.ts` validates it (positive integer, loud throw) and uses
     `timeout: matrix.runner.timeoutMs`. Same 240 s value as before — no behavior change.
   - P3-3 (spawn-error skips server shutdown) **CLOSED**: the awaited child promise now has
     `.finally(() => { for (const server of servers) server.close(); })` — closes on every exit path
     including spawn-error.
   - P3-4 (parseRange multi-range) **CLOSED**: a comma in the range spec → one-line warning + `null`
     → 200 full body (RFC 9110 §14.2 "a server MAY ignore the Range header field" — citation
     correct). No longer silently serves a wrong single window.
   - P3-5 (worklog entry) **OPEN**: nothing appended to repo-root `worklog.md` in this delta.
     Expected — lane A is still running — but carry it to the close-out checklist.
5. **The range-blind staging gap fix** — **SOUND (code review; not yet browser-exercised).** The
   capture's `fetch-range-blind-server` fail ("expected 200, got 404") was a staging gap: the static
   server maps URL routes to files under `sitedist/`, and the fixture was staged only under
   `servedFixture` (`fixture.avi`), never under `rangeBlindPath` (`fixture-norange.avi`) — so the
   route 404'd before the range-blind branch could run. The fix is one `copyFileSync` staging the
   same fixture bytes under the range-blind name. Reviewed against the server code: `GET
   /fixture-norange.avi` now statSyncs, hits the `rangeBlind` branch (Range header ignored, no
   `accept-ranges`, 200 + full `content-length` + full body) — exactly what the in-page scenario
   asserts (`status === 200`, `accept-ranges === null`, `bodyBytes === fixtureBytes`). The
   range-aware route is untouched (`fetch-range-supported-server` was already green: 206 + correct
   content-range). Path-traversal guard (`normalize` + `startsWith(siteDist)`) unaffected.
6. **COOP/COEP posture knob** — **CORRECT.** `serving.coopCoepDefault: false` in matrix.config.json
   (config-driven, boolean-validated by the loud shape check, off-by-default); rendered into every
   report (the "Shipped COOP/COEP posture (decision record)" section reads it); decision record added
   to doc 09 §9.5, consistent with §12 Q8 (off-by-default confirmation), §2.5 item 6 (both isolation
   modes still exercised by the harness — the knob deliberately does NOT change what the harness
   serves), §2.3 (deploy-check script still [PROPOSAL] — the acknowledged pending consumer), §3.3
   (flip lands with threading, Lead decision). All cross-referenced sections verified present.
   Informational: "closing §12 Q8" is lane-A-signed while §12 is addressed "to the Lead" — defensible
   (§9.5 already documented off-by-default, and card 7.K1 instructs the lane to apply it), but the
   Lead confirmation is implicit, not recorded.
7. **Strictness** — UI: tsc exit 0; eslint 0 errors / 28 pre-existing warnings (none in touched
   files). Rust: nothing touched → fmt/clippy/lint-literals N/A for this delta.
8. **Scope** — **PASS.** 8 files: `docs/09-web-wasm-target.md` (§9.5 — the lane's target doc, cards
   7.I1/7.K1), `docs/web/browser-matrix-20261110.{json,md}` (lane deliverables),
   `ui/e2e/web/matrix.config.json`, `ui/e2e/web/site/harness.js`, `ui/e2e/web/web-matrix.spec.ts`,
   `ui/playwright.web.config.ts`, `ui/scripts/web-matrix-run.mjs` — all inside lane A's ownership per
   R8's scope map. No foreign crates, no feature-matrix.toml, no xtask, no ci.yml.

## Findings

### P0
None.

### P1
1. **The honesty note's non-runnability claims are false — the recorded browser still exists and
   runs.** `docs/web/browser-matrix-20261110.md` ("Honesty note — read this first"): "the sandbox's
   Chromium binary was deleted after the capture (only ffmpeg remains under the Playwright cache;
   re-download is forbidden), so the matrix is NOT re-runnable in this sandbox", and the fail-shape
   section: the range-blind fix "awaits a browser re-run on a machine that has one". Verified false:
   the binary the capture ACTUALLY drove — `/home/z/.agent-browser/browsers/chrome-157.0.8103.50/chrome`,
   named by the record's own `tooling.chromiumExecutable` — exists, has mtime 06:34:05 (predates the
   capture; never deleted/reinstalled), and executes (`--version` → "Google Chrome for Testing
   157.0.8103.50", exit 0). The wasm artifact is likewise still present with the exact recorded
   sha256, and the fixture matches. What WAS deleted (sometime 22:50–22:52) is the ms-playwright
   CACHE content — chromium-1200 and even ffmpeg-1011 (so "only ffmpeg remains" was already stale
   when written at 23:18) — i.e. a different install the capture did not use. In a deliverable whose
   acceptance rule is "no claim beyond evidence" (card 7.I1), this is the one claim that goes beyond
   — and against — evidence, and it justified skipping the mandated final-tree re-run.
   **Suggested fix:** correct the honesty note (renderer prose in web-matrix-run.mjs — it is
   rendered, so fix it there and re-`--restamp`), and actually re-run the matrix on the final tree
   while the browser exists (see P2-1 below), committing the fresh record.

### P2
1. **R8 P2-1 is only half-closed: the committed capture predates the committed harness.** The
   2026-11-10 capture ran against the `9a13c39`(+dirty) tree, i.e. BEFORE `b22bff5`'s range-blind
   staging fix, multi-range guard, and P3 fixes — none of the b22bff5 changes have browser evidence,
   and the record's two red `fetch-range-blind-server` rows describe a defect already fixed on the
   branch. R8's ask was "re-run on the final tree and commit the regenerated record before lane
   close"; that re-run is possible in this sandbox (see P1) and remains outstanding. The report does
   disclose the situation honestly in its fail-shape prose ("fixed on w24/a after it … awaits a
   browser re-run") — the disclosure is honest, the stated impossibility is not.

### P3
1. **Stale config comment misdescribes the evidence shape.**
   `ui/e2e/web/matrix.config.json` (`browsers.chromium.comment`) still says the sandbox evidence run
   drives "Python-playwright's chromium-1200, Chrome for Testing 143.0.7499.4 … instead of the
   pinned 156.0.8078.4" — but the committed capture drove the `.agent-browser` chrome-157.0.8103.50
   build (per the record's tooling block). Update the comment to match the actual evidence run.
2. **`capture.commit` records the base commit, not the executed tree (no dirty flag).** The run
   executed at 22:14:59 with HEAD `9a13c39` and a DIRTY tree (the spec change that became `389bfcc`
   at 22:21:02 — required for red fragments to be written at all); the capture note's "the tree the
   browser lanes executed against" is therefore imprecise. Suggest `gitInfo()` also record a
   dirty flag (`git status --porcelain` non-empty) so future records distinguish clean from dirty
   captures.
3. **Hardcoded "735 lines" literal in the renderer prose has drifted.** The Method section of the
   .md states "site/harness.js, 735 lines"; the file was 734 lines at render time and is **750**
   after b22bff5. A non-record-derived literal in a renderer whose header promises "it adds no facts
   of its own". Drop the count or derive it.
4. **R8 P3-5 still open: no worklog entry.** Nothing appended to repo-root `worklog.md` in the
   audited delta (lane still running — carry to close-out).

### Informational
- The `--restamp` maintenance mode is well-designed (capture facts verbatim, loud schema check,
  reason overridable via `WEB_MATRIX_RESTAMP_REASON`, exits 0 without touching lane data) — a good
  pattern for future record maintenance.
- `renderMatrixMarkdown` clips cells at 220 chars and escapes pipes; error text is JSON-dumped for
  passes and stringified for fails — full text stays in the JSON. Sound.
- The report's "Record git block … its containing commit is its child — see git log" wording is
  accurate for how the artifacts were actually produced (verified: git block `389bfcc` ↔ contained in
  `ca21cb5`).
- This audit did NOT run the browser matrix itself (read-only mandate; running it would rewrite the
  committed record files). The re-run recommendation above is for lane A, which owns the record.

— W24-R11, 2026-11-11. Audited `w24/a` @ `b22bff5` (tip at audit start, re-verified unchanged) from
`wt/w24r11`, branch `w24/r11`.
