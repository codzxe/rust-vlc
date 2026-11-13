# Phase 4 Exit Gate — Desktop Product (card 4.I1-V)

- **Verdict: PENDING LEAD SIGNATURE** — this is the evidence record produced by
  gate agent P4-GATE (card 4.I1-V) for the Lead's review. The Lead signs the
  final verdict after reviewing the evidence below. Nothing in this document
  is invented: every number is from a run executed this session (logs
  committed under `docs/gates/logs/phase4-exit-*`) or from a log/commit
  already on disk (cited per row). Items this session could not verify are
  labeled `[VERIFY]` or PENDING (§4/§5).
- **Date:** 2026-10-24 · **Agent:** P4-GATE (gate evidence; worktree
  `worktrees/p4-gate`, branch `wip/p4-gate`, based on `main @ be4e4c6` + this
  branch's evidence/hygiene commits) · **Final verdict owner:** Lead.
- **Baseline:** `main @ be4e4c6` — all 24 pre-gate Phase-4 cards landed
  (4.A1/4.A2 foundation, 4.B1-B4 primitives, 4.C1-C3 player, 4.D1/4.D2
  playlist+info, 4.E1-E4 settings, 4.F1-F4 variants+onboarding+a11y, 4.G1/4.G2
  visual harness+matrix, 4.H1 desktop shell, 4.H3 l10n). **4.H2 (rebrand) is
  IN FLIGHT in a parallel worktree** and merges after this session (§5).
- **Structure:** modeled on `docs/gates/phase3-exit.md` (gate-question table,
  evidence command table, card ledger, honest pending, Lead sign-off left
  PENDING), secondary model `docs/gates/phase2-exit.md`.

## 1. The doc-13 §7 gate question (card 4.I1-V), mapped to status

Card 4.I1-V: **"Phase 4 gate: visual matrix within tolerance; rebrand test
green; config-propagation proof; l10n rules green; packaging launch tests on
3 OS. AC: evidence + ledger + Phase 5 launch list."**

| Clause | Status | Evidence |
|---|---|---|
| visual matrix within tolerance | **MET (Linux, this sandbox)** | G2's committed record `ui/e2e/visual/MATRIX-20261024.md`: nightly **136/136 passed (3.2 m verify, exit 0)** after a 4.5 m capture (`--mode=nightly --update`), tolerance UNCHANGED from G1 (`maxDiffPixelRatio: 0.001`, per-pixel `threshold: 0.1`), zero remediations, zero tolerance loosening, zero hand-edited baselines; PR lane **30 passed (43 s)** in G2's run. **Re-verified THIS session: `npm run visual -- --mode=pr` → 30 passed (1.1 m), exit 0** (§2 row 4). **3-OS: infra-blocked** — Linux-only sandbox; per-OS baseline dirs are structural (`baselines/linux/` only populated); macOS/Windows matrix cells PENDING runners (same posture as gate-3 §1 row 3) |
| rebrand test green | **MET (fast tier; 2 surfaces infra-blocked, honestly)** | H2 merged @ 9aac5ca (merge 8a8733f): rebrand suite **7 PASS / 2 BLOCKED / 0 FAIL** — fixture-load, window-title, installer-identity (config plane), About, CLI-identity (resolution plane), logs, docs all Nova-verified; installer-build BLOCKED (no bundler in sandbox — nightly owns it), CLI banner host-wiring BLOCKED (apps/cli help.rs v0 hardcodes; own [PROPOSAL] names it). Log: docs/gates/logs/rebrand-acceptance-4h2.log |
| config-propagation proof | **PART — provable seams green; app-layer wiring NOT yet provable (honest)** | Provable this session: (a) **schema→screen**: `schemas/settings.toml` (authored truth, 1.B2) → `ui/scripts/gen-settings-screens.mjs` → generated screens, drift-guarded — **`npm run settings:check` OK (5 ui areas, 21 ui entries, 27 total)** (§2 row 3); (b) **schema→engine-side consumption exists in the CLI host**: `crates/settings-schema` runtime seam (`schema_meta` validation + setting get/set) consumed at `apps/cli/src/host.rs:171-172,1094-1129` and referenced by `crates/commands` catalog — `cargo test -p desktop -p playlists -p lumen-sweep` **82/0 green** this session (§2 row 5; note: these crates compile against the seam). NOT provable (per doc 07 intent vs reality, no overclaim): the **desktop app-layer wiring is H1's registered handoff** — `ClientProvider` still mounts `MockTransport` (swap to `createTauriTransport()` behind a host probe is unwired), real config writes from UI, engine feed to UI, `playlist.load`, dialog/drag-drop handlers, `window_config.rs` application are all unexercised end-to-end; the settings framework's persistence seam in UI tests is localStorage-mock (E1's record). Token-side: 4.A2's closure survey found **zero engine consumers of theme tokens** today (value-passing recommended, merge c54271f) — propagation is a Phase-5+ wiring task, not a Phase-4 claim |
| l10n rules green | **MET** | **`npm run l10n:check` PASS this session: 472 MessageKeys, 472 en-US strings — missing 0, duplicates 0, orphans 0, key-grammar ok, placeholders 0 ok, allowlist 5 entries** (§2 row 3); H3's en-US 100% gate + pseudo-locale clipping proof landed at 9b56338 (pseudo-locale clean per H3's committed evidence). Session finding: H3's `l10n-pseudo-probe.mjs` carried **6 pre-existing lint errors** on main (browser globals under a node-globals eslint block) — fixed by P4-GATE as hygiene (§2 row 2); its hardcoded `worktrees/p4-h3` path is flagged §4.7, not fixed |
| packaging launch tests on 3 OS | **INFRA-BLOCKED (honest; sandbox-provable scope MET)** | `apps/desktop/tauri/INFRA-BLOCKED.md` (H1): webkit2gtk-4.1, javascriptcoregtk-4.1, gtk+-3.0, soup-3.0, xdo ALL missing via pkg-config — tauri cannot even `cargo check` here and stays OUT of `Cargo.toml` per `docs/dependencies/records/tauri-wry-2.x.md` (exception-ADR-004, NOT imported). Sandbox is Linux-only; the 3-OS matrix stays parked (`.github/workflows-disabled/`, gate-3 §4 gap 1 precedent). Provable instead: **engine composition root through the single `lumen_dispatch` — `cargo test -p desktop` 38/0 this session (in the 82/0 row)**; TauriTransport UI seam 9/9 (H1); dev+release launch on any OS: NOT RUN, honestly |
| evidence record | **THIS DOCUMENT** | §2 command table + committed logs `docs/gates/logs/phase4-exit-*` |
| ledger | §3 | all 25 Phase-4 rows (4.A1..4.I1-V) with status + merge refs |
| Phase 5 launch list | §7 | concrete launch list from doc 13 §8 (waves 5.A-5.R, 45 tasks) + wave-7 corpus/parity handoffs |

## 2. Evidence record — verification commands run by P4-GATE on `wip/p4-gate`

All runs against `main @ be4e4c6` + this branch's hygiene commits (two fixes,
rows 2b/6b — both behavior-neutral, flagged for Lead fold-in per the 7-GATE
precedent). Environment: node 24, `ui/node_modules` symlinked to
`/home/z/my-project/ui/node_modules`, Chromium 156.0.8078.4, Linux sandbox,
Playwright/vitest/cargo all executing concurrently (recorded; medians not
needed — all gates are pass/fail).

