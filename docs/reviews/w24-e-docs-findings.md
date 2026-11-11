# W24-R2 review — lane E docs audit (issue #58, branch `w24/e`)

Read-only audit of `origin/w24/e` (release-docs lane) against
`origin/main` (`c339ee8`). Auditor: W24-R2, wave 24 review scout 2.
Scope per task card: truth audit of the new docs, overclaim sweep,
rc-smoke harness stub/mock check, consistency of referenced paths,
formatting/rules. Every check below was re-run from a fresh worktree
(`/home/z/my-project/wt/w24r2`, branch `w24/r2`) unless marked
"not re-run" with the reason.

## Verdict

The lane is **substantially honest and high-quality**. The core
truth-audit claims I sampled reproduce exactly (version-stamping
two-track mechanism, matrix recompute 73/37/0, all four xtask gate
outputs, keymap/settings counts, README overclaim greps, F-1/F-2
defect records, ADR register statuses, format-record statuses vs the
matrix). The rc-smoke harness is real — no stubs, no fake passes,
exit code is the gate, BLOCKED counts as FAIL. I found **zero P0/P1**
findings, **3 P2** (accuracy defects in the audit doc / checklist —
one wrong row classification, one dangling evidence pointer, one
defective verification command) and **6 P3** (count/output recording
nits, one portability nit, two staleness/process notes). None of them
invalidate the lane's all-green verdict for its docs scope; all have
suggested one-line-class fixes.

## Findings

### P2-1 — Audit §1.1 table misclassifies `codec-video-h264-software` as a *claimed* row and omits `codec-video-vp9`

- **File:line:** `docs/audit/docs-sweep-20261110.md:40`
- **Evidence:** The §1.1 table's video-codec row lists claimed rows as
  "mjpeg, av1, h264, hevc, prores-dnxhd, **h264-software ★**".
  Re-parsed `docs/feature-matrix.toml` today: the six video-codec rows
  with desktop status implemented/partial are mjpeg, av1, h264,
  hevc, prores-dnxhd, **vp9** (`codec-video-vp9` = implemented ×3).
  `codec-video-h264-software` = **blocked ×4** (its own new record
  `docs/formats/video-h264-software.md:10-14` says so correctly).
  The count (6) and the GREEN verdict are unaffected — vp9 is covered
  by `video-vp8-vp9.md` (exists; asserted in the §1.1 preamble) and
  h264-software now has its record — but the table, which is the
  deliverable's proof of "37/37 claimed rows covered", names the wrong
  6th row. Notably `docs/guides/getting-started.md:90` gets it right
  ("MJPEG/AV1/VP9/(partial) H.264/HEVC/ProRes").
- **Suggested fix:** In the §1.1 video-codec row, replace
  "h264-software ★" in the claimed-rows cell with "vp9", list
  `video-vp8-vp9.md` in the records cell, and move h264-software to
  the §1.2 no-record-owed-style register (blocked row, record authored
  this lane as a gap record).

### P2-2 — Two new format records cite `docs/audit/rc-smoke-2026-11-10.md`, which was never committed on any branch

- **File:line:** `docs/formats/container-avi.md:68`,
  `docs/formats/audio-pcm.md:56-58`
- **Evidence:** Both records' Gates sections point at
  `docs/audit/rc-smoke-2026-11-10.md` ("smoke row 3"). `ls
  docs/audit/` shows only `docs-sweep-20261110.md`;
  `git log --all --diff-filter=A -- docs/audit/rc-smoke-2026-11-10.md`
  is empty (never existed on any local branch; the remote has no such
  file on `w24/e`). The rc-smoke run evidence actually lives in the
  `ff001e5` commit message and `docs/release-checklist.md` §2.
  Secondary nit: both call it "smoke row 3", but in the landed harness
  (`scripts/rc-smoke.sh`) headless-decode is check **4** and
  command-layer-play is check 5 — the records were authored in
  `d915188`, before the harness (`ff001e5`) existed, and reference a
  planned file that never landed.
- **Suggested fix:** Repoint both references at the real evidence
  (release-checklist §2 / commit `ff001e5`), or commit the rc-smoke
  run log under `docs/audit/` at wave merge; fix the row number (4).

### P2-3 — Checklist row 1.2's syntax-check command does not check what it claims (bash -n multi-file pitfall)

