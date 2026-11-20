# CLAIM AUDIT: docs/PARITY-REPORT.md

| Field | Value |
|---|---|
| Auditor | W19-A01 |
| Date | 2026-11-16 |
| Claims extracted | 26 | REAL 15 | REAL-UNPROVEN 1 | SHALLOW 1 | STUB 0 | PAPER-ONLY 1 | FALSE 8 | UNVERIFIED 0 |
| Internal contradictions found | 1 |

## 1. Blunt verdict

The document's spine is honest — verified parity is genuinely 0/252, the tooling is genuinely read-only, and the VLC differential numbers it cites are real committed artifacts. But every operational number in the file is a 2026-11-10 snapshot presented as "the finalized honest parity statement", and at HEAD 727ad6b2 (six days and waves w27/w30/w31 later) it is wrong on every count that matters: the audit it quotes as PASS now FAILS with 2 R4 violations, its per-platform totals (64/252 "identically") are now 66/61/66 and non-identical, its cell counts are off by 4–5 in three buckets, and its central causal claim ("CI is billing-blocked, every run dies at 0 steps") is contradicted by four refreshed docs and the repo's own w30.10 remediation worklog. A parity report whose numbers are stale is a broken parity report: this one certifies a world that ended on 2026-11-10.

## 2. Claim ledger