| # | Command (as run) | Result | Log |
|---|---|---|---|
| 1 | `cd ui && npm run typecheck` | **CLEAN (exit 0)** — `tsc --noEmit` over the full tree incl. e2e (the 4.G1 TS2591 era is over: `@types/node` landed via be4e4c6) | `phase4-exit-ui-lint-postfix.log` (chained tail) |
| 2 | `cd ui && npm run lint` (first run) | **FAIL — 6 errors / 26 warnings, exit 1**: all 6 errors are `no-undef` in `ui/scripts/l10n-pseudo-probe.mjs` (8:26 `setTimeout`, 19/22/24/29 `document`/`window`/`getComputedStyle`) — H3's Playwright probe executes a `page.evaluate()` callback IN THE BROWSER but `eslint.config.js` grants `scripts/**/*.mjs` only node globals (console/process). **Pre-existing on main @ be4e4c6** (file from 9b56338; the be4e4c6 "lint clean" claim is contradicted by this run — recorded, not hidden) | `phase4-exit-ui-gate.log` |
| 2b | **lint hygiene fix (this session)** + re-run | P4-GATE added a browser-globals block for that one file to `ui/eslint.config.js` (config-only, zero behavior change). Re-run: **exit 0 — 0 errors / 26 warnings** (all the accepted `react-refresh/only-export-components` class + 2 unused-disable warnings, non-blocking). Lead should fold at merge | `phase4-exit-ui-lint-postfix.log` |
| 3 | `cd ui && npm run tokens:check && npm run settings:check && npm run l10n:check` | **ALL THREE GREEN (exit 0)**: tokens:check OK (sha-pinned generator, 4.A2); settings:check OK — "5 ui areas, 21 ui entries, 27 total"; l10n:check PASS — "472 MessageKeys, 472 en-US strings: missing 0, duplicates 0, orphans 0, key-grammar ok, placeholders 0 ok, allowlist 5 entries" | `phase4-exit-drift-guards.log` |
| 4 | `cd ui && npm run visual -- --mode=pr` | **30 passed (1.1 m), exit 0** — the PR visual lane re-verified on this tree (G2's committed PR-lane result: 30 in 43 s; nightly 136/136 in 3.2 m per `ui/e2e/visual/MATRIX-20261024.md`) | `phase4-exit-visual-pr.log` |
| 5 | `cargo test -p desktop -p playlists -p lumen-sweep` | **82 passed / 0 failed across 10 test binaries** — desktop 38/0 (composition root through `lumen_dispatch`), playlists m3u_suite 18/0, lumen_sweep 15/0, desktop protocol 4/0 + zero_alloc_transport 2/0 + 5 zero/empty mains. **Full workspace NOT re-run — deliberate**: Phase-3 gate (3.F2-V §2 row 1b) covered the workspace at 1939/0/36 and the Lead re-verified on main; Phase-4's Rust deltas live in exactly these three crates (H1 desktop, D1 playlists-v0, sweep tooling), so the phase-scoped run is the honest minimal set | `phase4-exit-rust-phase4-crates.log` |
| 6 | `cd ui && npx playwright test` (default config) | **37 passed / 31 failed (2.3 m)** — decomposed: a11y **16/16 green**, theme matrix **21/21 green**, visual.spec **0/30**, smoke **0/1**. Both failure classes diagnosed: (a) **30 visual failures are a CONFIG artifact, not regressions** — the default `playwright.config.ts` resolves Playwright's snapshot dir to `e2e/visual/visual.spec.ts-snapshots/`, NOT the pinned `e2e/visual/baselines/linux/`; every case fails "A snapshot doesn't exist … writing actual". The sanctioned visual entry is `npm run visual` (selects `playwright.visual.config.ts`), which is **green 30/30 this session** (row 4) and 136/136 nightly (G2). (b) **1 REAL stale test**: `e2e/smoke.spec.ts` "boot lands on the player placeholder" expected heading "Player", received "Welcome to Lumen" — stale since 4.F1's onboarding first-run redirect landed; the full e2e suite had not been re-run as a whole since (each card ran its own lane). | `phase4-exit-playwright.log` |
| 6b | **smoke fix (this session)** + re-run `npx playwright test e2e/smoke.spec.ts e2e/a11y.spec.ts e2e/theme.spec.ts` | P4-GATE updated smoke.spec.ts to seed the wizard-complete `lumen.onboarding.v1` state (the same OnboardingState seam visual.spec uses) before asserting the Player heading — the 4.A1 expectation preserved, onboarding's own first-run behavior covered elsewhere. Re-run: **38 passed (40.2 s), exit 0** — smoke 1/1, a11y 16/16, theme 21/21. **Net honest e2e state: a11y+theme+smoke green post-fix; visual lane green via its sanctioned entry; default-config visual wiring is a successor item (§4.6)** | `phase4-exit-playwright-postfix.log` |
| 7 | `cd ui && npm run test` (vitest) | **59 files / 557 tests passed, 0 failed** — the full UI unit gate, matching the post-H-wave merge state (f29bba8: "557/557") | `phase4-exit-ui-vitest.log` |

## 3. Card ledger — Phase 4 (doc 13 §7, 25 rows)

All statuses mined from `git log` + `worklog.md` (grep `Task ID: 4.`) on
`main @ be4e4c6`. "COMPLETE" = merged to main with committed evidence; every
row cites its merge/feature hash.

| ID | Card | Status | Evidence anchor |
|---|---|---|---|
| 4.A1 | ui scaffold | **COMPLETE (CLOSED)** | merge 171d0a7 (four gates green, e2e 1/1 chromium, eslint v2, CI ui.yml proposed); worklog 8264+8328 (sessions 1-2); session-1 infra: `ui/NEXT-STEPS.md` ordered list |
| 4.A2 | token pipeline | **COMPLETE (CLOSED)** | partial ca1e630 + complete c54271f (9×2 accent matrix e2e 22/22, tokens:check drift guard, Rust-consumer survey: zero engine consumers); worklog 8363 |
| 4.B1 | button/form primitives | **COMPLETE (CLOSED)** | partial 91e1f15 + closure 1778be8 (8 components, §4.4 matrix, 86 tests); worklog 8380 |
| 4.B2 | slider/seekbar/volume | **COMPLETE** | d905165 (pointer+keyboard interaction tests, truthful pending-seek per doc 08 §5) |
| 4.B3 | menu primitives | **COMPLETE** | 85bce35 (MenuBar/Dropdown/ContextMenu/Tooltip, data-driven doc 05 §8.3, WAI-ARIA roving tabindex, 38 tests) |
| 4.B4 | surface/list/icon | **COMPLETE** | e266a7e (Dialog/Sheet focus trap, Tabs, ScrollArea, virtualized List/Table 10k-row AC, 1.5-stroke icons; 35 tests, 117/117 suite) |
| 4.C1 | player shell | **COMPLETE** | d5b9b3f (merge p4-c1; layout-config-driven panel structure) |
| 4.C2 | control bar | **COMPLETE** | 12ec606 (transport/seek/volume/track menus over the client seam) + breadcrumb 78ffa07 |
| 4.C3 | UX states | **COMPLETE** | d55ef69 (StateRouter + Loading/Buffering/Unsupported/Error/Empty overlays) + breadcrumb 27f49a2 |
| 4.D1 | playlists crate v0 + panel | **COMPLETE (CLOSED)** | merge f0f210c; feature b05cbb6 (37 tests; catalog-contract commands, honest not-exposed); worklog 8397 + closure 27bc31f (app-layer wiring handoff to 4.H1) |
| 4.D2 | media info screen | **COMPLETE** | c5ad6d0 (fixture-driven, unknowns shown "Unknown", never invented) |
| 4.E1 | settings framework | **COMPLETE** | 19f244f (merge p4-e1) + tests 74d6770 (drift proof + flows; localStorage persistence seam recorded honestly) |
| 4.E2 | settings group 1 | **COMPLETE** | f3bc058 (Interface/Appearance/Language/Hotkeys, conflict detection) + breadcrumb 59decad |
| 4.E3 | settings group 2 | **COMPLETE** | c2b6e66 (audio/video/subtitles, disabled-with-reason) + breadcrumb a67634b (schema-owner handoffs registered) |
| 4.E4 | settings group 3 | **COMPLETE** | 5ac4c13 + merge 8caedf3 (codecs/library/network/about/advanced; F1/F2 feature-exposure gating) |
| 4.F1 | onboarding | **COMPLETE** | ee6fdd0 (opt-in-off privacy, skippable, persisted) + merge 591b1af |
| 4.F2 | player variants | **COMPLETE** | f21f407 (miniplayer 391×339 rx14 + music mode, layout-driven switching) + merge 66fc455; worklog 8413 (keymap-seed [PROPOSAL] handoff) |
| 4.F3 | customize panels | **COMPLETE** | 913d952 (capability-gated Crop/Aspect, Colors, Essential, HDR — no dead controls) + merge 5010dc6 |
| 4.F4 | a11y pass | **COMPLETE** | 47388e5 (axe suite, tab order, token focus rings) + merge 783436c; **re-verified this session: a11y 16/16 green** (§2 row 6) |
| 4.G1 | screenshot-diff harness | **COMPLETE** | 4c65749 (merge e2dabbc; `npm run visual` + `cargo xtask visual`, seeded 2-px regression proof, flakiness controls) |
| 4.G2 | baseline matrix | **COMPLETE (Linux)** | 657c8d0 + merge b1141a4; `ui/e2e/visual/MATRIX-20261024.md`: nightly **136/136**, PR **30/30**, tolerance unchanged; **PR lane re-verified this session 30/30 (1.1 m)** (§2 row 4); 3-OS cells infra-blocked |
| 4.H1 | Tauri shell | **COMPLETE (sandbox-provable scope); launch/3-OS INFRA-BLOCKED** | pt.1 c8b13d2 (composition root + dispatch/dialog/window-config seams, 38/0) + pt.2 b235ecd (tauri scaffold + TauriTransport 9/9) + merge d86b2b0; worklog 8434; `apps/desktop/tauri/INFRA-BLOCKED.md`; **desktop 38/0 re-verified this session** (§2 row 5) |
| 4.H2 | rebrand test | **IN-FLIGHT** (parallel worktree; merges after this session) | §5 — Lead fills at sign-off |
| 4.H3 | localization | **COMPLETE** | 9b56338 (en-US 100% gate, pseudo-locale clipping proof) + merge 6a947fa; **l10n:check re-verified this session: 472/472, 0 missing/dup/orphan** (§2 row 3); lint debt in its probe script found+fixed (§2 row 2/2b) |
| 4.I1-V | this gate | **EVIDENCE COMPLETE — verdict PENDING LEAD SIGNATURE** | this document |

## 4. Honest gaps — what this sandbox cannot prove at 4.I1-V

1. **H2 rebrand (IN-FLIGHT).** Owner: the 4.H2 agent in a parallel worktree;
   merges after this session. The clause stays PENDING with a Lead fill-in
   slot (§1, §5). Nothing rebrand-shaped is claimed here.
2. **3-OS everything (infra-blocked, carried from gate-3 §4 gap 1).**
   Visual matrix (Linux baselines only), packaging launch tests (no webkit →
   tauri not even cargo-checkable), scenarios (Phase-3 clause, same posture).
   Linux-only sandbox; CI is one lean ubuntu job by design; 3-OS matrix parked
   under `.github/workflows-disabled/`. To-run: Lead reconciles the matrix;
   per-OS baseline capture (`baselines/{macos,windows}/`) + launch tests on
   the merge commit.
3. **4 h soak follow-up (Phase-3 carryover, still open).** The 4 h nightly
   run has still not been executed (wall-clock); 5 MB/h target stays
   [PROPOSAL]. Registered as Phase-4-parallel follow-up at gate-3 sign-off —
   still open at this gate.
4. **App-layer wiring handoffs (H1's registered list — the desktop product's
   last honest mile):** swap `ClientProvider`'s MockTransport for
   `createTauriTransport()` behind a host probe; wire `files.rs` dialog/drop
   handlers to dispatch `open_file_request`; apply `window_config.rs` at
   window build; `playlist.load` per 4.D1's handoff; real config writes +
   engine feed to UI. Until these land, "config-propagation" is proven at
   the seams (§1 row 3), not end-to-end.
5. **Schema-owner gaps (E3/E4 handoffs):** E3's breadcrumb (a67634b)
   registers audio/video/subtitles default-seed handoffs to the
   settings-schema owner; E4's feature-exposure depends on product.toml
   F1/F2 completeness; 4.F2's variant-toggle keymap seed is [PROPOSAL].
   None gate-blocking (screens render + drift guards green), all named.
6. **Default Playwright config vs visual harness (session finding, §2 row
   6).** `npx playwright test` runs visual.spec against the WRONG snapshot
   dir (default `-snapshots/` instead of `baselines/linux/`) → 30 spurious
   failures for anyone running the plain command. The sanctioned entry
   (`npm run visual`) is green. Successor/Lead: set `snapshotDir` (or
   `testIgnore`) in `playwright.config.ts` so the plain command can't
   mislead.
7. **`l10n-pseudo-probe.mjs` reproducibility wart.** Hardcoded
   `cwd: /home/z/my-project/worktrees/p4-h3/ui` — the probe only runs from
   that path (H3 leftover). Lint now clean (§2 row 2b) but the path should
   be relativized by its owner.
8. **Lint warnings (accepted class).** 26 warnings remain (react-refresh
   export-shape + 2 unused eslint-disable directives in generated files);
   zero errors. Non-blocking; the 2 fixable ones can be `--fix`ed at merge.

## 5. In-flight: 4.H2 (rebrand) — NOT gate-blocking to record, REQUIRED to sign

- State: **IN-FLIGHT** in a parallel worktree at this writing; the result
  merges to main after this session. Owner: the **4.H2 agent** (rebrand
  acceptance test per doc 07 §8 — Nova fixture → window title, installer
  identity, About, CLI identity, logs, docs).
- **Lead sign-off slot (fill after the H2 merge):**
  `> 4.H2 merged @ <hash>: rebrand suite <passed/total> — clause MET / NOT MET: ____`

## 6. Decisions for the Lead at sign-off

- Sign the verdict (or reject with named blockers) on this evidence record —
  the H2 clause (§5) must be filled first.
- Fold this branch's two hygiene fixes into main: the eslint browser-globals
  block (§2 row 2b) + the stale smoke.spec onboarding seed (§2 row 6b).
- Default-config vs visual-harness wiring (§4.6) — assign owner.
- 3-OS matrix reconciliation (§4.2) — the recurring structural item.
- Phase 5 launch (§7) — issue the dispatch.

## 7. Phase 5 launch list (doc 13 §8 — waves 5.A-5.R, 45 tasks)

Gate dependency: **every Phase-5 format card chains from 4.I1-V** (doc 13:
5.A1 "Deps: 4.I1-V"). Parallelism 2-5 (per-format ownership zones + registry
snippets per R3; `+FB` bundle on every format card; `[CONDITIONAL]` cards
launch only if the named decision landed).

**P0 first — the lossless-audio + core set, all dep-only-on-gate (launch
immediately on pass, 4 parallel):** 5.A1 FLAC (SEEKTABLE + binary-search
fallback), 5.A2 WAV+AIFF native (RIFF variants + COMM/SSND), 5.A3 MP3
(symphonia + ID3v2/VBRI/Xing + gapless), 5.B1 Ogg polish (chained streams,
granule discontinuities). Then 5.B2 Vorbis after 5.B1.

| Wave | Cards (deps) | Launch shape |
|---|---|---|
| 5.A-5.B P0 core | 5.A1/5.A2/5.A3/5.B1 (all dep 4.I1-V), 5.B2 (dep 5.B1) | **Launch set of 4 on gate pass**; 5.B2 follows B1 |
| 5.C-5.D MP4/MKV | 5.C1 MP4/MOV full (dep 4.I1-V + 2.B3 outcome), 5.C2 AAC-LC+ADTS (dep 5.C1), 5.D1 MKV/WebM (dep 4.I1-V), 5.D2 MKV extras (dep 5.D1) | 5.C1 ∥ 5.D1 with the P0 wave; C2/D2 follow their parents |
| 5.E AV1 | 5.E1 rav1d asm-off (dep 4.I1-V), 5.E2 conformance+differential (dep 5.E1) | E1 joins an early wave; E2 sequential |
| 5.F-5.G H.264/HEVC | 5.F1 AVC parser (dep 4.I1-V), 5.F2 `[COND ADR-001]` decode integration, 5.G1 HEVC parser (dep 5.F1), 5.G2 `[COND]` | parsers early; integration cards gated on ADR-001/hw path |
| 5.H MPEG video | 5.H1 MPEG-1/2 (dep 4.I1-V), 5.H2-I ASP decision, 5.H3 MJPEG polish (dep 2.B4 ✅) | H3 can start immediately (2.B4 landed); H1 with a codec slot |
| 5.I systems containers | 5.I1 MPEG-TS (dep 4.I1-V), 5.I2 PS/VOB (dep 5.I1), 5.I3 ES wrappers (dep 5.F1+5.A3) | I1 early; I2/I3 fan out |
| 5.J AVI + audio breadth | 5.J1 AVI full (dep 4.I1-V + Phase-2 decision), 5.J2 ADPCM, 5.J3 ALAC+MP2 | all three parallel in a breadth wave |
| 5.K audio tail | 5.K1 TrueHD hardening, 5.K2-I Opus decision, 5.K3-I ASF/WMA decision, 5.K4-I AC-3/DTS decision | K1 imp + three `-I` investigations parallel |
| 5.L-5.N subtitles | 5.L1 overlay pipeline (dep 4.I1-V), 5.L2 cue model (dep L1), 5.M1 SRT / 5.M2 ASS / 5.M3 VTT (dep L2), 5.N1-N3 `[CONDITIONAL bitmap/CC]` | L1 → L2 → M-wave parallel; N-wave gated on the bitmap/CC decision |
| 5.O playlists | 5.O1 M3U8/PLS/XSPF (dep 4.D1 ✅), 5.O2 CUE (dep O1), 5.O3 probe/dir-load (dep O1) | can launch alongside early codec waves (Rust-only zone) |
| 5.P video-tail decisions | 5.P1-I VP8/VP9, 5.P2-I Theora + blocked-registry batch | investigation cards, any wave |
| 5.Q hardening | 5.Q1 fuzz sweep (dep 5.O*+5.P*), 5.Q2-V records/differential/blocked audit (dep 5.Q1) | late, after format waves |
| 5.R perf + gate | 5.R1 perf-vs-VLC breadth snapshot (dep 5.Q2-V), 5.R2-V `[LEAD]` Phase-5 gate (dep 5.R1) | terminal pair → `docs/gates/phase5-exit.md` |

**Wave-7 corpus/parity handoffs = Phase 5's actual workload (cite):** the
corpus program (4.WAVE4-CLOSE: corpus-big 485 files/24 classes, zero clean
ffprobe failures; 7-h census 538 files) and 7-d's parity sweep stand ready —
**7-d's record shows 19/19 frame counts + 8/9 PCM MD5s comparable on the
sweep set, with the unsupported-container files (the 103-class coverage
gap) carrying NO parity claim by construction**: those files ARE the Phase-5
demux/decode work (each format card flips its corpus class from
unsupported→comparable). 7-d's silent-skip handoffs (the demux dup-range
payload-loss note) route to the owning demux cards (5.C1/5.D1/5.I1/5.J1).
Differential-vs-VLC evidence per format reuses 3.E3's harness
(`tools/bench-vs-vlc`, PENDING-VLC oracle cells still open — Phase-5 cards
fill comparable rows as decoders land).

