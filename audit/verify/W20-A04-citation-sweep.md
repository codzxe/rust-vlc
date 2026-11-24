# W20-A04 — Citation Fraud Sweep (§8.1, 10% sample)

| Field | Value |
|---|---|
| Agent | W20-A04 (citation sweep, audit-of-the-auditors) |
| Date | 2026-11-26 |
| Population | 3,383 audit files (`audit/files/**` 2,550 + `audit/docs/**` 833) |
| `Where:` citations found | 3,932 regex matches; 3,468 have a ``` code block within 900 chars |
| Sample | 347 (10.0%, deterministic `random.Random(202604).sample`) |
| Checked | 296 (cited file exists + range ≤ 14 lines + non-empty quote) |
| Excluded | 51 RANGE>14 (cited ranges of 15–281 lines); 0 missing-source |
| Full CSV | audit/verify/W20-A04-citations.csv (347 rows, every field) |
| Script | audit/scratch/W20-A04/citation_sweep.py (reproducible, seeded) |

## 1. Headline result

| Classification | n | % of checked |
|---|---|---|
| VERBATIM | 258 | **87.2%** |
| ANCHOR-OFFSET | 38 | **12.8%** |
| WRONG-FILE (misattribution) | **0** | **0.0%** |
| NOT-PARSEABLE (text not found anywhere) | **0** | **0.0%** |

**Fabrication rate (wrong-file + not-found): 0 / 296 = 0.0%** — 95% CI upper
bound ≈ 1.0% (rule of three). **Citation-precision defect rate (anchor/format):
12.8%** — in every one of those 38 cases the quote text is real and present in
the cited file; only line anchors/formatting are imprecise.

Verbatim kinds: exact 137, subsequence (quote ⊂ cited range, line-exact) 106,
multiset-subset 2, alt-cited-file 13 (quote verbatim against a *second*
path:range listed in the same Where statement, e.g. "parser.rs:12-13 (doc)
vs 588-592 (code)" — all 13 hand-checked, all legitimate).

## 2. Method (mechanical; deviations documented)

Extraction: regex `Where:\s*([\w./-]+\.(rs|ts|tsx|toml|md|sh|json|yml|js|mjs|py|wxs|ps1|lock)):(\d+)(?:-(\d+))?`
followed within 900 chars by a ``` block. For each sampled citation where the
source exists and range ≤ 14 lines: real range read with sed semantics
(split on `\n` only — Python `splitlines()` drifts line numbers on files
containing `\x0c`/`\u2028`, found and fixed during classifier debug).

Classification order: VERBATIM (normalized equality / contiguous subsequence /
multiset subset, then internal-whitespace-collapsed as a flagged-loose last
resort — zero cases needed it); else ANCHOR-OFFSET if the quote's key line is
found in the cited file, or ≥50% of quote lines are present, or ≥70% of quote
tokens are present (reworded/reflowed quotes); else WRONG-FILE if the text
appears in another same-basename file (cap 6 checked); else NOT-PARSEABLE.

Refinements added after hand-verifying classifier edge cases against real
files (each verified with `sed -n 'A,Bp'` before adoption):
- R1 multi-cite: a Where statement listing several refs (incl. bare "vs 588-592"
  numbers inheriting the last cited path) — quote verified against ANY listed
  ref. Without this, honest multi-source citations misclassify as not-found.
