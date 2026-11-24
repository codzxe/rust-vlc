# AUDIT: tools/corpus/speed_bench.py
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 210 / 8 functions |
| File verdict | REAL-UNPROVEN (as a Lumen-parity instrument: the Lumen side has ZERO committed evidence) |
| Reachable from shipped entry point? | NO (dev tool). Invoked by scripts/quality/27_speed_parity.sh:80 and scripts/w4-measure.sh:7; no committed run of either lane contains a Lumen row |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | Read; python3 -m py_compile OK; rg consumers; head docs/corpus/speed-baseline-20261022.md |

## 1. One-paragraph verdict
The tool itself is competently written and honest-by-construction: warmup
decode discarded (:85-86), fresh process per timing (:78), DNF never averaged
(:88-90), low-N marking (:189), median/p95 walls + child-CPU via rusage
(:48-50), JSON sidecar (:200-204). It compiles (py_compile OK). Its purpose,
however, is "Lumen vs ffmpeg (parity target)" (:2-4) — and the ONLY
committed output of this tool (docs/corpus/speed-baseline-20261022.md, via
scripts/w4-measure.sh:7) ran the ffmpeg side ALONE: 36 files, `runners:
ffmpeg`, every "ratio vs ffmpeg" cell "-". Not one Lumen measurement exists
in the tree, and the gate that would add teeth
(scripts/quality/27_speed_parity.sh) has no baseline memory file
(docs/audit/speed-baseline.json missing) and no committed run. The
"measured number, not an aspiration" claim (:5) is, for Lumen-vs-ffmpeg,
still an aspiration.

## 2. What actually works (verified)
— py_compile: OK (exit 0).
— Output-format round-trip: docs/corpus/speed-baseline-20261022.md carries
the exact table shape this script emits (fmt_row/fmt widths), classes with
low-N notes — the ffmpeg side ran for real (36 files, best-of-2).
— Runner plumbing verified by read: `--runner lumen='CMD {file}'` →
split("=",1) (:134) + `{file}` replacement (:74) — the lumen hook is real
code awaiting a run.

## 3. Findings
F-001 [S2]
Where: purpose vs committed evidence; docs/corpus/speed-baseline-20261022.md:1-3
Code: `corpus: \`corpus-big\` | files: 36 | best-of-2 | runners: ffmpeg`
What is wrong: the parity benchmark's committed baseline measures ONLY the
reference side. No Lumen runner was passed (scripts/w4-measure.sh:7 has no
--runner), so no row, no fps, no ratio exists for Lumen anywhere in the tree.
Why it matters: the doc named "speed-baseline" is the de-facto speed-parity
evidence; citing it as parity evidence would be citing a one-sided baseline.
What correct looks like: one run with `--runner lumen=...` re-committing the
md/json.
Confidence: HIGH — Verification: read + rg (no other speed_bench output in
the tree).
F-002 [S3]
Where: consumers — scripts/quality/27_speed_parity.sh:16-19 promises a
memory file; `docs/audit/speed-baseline.json` does not exist (ls).
What is wrong: the regression-detection gate has never completed a run that
writes its baseline; the tool is unwired in every committed lane.
Confidence: HIGH — Verification: ls + rg full_gate baseline (no speed
section).
F-003 [S4] :40-41 — FFMPEG/FFPROBE resolved from env with no version pinned
into the output (md/json record no ffmpeg version) → measurements are not
reproducible against a specific ffmpeg. Confidence: HIGH.
F-004 [S4] :176 — `str(len(per_file and [r for r in per_file if r['class'] == cls]))`
is an obfuscated way to write the class file count in the DNF row; works,
but is a readability land mine. Confidence: HIGH.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| sh | 44-45 | REAL | — | timeout honored; caller catches TimeoutExpired via Exception |
| child_cpu | 48-50 | REAL | — | RUSAGE_CHILDREN delta (approximates the whole child tree) |
| decode_frames_and_duration | 53-65 | REAL | — | ffprobe frames+duration; failure → (0, 0.0) → fps None |
| bench_one | 68-94 | REAL | — | warmup discard, DNF, best-of-N, median |
| fmt_row | 97-98 | REAL | — | md table row |
| main | 101-205 | REAL | S2/S4 | aggregate + p95 approx + ratio convention (ffmpeg_time/lumen_time, documented in 27_speed_parity.sh:15-16) |
| module | 1-41 | REAL | — | honest rules documented in the docstring |

## 5. Test honesty
No tests in this file (it IS a measurement tool). Its own honesty rules
(warmup/DNF/low-N) are implemented as documented — verified by read.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This tool makes that a MEASURED number, not an aspiration" | :4-5 | FALSE-for-Lumen | only the ffmpeg side is measured in the committed record |
| "ratio vs ffmpeg" capability | :14-15 | REAL-UNPROVEN | code path exists (:184-188); zero committed ratio values |
| ratio convention | scripts/quality/27_speed_parity.sh:15-16 | REAL | ffmpeg_time / lumen_time |

## 7. Dead / duplicated / unreachable code
None dead; the lumen-runner half of the tool is reachable-but-never-run
(see F-001).

## 8. Dependencies & flags
python3 stdlib only + external ffmpeg/ffprobe binaries (env-overridable).
No repo-crate deps.

## 9. Open questions for the next agent
Is `lumen decode-bench` (the CLI hook the runner template names,
27_speed_parity.sh:21) able to decode-to-null for arbitrary corpus files?
engine-info in rc-smoke says this build lands only mjpeg+pcm — a full-corpus
parity run would DNF on most classes.