## 8. Log index (committed on this branch)

- `phase4-exit-ui-gate.log` — typecheck+lint+test first run (lint FAIL at 6
  pre-existing errors; chain halted before vitest — vitest run separately,
  row 7).
- `phase4-exit-ui-lint-postfix.log` — post-fix lint (exit 0) + typecheck.
- `phase4-exit-ui-vitest.log` — `npm run test`: 59 files / 557 passed.
- `phase4-exit-drift-guards.log` — tokens:check + settings:check + l10n:check
  (all green).
- `phase4-exit-visual-pr.log` — `npm run visual -- --mode=pr`: 30 passed.
- `phase4-exit-playwright.log` — `npx playwright test` first run: 37/31
  (decomposed + diagnosed §2 row 6).
- `phase4-exit-playwright-postfix.log` — smoke+a11y+theme post-fix: 38/38.
- `phase4-exit-rust-phase4-crates.log` — `cargo test -p desktop -p playlists
  -p lumen-sweep`: 82/0 across 10 binaries.

## 9. LEAD SIGN-OFF (card 4.I1-V — final verdict)

- **Verdict: PASS WITH REGISTERED CAVEATS — PHASE 5 IS AUTHORIZED TO LAUNCH.**
- Signed: 2026-10-24, Lead, on main post-merge of all Phase-4 branches
  (H2 @ 9aac5ca merged 8a8733f; gate-evidence branch merged bf42c7b).