| # | Claim (verbatim ≤25 words) | Location (path:line) | Verdict | Evidence you gathered |
|---|---|---|---|---|
| 1 | "Computed 2026-11-10 from `docs/feature-matrix.toml` at this commit" | docs/PARITY-REPORT.md:7 | REAL | Date/method plausible; commit `c9a89c6` (2026-11-10) in git log. But see #9/#13 — outputs no longer match HEAD. |
| 2 | "252 rows / 21 categories" | docs/PARITY-REPORT.md:8 | REAL | Independent parse of docs/feature-matrix.toml: 252 `[[feature]]` blocks, 21 categories (audio-codec 19 … video-feature 10). |
| 3 | "schema frozen 2026-10-19 (doc 02 §4.7)" | docs/PARITY-REPORT.md:8 | REAL | Matrix header: "SCHEMA FROZEN 2026-10-19" (docs/feature-matrix.toml:29). |
| 4 | "populated by task 0.D2 from the completed VLC 3.0.23 inventories" | docs/PARITY-REPORT.md:8 | REAL | Matrix header "Population: Task 0.D2"; docs/inventory/matrix-seed-notes.md exists. |
| 5 | "Baseline: VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd)" | docs/PARITY-REPORT.md:9 | REAL-UNPROVEN | Pinned in docs/baselines/vlc-3.0.23*.md with per-platform raw records; external check impossible from sandbox. |
| 6 | "Both tools live in `tools/xtask` (std-only, zero dependencies)" | docs/PARITY-REPORT.md:28 | REAL | tools/xtask/src/cmd/{feature_audit,parity_report}.rs exist; parity_report.rs contains only `std::fs::read_to_string` (no write path). |
| 7 | "nothing is hardcoded and no parity percentage is ever stored" | docs/PARITY-REPORT.md:29-30 | REAL | parity-report output derives at run time (audit/runs/baseline/xtask-parity-report.txt); no percentage field in the matrix. |
| 8 | "no code path exists that edits the matrix" | docs/PARITY-REPORT.md:39-40 | REAL | `rg -n "write" tools/xtask/src/cmd/parity_report.rs` → only read_to_string (line 98). |
| 9 | "result: PASS" (§3 quoted ACTUAL run) | docs/PARITY-REPORT.md:70 | FALSE | Executed at HEAD: `result: FAILURE (2 violation(s))`, EXIT=1 — violations at matrix rows 23 (`crates/codecs/src/video/h264/sw`) and 163 (`ui/src/screens/settings/areas/advanced.tsx`) (audit/runs/baseline/xtask-feature-audit.txt). |
| 10 | "per-platform-status cells: blocked=133 implemented=192 not_started=505 partial=142 unsupported=36" | docs/PARITY-REPORT.md:68-69 | FALSE | HEAD run: implemented=193, partial=147, blocked=129, not_started=503, unsupported=36 — three of five buckets wrong (audit/runs/baseline/xtask-feature-audit.txt; reconciles with my own matrix parse 66+61+66+0=193 implemented cells). |
| 11 | "audit passed only after this card fixed two stale evidence-path spellings (commit `c9a89c6`)" | docs/PARITY-REPORT.md:73-75 | REAL | `git show c9a89c6` exists (2026-11-10): "fix 2 evidence path spellings … feature-audit R4 now resolves; ACTUAL run PASS 252 rows". |
| 12 | "The 252/252 default-agreement inventory cross-check … recorded in `docs/gates/phase0-exit-report.md` row 3" | docs/PARITY-REPORT.md:78-80 | REAL | phase0-exit-report.md:12 — "1,476 records, 100% coverage, 252/252 default agreement". |
| 13 | TOTAL "64/252 (25.4%)" on windows/linux/macos | docs/PARITY-REPORT.md:109 | FALSE | HEAD: windows 66/252 (26.2%), linux 61/252 (24.2%), macos 66/252 (audit/runs/baseline/xtask-parity-report.txt machine line; my independent matrix count agrees). Stale cells include protocol-access 0→1, video-codec 3→4 (windows), audio-codec linux 9→6, container linux 14→13. |
| 14 | "Machine line: … windows=64/252 linux=64/252 macos=64/252 web=0/252" | docs/PARITY-REPORT.md:111-112 | FALSE | HEAD machine line: "windows=66/252 linux=61/252 macos=66/252 web=0/252" (audit/runs/baseline/xtask-parity-report.txt). |
| 15 | "Verified-only … 0/252 (0.0%) on every platform, every category" | docs/PARITY-REPORT.md:114-115 | REAL | My matrix parse: `verified` count = 0 in all four platform columns at HEAD; executed parity-report agrees. |
| 16 | "Lumen's verified parity is 0/252, honestly" | docs/PARITY-REPORT.md:119 | REAL | Still true at HEAD (verified=0 everywhere); the strongest claim in the doc survived six days of churn. |
| 17 | "64 of 252 … implemented on each desktop platform (windows, linux, macos identically — the code is platform-shared…)" | docs/PARITY-REPORT.md:120-122 | FALSE | Not identical at HEAD: linux=61 vs windows/macos=66. Five rows implemented on windows+macos but partial on linux: container-asf-wmv, codec-video-vp9, codec-audio-pcm, codec-audio-aac-lc, codec-audio-alac (matrix parse). |
| 18 | "CI is billing-blocked on this private repository: every GitHub Actions run dies at 0 steps regardless of code" | docs/PARITY-REPORT.md:123-126 | FALSE | Contradicted by the refreshed doc set: RELEASE.md:145-149 ("workflow has executed (v0.1.0–v0.1.2). Billing was unblocked 2026-11-12"), KNOWN-LIMITS.md:17, README.md:127-131, release-checklist.md:238-248, and worklog.md:2140-2144 (w30.10: the "no workflow executed / no release published" claims were "false since 2026-11-12/13"). Its own citation target (RELEASE.md known-limit #2) now says the opposite of what this doc asserts. |
| 19 | "All landed evidence is … local-run, `hardware_class = mock` per doc 10 §7.1" | docs/PARITY-REPORT.md:126-128 | REAL | doc 10 §7.1 defines `hardware_class: mock (VirtualClock, null/software sinks, CI VMs)` (docs/10-testing-ci-quality.md:805); all evidence in-tree is local-run. |
| 20 | "the 64 implemented rows carry committed evidence … that `feature-audit` resolves file-by-file (§3)" | docs/PARITY-REPORT.md:134-136 | FALSE | At HEAD feature-audit FAILS: two implemented rows cite paths that do not exist (rows 23/163 — `crates/codecs/src/video/h264/sw`, `ui/src/screens/settings/areas/advanced.tsx`). "Resolves file-by-file" also means path-existence only: the empty-file experiment (audit/runs/baseline/experiment-feature-audit-empty-file.txt) shows a 0-byte file certifies "implemented" on all 4 platforms, PASS exit 0. |
| 21 | "133 platform cells are `blocked` …, 142 `partial`, 36 `unsupported`, 505 `not_started`" | docs/PARITY-REPORT.md:137-140 | FALSE | HEAD: 129 blocked, 147 partial, 36 unsupported, 503 not_started (executed feature-audit; my parse sums per platform: 34+34+35+26=129, 46+51+46+4=147, 99+99+98+207=503). |
| 22 | "The `web` column is honestly 0" | docs/PARITY-REPORT.md:141 | REAL | web implemented=0 at HEAD (my parse; executed parity-report web=0/252). |
| 23 | "`not_started → verified` jumps are rejected by the audit design (doc 02 §4.3 anti-fabrication)" | docs/PARITY-REPORT.md:143-145 | SHALLOW | The doc's own §2 (lines 52-53) admits the git-history jump rejection "needs a `--from` revision argument (v0 audits the working tree)" — i.e., NOT implemented. Combined with the empty-file experiment, the "anti-fabrication rule" is an aspiration plus path-existence. |
| 24 | "29/35 bit-exact … and 0/6 pcm_f32le rows passing the ratified RMS ≤ 1e-4 / peak ≤ 1e-3 tolerances" | docs/PARITY-REPORT.md:150-155 | REAL | docs/corpus/vlc-oracle-run-20261109.json + vlc-f32-tolerance-20261109.json exist; JSON policy block matches the thresholds verbatim; per-row values match §3's table (0022: samples 139244). |
| 25 | "19/19 mjpeg frame counts exact; durations ≤ 0.001 ms on 12/19 comparable" | docs/PARITY-REPORT.md:161-163 | REAL | docs/perf/parity-v4-20261024.md:18 ("19/19 decoded mjpeg streams EXACT") and :45 ("12/19 comparable files agree to ≤ 0.001 ms"). |
| 26 | §7 unblock path: "CI billing restored → the ci.yml lanes run", evidence outbox per doc 10 §7.3 | docs/PARITY-REPORT.md:165-172 | PAPER-ONLY | The doc itself says the collect/outbox machinery is "[PROPOSAL] design, deliberately not built" (lines 40-42); nothing in the tree implements it. |

## 3. Contradictions

| This doc says | Other source says (path:line) | Which is true (proof) |
|---|---|---|
| "every GitHub Actions run dies at 0 steps" (PARITY-REPORT.md:123-125) | RELEASE.md:145-149 "The workflow has executed (v0.1.0–v0.1.2). Billing was unblocked 2026-11-12"; worklog.md:2140-2144 calls the blocked wording "false since 2026-11-12/13" | Unprovable from inside the tree (no GitHub Actions access) — but both cannot stand; the in-repo record weight (worklog w30.10/w28.2/w28.3, release-checklist §9, local tags v0.1.0–v0.1.3) supports "executed"; PARITY-REPORT is the stale side (its own §7 even lists "CI billing restored" as the unblock step). |
| §4/§5: 64/252 identical, cells 192/142/133/505, "result: PASS" | Executed feature-audit at HEAD: 66/61/66, cells 193/147/129/503, FAILURE 2 R4 (audit/runs/baseline/xtask-feature-audit.txt) | The current run. KNOWN-LIMITS.md:17 itself concedes "the PARITY-REPORT refresh is owed by its owner". |
| §5: "the 64 implemented rows carry committed evidence … resolves file-by-file" | audit/runs/baseline/xtask-feature-audit.txt: "violation: 23: evidence cites missing path `crates/codecs/src/video/h264/sw` (R4)" | The current run — two implemented rows have dangling evidence at HEAD. |
| §2 (lines 44-47): "feature-audit rejects evidence-less flips at audit time" | audit/runs/baseline/experiment-feature-audit-empty-file.txt: fabricated row + 0-byte evidence file → "result: PASS … EXIT=0" | The experiment — the audit's certification standard is filesystem existence, nothing more. |

## 4. Fabrication/inflation indicators

1. **Stale-numbers-as-final**: the doc's title says "the finalized honest parity statement", yet its §3/§4 numbers were already superseded by w27 (h264 flip, 2026-11-15) and w31 (streaming/canvas flips) — the file was never re-run after the state it certifies changed.
2. **The empty-file certification gap**: the anti-fabrication framing ("implemented-with-evidence, never a bare flip") rests on a check the repo's own hostile experiment proved certifies a 0-byte file as implemented on 4 platforms. Evidence presence ≠ evidence content.
3. **Citation inversion**: §5 cites "docs/RELEASE.md known-limit #2" as support for billing-blocked — that target now states the exact opposite (workflow executed, billing unblocked). The doc cites a refuted source as standing proof.
4. **Symmetry fiction**: "windows, linux, macos identically — the code is platform-shared" — at HEAD five codec rows break the symmetry (linux partial vs windows/macos implemented), so the identical-platforms sentence was either never true for the current matrix or stopped being true without the doc noticing.
