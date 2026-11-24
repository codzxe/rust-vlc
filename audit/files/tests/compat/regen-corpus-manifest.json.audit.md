# AUDIT: tests/compat/regen-corpus-manifest.json (parts 1-4, L1-2985)
| Field | Value |
|---|---|
| Auditor | W16-A01 |
| Date | 2026-11-10 (session) |
| Lines / entries | 2985 / 67 entries (31 deterministic, 36 runtime-random) |
| File verdict | REAL (data manifest; cross-checked against corpus-big manifest) |
| Reachable from shipped entry point? | NO — test infra only (consumed by tests/compat harness via compat-matrix.json:59) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 sha256 batch; python3 json cross-check vs docs/corpus/corpus-big-manifest-20261022.json |

## 1. One-paragraph verdict
131,927-byte JSON, sha256=60c27808529560d31e499d98db19087f2ebbcbfce660f5fdeea3b3e243986279, schema `compat-matrix/corpus-manifest/1`. 67 entries: ffmpeg argvs (with `__OUT__` placeholder), sha256 pins for deterministic rows, `deterministic` flag, purpose strings. This is committed-generated metadata describing how to regenerate a compat corpus on demand; the bytes themselves are NOT committed (cache_dir tests/compat/.regen-cache is gitignored runtime state). Provenance claims verified by sampling; one doc-level count mismatch found.

## 2. What actually works (verified)
- Entry `0022-avi-av.avi` argv matches corpus-big manifest `/files[19]/cmd` verbatim after removing the ffmpeg binary token and mapping the output path to `__OUT__` (python3 diff: argv==cmd minus binary/out). Provenance "argvs verbatim from docs/corpus/corpus-big-manifest-20261022.json" — TRUE for sampled entry.
- "19 of 35 are runtime-random realizations": rows 1-35 contain exactly 19 `deterministic:false` entries (python3 count). TRUE.
- Rows 66-67 (mpegps-dash.mpg, asf-head.wmv) are indeed absent from corpus-big (python3 set diff), matching the issue-#53 claim.
- Consumed by: tests/compat/compat-matrix.json:59 (`"manifest": "tests/compat/regen-corpus-manifest.json"`), tests/compat/src/config.rs:364 (test fixture copy), tests/compat/src/report.rs:1558 (report string). Execution requires ffmpeg (regen lane) — no ffmpeg run performed here (cargo/rustc absent; ffmpeg not exercised) — UNVERIFIED that regeneration actually reproduces pinned sha256s.

## 3. Findings
F-001 [S3]
Where: docs/corpus/compat-matrix-20261110.md:8 vs manifest entry count
Code (doc): `| Corpus | 46 committed + 54 malformed (error-contract) + 66 regenerable (manifest-anchored, `tests/compat/regen-corpus-manifest.json`) |`
What is wrong: the manifest contains 67 entries (`grep -c '"rel":'` = 67), the doc says 66 regenerable. Off-by-one in the published compat report. Why it matters: the compat report is the evidence artifact for the differential lane; its corpus-size claim is wrong. What correct looks like: 67, or document which entry is excluded. Confidence: HIGH — Verification: ran (grep + python3 len(entries)=67).
F-002 [S4]
Where: tests/compat/regen-corpus-manifest.json:2467
What is wrong: single 49,167-char line (giant `-metadata encoder=<random>` blob) — hostile to line-oriented tooling (Read tool of this very audit choked on it). Confidence: HIGH — Verification: ran (awk max line length).

## 4. Test honesty
Not a test file; it is fixture data for the compat harness. Honesty question is whether the harness treats a failed regeneration as failure — corpus.rs:251-257 records `regen_failed` as "a recorded fact", i.e. a failed ffmpeg regen becomes a recorded row outcome rather than a hard error; rows can be vacuous-green if the lane is not run (see compat harness audits, other agents). UNVERIFIED in full (harness execution not run here).

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "argvs verbatim from corpus-big manifest" | manifest provenance L4 | REAL | python3 cross-check entry 0022 |
| "19 of 35 runtime-random, honestly non-pinned" | provenance L4 | REAL | python3 count rows 1-35 nondet=19 |
| "66 regenerable" | docs/corpus/compat-matrix-20261110.md:8 | FALSE | 67 entries counted |