- **Lead review performed:** (a) the 4.H2 slot is filled above from the
  MERGED branch (7/2/0 per its committed log — the two BLOCKED surfaces
  carry exact infra reasons + owners, blocker register pinned in-test so
  it cannot silently persist); (b) the two gate-session hygiene fixes
  (H3 pseudo-probe eslint globals; stale smoke.spec after F1's redirect)
  were folded at merge and the playwright run re-verified 38/38 green on
  the fixed tree; (c) spot re-verified on the merged tree this session:
  vitest 557/557, typecheck clean, lint clean, l10n 472/472, drift guards
  green, visual PR lane 30/30, Rust phase-4 crates 82/0.
- **The caveats (registered, not hidden):** (1) visual matrix is
  Linux-only — 136/136 nightly + 30/30 PR; the per-OS baseline mechanism
  exists (baselines/<platform>/) and activates when CI runners do;
  (2) packaging launch tests on 3 OS are INFRA-BLOCKED (no webkit2gtk /
  Linux-only sandbox — H1's INFRA-BLOCKED.md carries the exact dependency
  list + admission checklist); the desktop composition root IS proven
  (38/0 incl. a real-engine open→settle→events-in-order test) and the
  single lumen_dispatch seam (doc 05 §5.1) is wired engine-side and
  UI-side (TauriTransport 9/9); (3) config-propagation is an honest PART —
  schema→screen generation is drift-guarded both directions and the
  CLI-host seam is proven, but the app-layer wiring (real config writes,
  live engine feed to the UI) is H1's registered handoff, not faked here;
  (4) rebrand installer-build + CLI banner surfaces are BLOCKED per §1
  with owners; (5) carried from Phase 3: the 4 h soak baseline and the
  3-OS scenario matrix remain open follow-ups (7-g's sandbox-reaping
  evidence recorded; CI-nightly is the escalation path).
- **Why PASS is honest at this bar:** all five gate clauses have real,
  committed, re-runnable evidence or a named infra blocker with an owner;
  nothing was mocked as green (mock/real separation is mechanical in the
  differential harness; the rebrand blockers are pinned IN-TEST); the
  25-card ledger is complete with merge refs. The Phase-5 launch list
  (§7) is the operative output — it already folds wave-7's honest
  handoffs (silent-skip bugs, 103-unsupported container coverage) into
  the format cards that own them.

---

## Lead sign-off (the owed square-up, 2026-11-13)

**Verdict: PASS (Lead-signed 2026-11-13).** Same administrative
ratification as Phase 3's sign-off the same day: the evidence above stood
verdict-PENDING from 2026-10-24 while Phases 5–6 (and the wave-23/24/26
programs) proceeded on it, and Phase 5's Lead sign-off reviewed the chain
explicitly. The gate's named follow-ups — 3-OS scenario matrix breadth and
the CI-nightly escalation path — are discharged by the now-running hosted
CI (first full real execution 2026-11-13, wave-L) and the compat-matrix
workflow; the rebrand blockers it pinned IN-TEST are green (rebrand
skeleton + reverse-control suites pass; wave-L round 3 closed the last
surface-path drift). Signed: Lead (wave-26 orchestration), issue #59
square-up.
