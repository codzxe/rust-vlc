# PARITY-REPORT — the finalized honest parity statement (issue #57, cards 8.F1–8.F2)

| Field | Value |
|---|---|
| Purpose | The Phase 8 parity statement (MP §16): exactly what is verified, partial, unsupported and blocked — with evidence links, per MP §17 and doc 10 §7 |
| Card / issue | 8.F1 + 8.F2 / GitHub issue #57 (finalization lane `7.w22h`) |
| Computed | 2026-11-10 from `docs/feature-matrix.toml` at this commit, by the tools in §2 (their ACTUAL run outputs are quoted verbatim) |
| Source of truth | `docs/feature-matrix.toml` — "THE parity ledger" (MP §1); schema frozen 2026-10-19 (doc 02 §4.7); 252 rows / 21 categories, populated by task 0.D2 from the completed VLC 3.0.23 inventories (`docs/inventory/matrix-seed-notes.md`) |
| Baseline | VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd) |

---

## 1. What "parity" means here (the binding definition)

MASTER-PROMPT §1: **"Parity is the share of inventory features in
docs/feature-matrix.toml that are verified by automated evidence."** Two
distinct quantities are therefore tracked, and conflating them is the
program's core honesty failure mode (risk R-24/R-26):

- **Implementation share** — rows whose platform status is `implemented` or
  `verified` (code exists, evidence attached, but not yet CI-verified).
- **Verified parity (strict)** — rows with status `verified` only: specified,
  complete incl. error paths, tested on every CLAIMED platform, documented
  limits, non-regressing (MP §17), backed by automated evidence.

## 2. The tooling (card 8.F1 — lockdown state)

Both tools live in `tools/xtask` (std-only, zero dependencies) and derive
every number at run time from the matrix file — **nothing is hardcoded and
no parity percentage is ever stored** (matrix header rule):

- **`cargo xtask feature-audit`** (`tools/xtask/src/cmd/feature_audit.rs`)
  — structure checks (R9/R1/R2/R3 + R4/R5/R6 conditionals, CI-parity),
  evidence-presence (implemented/verified rows must carry evidence — the
  "implemented-with-evidence, never a bare flip" backfill rule), and
  **evidence-resolution**: every repo-path token in every `evidence` string
  must resolve to an existing file/dir under the workspace root.
- **`cargo xtask parity-report`** (`tools/xtask/src/cmd/parity_report.rs`)
  — the derived per-category / per-platform table (§4). It is read-only:
  **no code path exists that edits the matrix** — the doc 10 §7.3
  `collect`/outbox/transition-proposal machinery is `[PROPOSAL]` design,
  deliberately not built. Status transitions happen only through Lead-
  reviewed commits; `feature-audit` rejects evidence-less flips at audit
  time. "Attempting a manual status flip" in the working tree succeeds only
  as an uncommitted edit that the next audit judges — the anti-fabrication
  rule is the audit + git history, and **agents never mark their own work
  verified** (MP §15).