- R2 "quoted above": Where lines saying "quoted above" take the PREVIOUS code
  block, not the next (else the next finding's block is mis-paired).
- Path resolution: as-is → audit's own subject → path-suffix match → subject
  sibling → global basename index. All 347 resolutions logged in the CSV
  (`cited_resolved_how`): 330 audit-subject, 16 as-is, 1 suffix-unique — no
  ambiguous basename-first resolutions in the sample.

## 3. Per-wave table (checked citations)

| Wave | n | verbatim | anchor/format | defect rate |
|---|---|---|---|---|
| W01 | 17 | 16 | 1 | 5.9% |
| W02 | 14 | 11 | 3 | 21.4% |
| W03 | 16 | 13 | 3 | 18.8% |
| W04 | 41 | 32 | 9 | **22.0%** |
| W05 | 12 | 11 | 1 | 8.3% |
| W06 | 15 | 14 | 1 | 6.7% |
| W07 | 16 | 16 | 0 | 0% |
| W08 | 21 | 21 | 0 | 0% |
| W09 | 16 | 14 | 2 | 12.5% |
| W10 | 30 | 25 | 5 | 16.7% |
| W11 | 16 | 13 | 3 | 18.8% |
| W12 | 15 | 15 | 0 | 0% |
| W13 | 12 | 11 | 1 | 8.3% |
| W14 | 5 | 4 | 1 | 20.0% |
| W15 | 14 | 12 | 2 | 14.3% |
| W16 | 12 | 11 | 1 | 8.3% (incl. RR2 recovery audits: 6/6 verbatim) |
| W17 | 22 | 17 | 5 | **22.7%** |
| W18 | 2 | 2 | 0 | 0% |

W19: no sampled citations — wave 19 audited 724 mostly data/doc files and
produced only 29 Where-citations in the whole population (≈3 expected in a 10%
sample; 0 landed). W20 verification outputs live under audit/verify/, not
files/docs, and are out of this sweep's scope.

## 4. Per-agent outliers

Worst 5 by defect rate, n ≥ 3 (defect = anchor/format; NO agent has a
wrong-file or not-found case):

| Agent | n | defects | rate |
|---|---|---|---|
| W04-A07 | 3 | 2 | 67% |
| W17-A09 | 3 | 2 | 67% |
| W04-A05 | 4 | 2 | 50% |
| W10-A04 | 4 | 2 | 50% |
| W04-A04 | 7 | 3 | 43% (largest absolute count) |

At n ≥ 5 only two agents show defects at all: W04-A04 3/7 and W17-A04 1/6.
Clean at n ≥ 5 with zero defects: W01-A06, W04-A01, W04-A02, W04-A08,
W05-A09, W06-A01, W10-A05 (1/6), W10-A10, W12-A07 (n=4), W16-RR2 (6/6
verbatim — the recovery re-run's citations are clean).

## 5. Anchor-defect anatomy (all 38 cases: text real, anchor imprecise)

- Drift |actual − cited| for the 33 key-line cases: median **1 line**; 27/33
  within ±1, 30/33 within ±3. Outliers: idx.rs cited 53-55, text at 144
  (drift 92); recovery_env.rs cited 1474, composite elided quote whose lines
  live at 914+ (drift 560, spliced elision with "..." markers — counted as
  anchor defect, text present).
- Justification branches: 33 key-line-found, 4 reworded/reflowed (≥70% tokens
  present — e.g. e2e.rs `//!` vs `///` comment-marker swap; 26_complexity.sh
  "The" dropped; tokens.json single-line "..." truncation), 1 half-lines
  present (deny.toml paren dropped — the wave-1 gate's known defect).
- Pattern matches the orchestrator register's ANCHOR DEFECT policy (quote text
  real and present in the cited file; substance stands).

## 6. Special section — finding-fabrications and gate history (not citation stats)

- **W12-A06 [S0] FINDING-FABRICATION — REFUTED by W20-A02.** The wave-12 S0
  claimed apps/web/src/io/mod.rs's evidence label "cites a fetch-lane playback
  proof over the REAL first-vertical fixture that does not exist". W20-A02
  disproved it by execution (audit/verify/W20-A02-reverification.md Finding 9):
  the proof test `fetch_lane_serves_the_real_fixture_to_the_demuxer`
  (staged_source.rs:1159-1218) exists and PASSES (`cargo test -p lumen-web
  ... ok`), and the fixture tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
  exists (425,516 bytes, MANIFEST.toml-declared, sha256 pinned). A false S0 —
  the only confirmed finding-level fabrication in the audit corpus; it is a
  fabrication of a *negative* claim, not of quoted code, so a citation sweep
  alone cannot catch this class.
- **Orchestrator-gate misattributions (repaired, historical):** W1 — W01-A08
  attributed host.rs:1798-1799 code to check_identity.rs:13-24 (re-run
  executed; repaired file byte-verified 5/5 blocks); W4 — 1 misattribution
  (orchestrator-repaired; 243 checked, 230 verbatim); W5 — 1 wrong-file
  citation (repaired; 118 checked, 111 verbatim). None of these surfaces in
  this 10% sample; my sweep found **zero new** misattributions, consistent
  with the repairs holding.

## 7. Verdict on the audit-of-the-auditors question

On citation mechanics the corpus is clean at the 10% sample: **0/296
fabricated, invented, or misattributed quotes** (upper bound ≈1%); every quote
checked traces to real text in the cited file. What does not hold is anchor
precision: ~13% of citations carry line-number or formatting defects
(median drift 1 line), concentrated in waves 04 and 17 (≈22%) and a handful
of agents (W04-A04/A05/A07, W10-A04, W17-A09). These do not change finding
substance under the register's policy but do mean citations cannot be trusted
as line-exact pointers without re-verification. Separately, one proven
finding-level fabrication exists (W12-A06's refuted S0), showing the
falsification risk in this audit lives in *claims about existence/execution*
(evidence labels, fixtures, CI run ids — e.g. e2e.rs's unverifiable CI-run
narrative, deny.toml's gitignored evidence dir), not in quoted code.
Recommended: keep per-wave citation gates (they work — every flagged case
above was caught by a gate or this sweep); add an automated existence-check
for evidence-path citations (tmp/, CI run ids) in the gate, since that is the
one fabrication vector with a confirmed instance.

## 8. Reproducibility

`python3 audit/scratch/W20-A04/citation_sweep.py` — deterministic (seed
202604), read-only outside audit/, no git. Outputs: this report +
audit/verify/W20-A04-citations.csv + audit/scratch/W20-A04/sampled.json.
Classifier iterations logged in audit/runs/W20-A04.log.md (5 debug
fixes, each validated by hand `sed` comparison before adoption; final-run
numbers only are reported above).