- **File:line:** `docs/release-checklist.md:51` (and the recorded
  output `docs/audit/docs-sweep-20261110.md:199`)
- **Evidence:** The row's command is
  `bash -n scripts/rc-smoke.sh scripts/release/*.sh`. `bash -n`
  syntax-checks only the **first** file argument; the rest become
  positional parameters and are silently unchecked. Demonstrated in
  this audit: `bash -n good1.sh bad2.sh good3.sh` → exit 0 with
  `bad2.sh` containing a deliberate syntax error. The audit §7's
  `bash -n scripts/release/*.sh (8 scripts)` has the same defect (and
  the glob is 7 scripts, see P3-5). The **conclusion** is true — I
  re-ran a proper loop (`for f in …; do bash -n "$f" || echo FAIL;
  done`) and all 8 sh scripts pass — but the row as written yields a
  false green on RC day if any release script is broken, which defeats
  the checklist's "every row names the exact command" contract.
- **Suggested fix:** Replace with a loop, e.g.
  `for f in scripts/rc-smoke.sh scripts/release/*.sh; do bash -n "$f" || exit 1; done`
  (and mirror in audit §7).

### P3-4 — Audit §7 recorded command/output pair for the settings count is not reproducible as written

- **File:line:** `docs/audit/docs-sweep-20261110.md:110,196`
- **Evidence:** Recorded: `rg -c '\[\[setting\]\]'
  schemas/settings.toml` → **32**. Actual output today: **33** — line
  13 of the schema is a comment containing the literal `[[setting]]`.
  The true entry count is 32 (`rg -c '^\[\[setting\]\]'` → 32), so the
  claim is right but the recorded command/output pair is off by one
  (unanchored pattern).
- **Suggested fix:** Record the anchored pattern
  `rg -c '^\[\[setting\]\]'` → 32.

### P3-5 — "8 scripts" count for `scripts/release/*.sh` is wrong (7)

- **File:line:** `docs/audit/docs-sweep-20261110.md:199`;
  `docs/release-checklist.md:51` ("8 sh + 1 mjs + 2 py")
- **Evidence:** `ls scripts/release/*.sh` → 7 scripts (gen-wxs,
  make-app, make-appimage, make-dmg, package-deb, package-windows,
  release-notes). The checklist's "8 sh" is only correct if
  rc-smoke.sh is included, which its own command lists separately; the
  audit's "`scripts/release/*.sh` (8 scripts)" is unambiguous and
  wrong.
- **Suggested fix:** Say "7 release scripts + rc-smoke.sh".

### P3-6 — `streaming-dash.md` presents a stale crate test count as "on disk at authoring"

- **File:line:** `docs/formats/streaming-dash.md:41-44`
- **Evidence:** "Test count on disk at authoring: 27 dash `#[test]` fns
  inside the streaming crate's **191 lib tests (2 ignored)**". The
  dash-specific 27 is exact (re-counted: resolve 12 + fuzz 4 + abr 4 +
  sched 7). But `crates/streaming/src` at HEAD has **206** anchored
  `#[test]` fns (193 at `93b7614`, the commit that recorded "191
  lib tests (2 ignored)" in the matrix row — 191 passed + 2 ignored =
  193, consistent then; 13 tests landed in later waves). The record
  attributes 191 to the matrix row but prefixes it with "on disk at
  authoring", which was already stale by 13.
