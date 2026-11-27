# PARITY-REPORT — the finalized honest parity statement (issue #57, cards 8.F1–8.F2)

> **Role — demoted 2026-11-26 (Devil-Fix wave FX8, agent FX8-A10; FIX-PLAN
> §1 Principle 1 + §8.4).** This file owns the **billing record** only:
> the CI billing history and the six live run IDs with their real outcomes
> (§5) — billing truth. It **no longer owns status**: this file was one of
> the audit's nine hand-edited status layers (DEVIL-AUDIT-REPORT §5 #1)
> and its counts drifted stale against the very matrix they described
> (CONTRADICTIONS C-020). Every status/capability number that used to live
> here is a pointer to the two single sources: the ledger
> **`docs/feature-matrix.toml`** (252 rows — swept to rg-verified code truth
> 2026-11-26, FX8-A03) and the generated status truth
> **`docs/generated/status-truth.toml`** (regen:
> `python3 scripts/quality/status_truth.py > docs/generated/status-truth.toml`;
> drift gate: `bash scripts/quality/status_truth_check.sh` — the ci.yml
> `status-truth` lane, FX8-A01/A02).

| Field | Value |
|---|---|
| Purpose | The Phase 8 parity statement (MP §16): exactly what is verified, partial, unsupported and blocked — with evidence links, per MP §17 and doc 10 §7 |
| Card / issue | 8.F1 + 8.F2 / GitHub issue #57 (finalization lane `7.w22h`) |
| Computed | 2026-11-10 from `docs/feature-matrix.toml` at this commit, by the tools in §2 (the run receipts are retained in §3; the status snapshots those runs printed are demoted to pointers — see the Role note; numbers are recomputed at read time, never stored) |
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

## 3. The 252-row audit status — demoted to pointer (2026-11-26, FX8-A10)

The 2026-11-10 run this section recorded printed the then-current
per-phase/per-category/per-status counts as a verbatim block — a
hand-copied status snapshot, and exactly the drift class the audit
registered against this file (CONTRADICTIONS C-020: "PARITY-REPORT's own
counts are stale vs the live matrix it describes"). Per Principle 1 the
counts no longer live here. **The live audit status is the validating
tool's own run against the live single sources, never this doc:**

- the ledger: `docs/feature-matrix.toml` (252 rows; FX8-A03's same-wave
  sweep re-stated 37 of them to rg-verified code truth, 2026-11-26);
- the generated status truth: `docs/generated/status-truth.toml`
  (decoder families / host verbs / catalog commands, parsed from source —
  regen + gate commands in its header);
- the tool: `cargo xtask feature-audit` (§2) — re-run it; its output is
  the audit status, at read time.

The audit passed only after this card fixed two stale evidence-path
spellings (commit `c9a89c6`: `tests/https_activation.rs` →
`crates/streaming/tests/https_activation.rs`, `tests/pcm_pipeline.rs` →
`crates/audio-output/tests/pcm_pipeline.rs` — both files exist; the tokens
had been written crate-relative and failed root-relative R4 resolution;
**no status cell changed**). The 252/252 default-agreement inventory
cross-check (Phase 0) is separately recorded in
`docs/gates/phase0-exit-report.md` row 3.

## 4. Derived parity table — demoted to pointer (2026-11-26, FX8-A10)

The 2026-11-10 table this section used to carry (21 categories × 4
platforms, the `parity: rows=252 …` machine line, the verified-only
0/252 line) was a hand-stored copy of numbers the ledger already owns —
registered stale by the audit (CONTRADICTIONS C-020) and demoted per
Principle 1. Parity numbers are **computed at read time, never stored**
(the matrix header rule, doc 02 §4.7):

```
cargo xtask parity-report        # the derived per-category/per-platform table
cargo xtask feature-audit        # the structural + evidence validation
```

The two single sources the numbers derive from: the ledger
`docs/feature-matrix.toml` (FX8-A03's 2026-11-26 sweep) and the generated
status truth `docs/generated/status-truth.toml` (FX8-A01; drift gate
`bash scripts/quality/status_truth_check.sh` — the ci.yml `status-truth`
lane, FX8-A02).

## 5. The honest parity statement — numbers demoted, billing retained (2026-11-26, FX8-A10)

> **Parity numbers no longer live in this file** (Principle 1 — one
> status layer, generated from the code; the audit registered this
> file's own counts stale against the live matrix it described —
> CONTRADICTIONS C-020). Verified-parity and implementation-share
> numbers are computed at read time from the two single sources — the
> ledger `docs/feature-matrix.toml` (252 rows; FX8-A03's 2026-11-26
> rg-verified sweep) and the generated status truth
> `docs/generated/status-truth.toml` (FX8-A01) — by the §2 tools
> (`cargo xtask parity-report` / `cargo xtask feature-audit`); the
> matrix-header rule stands: parity percentages are computed, never
> stored (doc 02 §4.7).
>
> **What this file still owns — billing truth:** CI billing was
> unblocked 2026-11-12 and the
> workflows HAVE executed (v0.1.0–v0.1.2) — the six
> in-tree live run IDs and their real outcomes, per
> `audit/final/CONTRADICTIONS.md` C-005 (the pre-unblock CI-blockade
> claim that stood in this paragraph — every Actions run dead at 0
> steps regardless of code — is registered FALSE there;
> side B = `docs/RELEASE.md:145-148` + `worklog.md:1625/1968/2065`):
>
> - **43378741271 / 43378741286 / 43378757248** — the first full real
>   execution of all five workflows (`worklog.md:1625`); the triage
>   recorded there: UI visual lane 13/50 on the runner (host-font
>   divergence → Inter bundled), gate step 3 cargo-test died pre-build
>   (→ build-before-test), release dry-run 4 lanes died (script exec
>   bits + MSI-guard CR) — each fixed in-band.
> - **43464162696** — the v0.1.0 tag run: all five OS jobs green, the
>   release job green through checksums + notes + tag cross-check, the
>   final step died `gh release create` HTTP 422 "body is too long"
>   (`worklog.md:1968`).
> - **43466814633** — the v0.1.0 release published (17:00:58Z, tag
>   `v0.1.0` @ `a64b7030`) with the MSI upgrade-path test green
>   (`docs/RELEASE.md:195`, `:210-211`).
> - **43729218652** — the v0.1.1 release-lane failure: MSI upgrade
>   test exit 1603 (windows job) — the gen-wxs.sh heredoc twin
>   divergence, fixed by the w28.1 hotfix with self-test pins +
>   `scripts/twin_parity.py` (`worklog.md:2065`).
>
> The runs above are CI-lane and release-pipeline executions; whether
> any row's evidence class (`hardware_class` per doc 10 §7.1) or its
> `verified` state moved after them is the ledger's business
> (`docs/feature-matrix.toml` per-row `evidence`; the rules: R4,
> doc 10 §7.1) — never recounted here.

What this statement does and does not claim (post-demotion):

- It does **not** mean nothing works — and it no longer counts what
  does: the implemented/partial/blocked/unsupported truth is the
  ledger's (`docs/feature-matrix.toml`, per-row `evidence` that
  `feature-audit` resolves file-by-file, `blocked_reason` citing an
  ADR/feasibility §/risk row where required — R5).
- It does **not** hide gaps: every `blocked`/`partial` cell and its
  reason is in the ledger; the consolidated limits register is
  `docs/KNOWN-LIMITS.md`.
- The `web` column's state is the ledger's (the wasm target is Phase 7
  work — `worklog.md` 7.w9w-w11w; `docs/09-web-wasm-target.md`).
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