Honest tooling gaps (the auditor's own NOT-done register): R7 config
cross-check is owned by `config-validator`; R8 inventory completeness
needs a formats.json↔row reconciliation spec; R10 anti-hallucination
beyond path existence; and `not_started → verified` **git-history** jump
rejection needs a `--from` revision argument (v0 audits the working tree).

## 3. The 252-row audit status — ACTUAL run, 2026-11-10, this commit

```
$ cargo xtask feature-audit
=== LUMEN FEATURE-AUDIT REPORT ===
matrix: docs/feature-matrix.toml
rows: 252
per-phase rows: 2=10 3=5 4=93 5=49 6=18 7=25 8=52
per-category rows: audio-codec=19 audio-feature=11 config-feature=20 container=21
  extension=1 hotkey=17 menu-item=16 network-feature=7 platform-feature=18
  playback-op=13 playlist-feature=9 playlist-format=7 plugin-feature=4
  protocol-access=11 services-discovery=6 subtitle-format=17 text-render=3
  ui-dialog=15 ui-screen=11 video-codec=16 video-feature=10
per-platform-status cells: blocked=133 implemented=192 not_started=505
  partial=142 unsupported=36
result: PASS
```

The audit passed only after this card fixed two stale evidence-path
spellings (commit `c9a89c6`: `tests/https_activation.rs` →
`crates/streaming/tests/https_activation.rs`, `tests/pcm_pipeline.rs` →
`crates/audio-output/tests/pcm_pipeline.rs` — both files exist; the tokens
had been written crate-relative and failed root-relative R4 resolution;
**no status cell changed**). The 252/252 default-agreement inventory
cross-check (Phase 0) is separately recorded in
`docs/gates/phase0-exit-report.md` row 3.

## 4. Derived parity table — ACTUAL run, 2026-11-10, this commit

Implementation share (`implemented` + `verified`), per platform column:

| Category | Rows | windows | linux | macos | web |
|---|---|---|---|---|---|
| audio-codec | 19 | 9/19 (47.4%) | 9/19 (47.4%) | 9/19 (47.4%) | 0/19 (0.0%) |
| audio-feature | 11 | 0/11 (0.0%) | 0/11 (0.0%) | 0/11 (0.0%) | 0/11 (0.0%) |
| config-feature | 20 | 11/20 (55.0%) | 11/20 (55.0%) | 11/20 (55.0%) | 0/20 (0.0%) |
| container | 21 | 14/21 (66.7%) | 14/21 (66.7%) | 14/21 (66.7%) | 0/21 (0.0%) |
| extension | 1 | 0/1 (0.0%) | 0/1 (0.0%) | 0/1 (0.0%) | 0/1 (0.0%) |
| hotkey | 17 | 0/17 (0.0%) | 0/17 (0.0%) | 0/17 (0.0%) | 0/17 (0.0%) |
| menu-item | 16 | 0/16 (0.0%) | 0/16 (0.0%) | 0/16 (0.0%) | 0/16 (0.0%) |
| network-feature | 7 | 1/7 (14.3%) | 1/7 (14.3%) | 1/7 (14.3%) | 0/7 (0.0%) |
| platform-feature | 18 | 0/18 (0.0%) | 0/18 (0.0%) | 0/18 (0.0%) | 0/18 (0.0%) |
| playback-op | 13 | 4/13 (30.8%) | 4/13 (30.8%) | 4/13 (30.8%) | 0/13 (0.0%) |
| playlist-feature | 9 | 0/9 (0.0%) | 0/9 (0.0%) | 0/9 (0.0%) | 0/9 (0.0%) |
| playlist-format | 7 | 3/7 (42.9%) | 3/7 (42.9%) | 3/7 (42.9%) | 0/7 (0.0%) |
| plugin-feature | 4 | 0/4 (0.0%) | 0/4 (0.0%) | 0/4 (0.0%) | 0/4 (0.0%) |
| protocol-access | 11 | 0/11 (0.0%) | 0/11 (0.0%) | 0/11 (0.0%) | 0/11 (0.0%) |
| services-discovery | 6 | 0/6 (0.0%) | 0/6 (0.0%) | 0/6 (0.0%) | 0/6 (0.0%) |
| subtitle-format | 17 | 6/17 (35.3%) | 6/17 (35.3%) | 6/17 (35.3%) | 0/17 (0.0%) |
| text-render | 3 | 0/3 (0.0%) | 0/3 (0.0%) | 0/3 (0.0%) | 0/3 (0.0%) |
| ui-dialog | 15 | 4/15 (26.7%) | 4/15 (26.7%) | 4/15 (26.7%) | 0/15 (0.0%) |
| ui-screen | 11 | 9/11 (81.8%) | 9/11 (81.8%) | 9/11 (81.8%) | 0/11 (0.0%) |
| video-codec | 16 | 3/16 (18.8%) | 3/16 (18.8%) | 3/16 (18.8%) | 0/16 (0.0%) |
| video-feature | 10 | 0/10 (0.0%) | 0/10 (0.0%) | 0/10 (0.0%) | 0/10 (0.0%) |
| **TOTAL** | 252 | 64/252 (25.4%) | 64/252 (25.4%) | 64/252 (25.4%) | 0/252 (0.0%) |

Machine line: `parity: rows=252 windows=64/252 linux=64/252 macos=64/252
web=0/252`.

Verified-only (MP §1 strict definition — automated evidence): **0/252
(0.0%) on every platform, every category.**

## 5. The honest parity statement

> **Lumen's verified parity is 0/252, honestly.** 64 of 252 inventory
> features (25.4%) are `implemented` on each desktop platform (windows,
> linux, macos identically — the code is platform-shared and untested on
> real per-OS hardware) and 0 on web; **not one row claims `verified`**,
> because CI is billing-blocked on this private repository: every GitHub
> Actions run dies at 0 steps regardless of code (`worklog.md` 7.CLOSEOUT;
> risk R-16/R-20 refresh; `docs/RELEASE.md` known-limit #2). All landed
> evidence is therefore **local-run, `hardware_class = mock`** per doc 10
> §7.1 — green local gates with pinned provenance (workspace tests, the
> first-vertical recovery matrix, schema/codegen drift checks, cargo-deny, e2e/visual lanes), but no automated CI evidence, and no
> reference-hardware artifacts. Under MP §1's binding definition the only
> truthful verified-parity number is **zero, pending CI**.

What this statement does and does not claim:

- It does **not** mean nothing works: the 64 implemented rows carry
  committed evidence (tests, records, ADRs) that `feature-audit` resolves
  file-by-file (§3).
- It does **not** hide gaps: 133 platform cells are `blocked` (with
  `blocked_reason` citing an ADR/feasibility §/risk row — e.g. h264/hevc
  software decode, patent-gated codecs), 142 `partial` (with `notes`),
  36 `unsupported` (by design), 505 `not_started`.
- The `web` column is honestly 0: the wasm target is Phase 7 work
  (`worklog.md` 7.w9w-w11w; `docs/09-web-wasm-target.md`).
- Per-cell status changes require new evidence (R4) and are audited in git
  history; `not_started → verified` jumps are rejected by the audit design
  (doc 02 §4.3 anti-fabrication).

## 6. Decode-level parity vs the pinned VLC oracle (cross-reference)

Independent of the matrix, the deterministic decode differential vs
**VLC 3.0.23** (issue #19; real-oracle run 2026-11-09) measured, on the
35 lossless-comparable corpus rows (PCM-in-AVI, first vertical):
**29/35 bit-exact** (VLC's dump a bit-exact prefix; tail-unflush artifact)
and **0/6 pcm_f32le rows passing the ratified RMS ≤ 1e-4 / peak ≤ 1e-3
tolerances** — six honest parity gaps, unhidden. Full methodology, per-row
measurements and the open VLC-float-chain follow-up:
**`docs/PERF-VS-VLC.md`** (the issue #55 final report), raw records
`docs/corpus/vlc-diff-report-20261023.md`,
`docs/corpus/vlc-oracle-run-20261109.json`,
`docs/corpus/vlc-f32-tolerance-20261109.json`.

Frame-count/duration parity vs ffmpeg 7.1.5 on the corpus-v4 smoke sweep
(19/19 mjpeg frame counts exact; durations ≤ 0.001 ms on 12/19 comparable)
is recorded in `docs/perf/parity-v4-20261024.md`.

## 7. What flips rows to `verified` (the unblock path)

1. **CI billing restored** (user-owned) → the ci.yml lanes run: schema
   validation, cargo test/clippy/fmt, the first-vertical recovery matrix,
   config-propagation gates, wasm lane, visual/e2e lanes.
2. Evidence outbox per doc 10 §7.3 `[PROPOSAL]`: green per-platform job
   results + scenario IDs per row → transition proposals → Lead applies in
   a reviewable PR.
3. Hardware-dependent categories additionally require
   `hardware_class = real` artifacts per claimed platform (doc 10 §7.1) —
   the 3-OS runner blocker (`docs/plans/phase8-cards.md` §1).
4. Every known limit that keeps a row `blocked`/`partial` is consolidated
   in **`docs/KNOWN-LIMITS.md`**; release-specific limits (unsigned
   binaries, unexecuted packaging workflow) are registered in
   **`docs/RELEASE.md`** (wave `7.w22g`).