- **Suggested fix:** Drop "on disk at authoring" for the 191 figure or
  re-count (e.g. "206 `#[test]` fns today; the matrix row records 191
  lib tests (2 ignored) at its 6.C2 flip").

### P3-7 — `rc-smoke.sh` check 6 uses GNU-only `stat -c %s` (fails on macOS)

- **File:line:** `scripts/rc-smoke.sh:131-132`
- **Evidence:** `stat -c %s "$WASM" 2>/dev/null || echo 0` — on macOS
  `stat -c` is invalid (BSD stat wants `-f %z`), so the size probe
  reads 0 and check 6 would always FAIL on a Mac even after a
  successful build. Today the harness is Linux-sandbox-scoped (3-OS
  rows are checklist-blocked), so this is portability hygiene, not a
  live break. Everything else in the harness re-verified clean: exit
  code is `exit $fail`; every BLOCKED path sets `fail=1` (an RC with
  a red web build is not shippable — matches the doc claim); no
  hardcoded must-be-config values (fixture/port/chromium are env
  overrides; binaries resolved from `CARGO_TARGET_DIR`); the check-2
  regex, check-3/5 grep patterns and check-4 awk fields all match the
  real CLI output formats (`apps/cli/src/help.rs:101`,
  `run.rs:631-636`, `bench.rs:22`, `run.rs:724`).
- **Suggested fix:** `stat -c %s "$WASM" 2>/dev/null || stat -f %z "$WASM"`.

### P3-8 — Checklist §3.2 "origin/w24/c was not pushed at authoring time" is now stale

- **File:line:** `docs/release-checklist.md:121-122`
- **Evidence:** `w24/c` **is** pushed now (`git ls-remote` →
  `fb97822`), and the referenced evidence
  (`docs/packaging/evidence/linux-lane-20261110.log`) plus all named
  `scripts/packaging/*` drivers exist on it (verified via
  `git ls-tree origin/w24/c`). The time-stamped phrasing keeps the
  sentence honest-as-of-authoring, and the rows still correctly
  "execute at wave merge", but per the file's own §8 maintenance rule
  ("when a row's reality changes … strike the blocker") this one is
  due for a strike-through.
- **Suggested fix:** Strike the "not pushed" clause, keep the
  execute-at-merge rows.

### P3-9 — Worklog covers the whole lane in one consolidated entry (r4), r1–r3 have no per-round entries

- **File:line:** `worklog.md:778-795`
- **Evidence:** `rg 'Task ID:' worklog.md` shows a single W24-E entry
  (r4 closeout). It is thorough and honest — it enumerates the r1–r3
  commits and their content, records the disk-full limitation, and the
  D-10 measure/not-tested blocks — but per-round detail otherwise
  lives only in commit messages. Noting for the record given the
  worklog discipline other waves followed (one entry per task).
- **Suggested fix:** None required; optionally split per round at wave
  merge if the Lead wants 1:1 task/entry mapping.

## What was sampled and verified (truth audit)

Re-run from the fresh worktree, with outputs compared to the docs'
recorded claims:

1. `scripts/release/product-env.py --print …` → `PRODUCT_VERSION=0.1.0-dev`,
   `PRODUCT_VERSION_CODE=1`, `MSI_VERSION=0.1.0`, `BIN_CLI=lumen`,
   `BIN_DESKTOP=lumen-desktop`, exit 0 — **matches checklist §0 / audit §7 exactly**.
2. Two-track versioning: `apps/cli/Cargo.toml:34` `version = "0.1.0"`
   vs `product.toml` `0.1.0-dev`; `apps/cli/src/help.rs:101`
   `format!("lumen {}\n", env!("CARGO_PKG_VERSION"))` with the
   `[PROPOSAL: product.toml identity …]` marker — **checklist §0's
   "NOT wired to product.toml" claim is true**.
3. `python3 yaml.safe_load(.github/workflows/release.yml)` → jobs
   windows / macos-arm64 / macos-x64 / linux-x64 / linux-arm64 /
   release; runners windows-latest, macos-latest, macos-15-intel,
   ubuntu-latest, ubuntu-24.04-arm (**continue-on-error: true**),
   release needs all five — **matches checklist §3.1 table**.
4. xtask gates (cached `target/debug/xtask`, same binary the audit
   used): `lint-literals` → PASS, 0 violations, **452 allowlisted**;
   `check-identity` → PASS, 0 violations, **453 allowlisted**;
   `sbom --check` → no drift, **157 components** (118 registry + 39
   path), subject `Lumen 0.1.0-dev`; `config-validator` → **FAILURE, 4
   violations, all missing `assets/brand/*`** (icon.ico, icon.icns,
   icon-512.png, logo.svg) — **all four match audit §7 / checklist
   §1.4–1.6 byte-for-byte**.
5. `scripts/full_gate.sh` has exactly the 9 steps the checklist lists
   and does **not** run config-validator — claim verified.
6. Matrix recompute (tomllib parse of `docs/feature-matrix.toml`):
   **73 format rows = 21+16+19+17; 37 claimed desktop = 14/6/9/8; 0
   web** — matches audit §1/§4.3, README, getting-started §3,
   release-notes skeleton.
7. `ls docs/formats/*.md | wc -l` → **53** — matches audit §1.
8. Format-record statuses vs matrix (8 rows sampled):
   musepack/speex/wavpack **blocked ×4** ✓, cc-608-708 / pcm / avi /
   pgs **implemented ×3, web not_started** ✓, encoding-detection
   **partial** ✓, h264-software **blocked ×4** ✓, network-dash
   **partial** ✓. Musepack feasibility quote matches
   `docs/feasibility/sections/audio-codecs.md` §1 row 18 verbatim.
9. Dash test counts: `resolve 12 + fuzz 4 + abr 4 + sched 7 = 27` —
   exact (the crate-wide 191 figure is the P3-6 nit).
10. `keymaps/default.toml` → **150** bindings; 17 guide-table bindings
    spot-checked (Space/S/N/P/./,/arrows/±3s/±60s/±300s/+-=/[]/↑↓/M/B/V/
    Shift+V/H/G/F/Esc/A/C/Shift+S/T/Ctrl+T/Ctrl+L/Q/L/Ctrl+Q) — all
    match, including provenance tags (C-01/C-02/D-003 design-vs-VLC).
11. `schemas/settings.toml`: **14 areas**, **21 `ui = true` entries**,
    **32 `[[setting]]`** (anchored), **5 `extensions.crash-*` keys** —
    matches settings.md (the unanchored rg output is the P3-4 nit).
12. `product.toml` defaults vs settings.md table: window 1280×720,
    min 480×320, theme dark, locale en-US, telemetry false,
    crash_reports "off", update_checks false, cache 32/64 MiB,
    max sessions 4, `features.hw-decode = false` — all match.
13. Fixture `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi` =
    **425516 bytes** — matches the getting-started BENCH example
    exactly; `apps/cli/tests/session_script.rs` exists.
14. CLI surface: `CliCommand` enum (Open/Play/Pause/Toggle/Stop/Close/
    Seek(Absolute|Relative|Percent|Chapter)/Info/Time/Duration/Tracks/
    EngineInfo/Commands/SettingGet/SettingSet/DecodeBench/Advance/Help/
    Quit) — matches the guides' verb lists including the
    `90s|+10s|-1m|50%|chapter 3` seek forms; BENCH line contract
    (`bench.rs:22`), `state-changed {from} → {to}` (`run.rs:724`),
    engine-info `  version:`/`  decoders:` (`run.rs:631-636`) all
    match rc-smoke's parsers and the guide examples.
15. rc-smoke harness (stub/mock check): **real checks, no fakes** —
    cargo build/version/info/decode-bench/open all invoke the built
    binary and parse its real output; wasm check verifies file size +
    `\0asm` magic; web checks run the actual `npm run build` +
    `vite preview` + Playwright load with a zero-pageerror assertion;
    `exit $fail` propagates; BLOCKED (check 8 after a failed check 7)
    sets `fail=1` exactly as documented. The `ff001e5` commit message
    records a failure-path proof (random-bytes fixture → checks 4+5
    FAILED, exit 1). Env overrides (RC_SMOKE_FIXTURE / _PORT /
    _CHROMIUM / _SKIP_WEB) exist and are documented. Only defect
    found: P3-7 (GNU stat).
16. F-1/F-2: `git show origin/main:ui/src/App.tsx | grep -c '<<<<<<<'`
    → **2** (lines 15, 100 — screen imports + route table, as
    troubleshooting.md says); merge commit `45c5c6f` exists with the
    quoted message; local `w24/a` has **0** markers and commits
    `70ed69c`/`771259c` exist with the quoted messages; **`origin/w24/a`
    is absent from the remote** — the "fix verified on lane A's local
    branch, unmerged" record is accurate. `/discovery` + `/convert`
    routes exist on both sides of the conflict (keep-BOTH fix is the
    right call).
17. Playwright pin drift: `ui/package-lock.json` pins
    `@playwright/test` **1.64.1**; sandbox ships `chromium-1200` +
    `chromium_headless_shell-1200`; the mjs bridges via explicit
    `executablePath` — matches troubleshooting.md and the mjs header.
18. Overclaim sweep re-run (README, RELEASE, KNOWN-LIMITS, guides,
    release-notes): patterns `all formats / every format / all codecs
    / supports everything / full parity / 100% compatible / complete
    compatibility / production-ready / blazingly` → **zero live
    overclaims**; the only hits are the rule quoting itself
    (KNOWN-LIMITS §8 R-26, guides README, skeleton) and one
    records-plural sentence in troubleshooting. Matches audit §4.1.
19. `git tag` → **0 tags** ("no release has been published yet" ✓);
    phase6-exit.md verdict line is **"DRAFT — PENDING LEAD DECISION"** ✓;
    README's Phase-7 merge claims all resolve to real `merge wave
    20–23` commits with the cited issue numbers (#40–#48, #50, #52,
    #55/#57, #56, #39).
20. ADR register: `docs/adr/` = **12 files** (10 ADRs + ADR-002
    evidence pack + README), register = 11 rows; Status lines quoted
    verbatim (ADR-001/002 accepted, ADR-004 accepted + per-OS
    PENDING, ADR-010/013 **proposed**); ADR-013 drift-flag commits
    `3b82b4b`/`67a139c` exist and
    `crates/codecs/src/video/h264/hw/vaapi/` exists — the honest
    drift flag is real.
21. `release-notes.sh` dry run re-executed: exit 0, dispatch-mode
    header, 20-commit list (`git log -20` cap — the "20-commit range"
    record is the script's behavior, not a miscount); first-release
    rule is `-50` at tag time, matching the skeleton's claim and
    RELEASE.md:121.
22. RELEASE.md known-limits numbering: 1 = unsigned, 2 = workflow
    never executed (CI billing), 5 = placeholder icons/brand assets,
    6 = MSI version sanitization — every "RELEASE.md known-limit N"
    cross-reference in the checklist resolves correctly.
23. Release scripts read `PRODUCT_*` env (no `0.1.0` literals in
    `scripts/release/*.sh`) — checklist §0's "never literals" holds.
24. Syntax battery re-run properly: `bash -n` on each of the 8 sh
    scripts individually + `node --check` mjs + `py_compile` both py —
    all pass (this is what makes P2-3 a command defect, not a false
    conclusion).
25. `/keyboard-shortcuts`, `/discovery`, `/convert` routes +
    `ui/src/screens/settings/areas/hotkeys/generated/bindings.ts` +
    `hotkeys:check`/`settings:generate` npm scripts all exist.
26. No TODO/XXX/FIXME markers in the new docs; all new files are
    English; markdown structure sane (tables well-formed).

**Not re-run (and why):** `cargo test`-based gate rows inside the
format records (85/28/39/32/2 counts) and a fresh rc-smoke execution —
the sandbox disk is at 97% (363 MB free) and cargo builds die with
ENOSPC; this matches the lane's own recorded disk-full limitation, and
the static evidence (module paths, test files, fixture sidecars) all
checks out. The "10 vCPU" environment claim in getting-started is not
verifiable from this auditor's container (2 cores here) — noted, not
counted either way.

## Summary table

| ID | Severity | File:line | One-liner |
|---|---|---|---|
| P2-1 | P2 | docs/audit/docs-sweep-20261110.md:40 | §1.1 table lists blocked h264-software as a claimed video-codec row; actual 6th claimed row is vp9 (implemented) — verdict unaffected, enumeration wrong |
| P2-2 | P2 | docs/formats/container-avi.md:68, audio-pcm.md:58 | Both cite `docs/audit/rc-smoke-2026-11-10.md` — never committed on any branch; also say "row 3" where the harness numbers it 4 |
| P2-3 | P2 | docs/release-checklist.md:51, docs-sweep:199 | `bash -n a.sh b.sh…` only checks the first file — row 1.2's command silently skips 7 of 8 scripts (conclusion independently re-verified true) |
| P3-4 | P3 | docs-sweep:110,196 | Recorded `rg -c '\[\[setting\]\]'` → 32 actually prints 33 (comment hit); anchored pattern gives the claimed 32 |
| P3-5 | P3 | docs-sweep:199, checklist:51 | `scripts/release/*.sh` is 7 scripts, not 8 |
| P3-6 | P3 | docs/formats/streaming-dash.md:41 | "191 lib tests" presented as on-disk-at-authoring; disk had 206 (191+2-ignored was true at the 6.C2 recording commit) |
| P3-7 | P3 | scripts/rc-smoke.sh:131 | GNU-only `stat -c %s` — check 6 would always fail on macOS |
| P3-8 | P3 | release-checklist.md:121 | "origin/w24/c not pushed" now stale (fb97822 is on the remote; evidence file verified there) |
| P3-9 | P3 | worklog.md:778 | Single consolidated W24-E entry covers r1–r4; per-round detail only in commit messages |
