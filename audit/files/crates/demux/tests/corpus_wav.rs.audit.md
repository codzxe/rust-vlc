# AUDIT: crates/demux/tests/corpus_wav.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 381 / 2 tests + 1 helper + 12-row truth table |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — integration test; runs (or skips) under cargo test; baseline :2224-2228 shows both "ok" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `test -d /tmp/my-project/corpus-v4/media` → DIR MISSING; rg for the claimed worklog table across worklog.md (2298 lines) and the repo → not found; python3 reads; sed quote checks; baseline citation :2224-2228 |

## 1. One-paragraph verdict
A genuinely strict differential suite in design — 12 corpus WAVs pinned to
ffprobe parameters and ffmpeg raw-dump MD5s, with the ADPCM fact-lie
discrepancy (0042/0102) documented, damage-counted, and asserted in
1024-byte block units. But in THIS environment the corpus is absent
(`/tmp/my-project/corpus-v4/media` verified missing), so both tests take
the loud-skip path and the baseline's "ok" for them is a vacuous pass: the
12 differential rows have never executed in any evidence I can see. The
module doc's evidence pointer — "the per-file PASS/FAIL table for this
instance lives in the worklog entry" — does not resolve: worklog.md
contains no corpus(wav) table (rg verified across the repo). The pinned
constants are internally consistent with the twin tables in
crates/codecs/tests/adpcm_corpus_differential.rs (0042 truth 168_822) and
apps/cli/src/containers/wav.rs:467, which is corroboration but not
execution.

## 2. What actually works (verified)
- Consistency cross-check (read): the 0042 ADPCM numbers (probe 167_853 /
  truth 168_822 / 166×1024 blocks) and the `fact-frames-short-of-data`
  reason string are identical in three independent files
  (this file :130-131, codecs/tests/adpcm_corpus_differential.rs:23,87,
  apps/cli/src/containers/wav.rs:467).
- Baseline executed the file: cargo-test-workspace.txt:2227-2228
  `corpus_wavs_survive_the_fuzz_harness ... ok`,
  `corpus_wavs_match_the_ffmpeg_truth ... ok` — but see F-001: both were
  skips in that run (corpus absent in this sandbox).
- The skip path itself is loud in code (eprintln :194-197) — invisible in
  a non---nocapture baseline log, which is the core of F-001.

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/corpus_wav.rs:8-14 (with baseline
audit/runs/baseline/cargo-test-workspace.txt:2228)
Code:
```
//! corpus regenerates with fresh seeds: when it is absent (and no
//! `LUMEN_CORPUS` override is set) this suite **skips loudly** — the
//! per-file PASS/FAIL table for this instance lives in the worklog
//! entry, exactly as measured.
```
What is wrong: two claims fail. (1) The skip is loud only to stderr —
in the orchestrator baseline (no --nocapture) it surfaces as
`corpus_wavs_match_the_ffmpeg_truth ... ok`, indistinguishable from a
passing 12-file differential; the corpus dir is missing in this sandbox
(`test -d` verified), so that "ok" proved zero rows. (2) The evidence
pointer is dead: neither worklog.md (2298 lines) nor any repo file
contains the claimed per-file PASS/FAIL table (rg for `corpus(wav)`,
`0042`, `168_822` table rows — only the three constant tables exist).
The card 5.A2 "real-corpus WAV differential" therefore has NO executable
or archived evidence in this environment. Why it matters: a green
baseline line that proves nothing is exactly the fabricated-evidence
shape the audit exists to catch; the pinned values may well be genuine
(they are cross-corroborated) but nothing here demonstrates it. What
correct looks like: `#[ignore]` + an explicit env-gated runner (like the
ignored long-fuzz lane), or committing a small corpus slice, or actually
committing the measured table the doc points to. Confidence: HIGH —
Verification: ran (`test -d` missing) + read; differential itself
UNVERIFIED (corpus absent; cargo-build-budget irrelevant — no bytes to
run against).

F-002 [S4]
Where: crates/demux/tests/corpus_wav.rs:274
Code:
```
                Ok(DemuxEvent::Discontinuity) => {}
```
What is wrong: same silent swallow as wav_aiff_fixtures.rs F-001 — an
unexpected Discontinuity would go unasserted in the "clean walk"
differential. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CorpusTruth + TRUTHS (12 rows) | 44-179 | REAL-UNPROVEN | S3 | pinned constants; internally cross-corroborated; not executable here |
| corpus_dir | 183-189 | REAL | — | LUMEN_CORPUS override else sandbox default; None when absent |
| corpus_wavs_match_the_ffmpeg_truth | 191-343 | REAL-UNPROVEN | S3 | strict assertions (codec/base/span/frames/md5/clean-EOS/damage math (truth−probe)×1024) — never ran here; F-001 |
| corpus_wavs_survive_the_fuzz_harness | 348-381 | REAL-UNPROVEN | — | fuzz sweep incl. dirty/garbage siblings when present; skipped here |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_wavs_match_the_ffmpeg_truth | YES *where the corpus exists* | exact md5/frames/damage asserts; in this environment it cannot fail at all (skip = vacuous ok) — F-001 |
| corpus_wavs_survive_the_fuzz_harness | YES *where the corpus exists* | same skip shape |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every pinned value below was measured this session from the corpus-v4 instance" | :8-10 | REAL-UNPROVEN | values cross-corroborated in 3 files; measurement act not re-executable (corpus absent) |
| "the per-file PASS/FAIL table for this instance lives in the worklog entry" | :13-14 | FALSE (as a repo pointer) | rg over worklog.md + repo: no such table exists |
| "skips loudly" | :12 | PARTIAL | loud on stderr only; silent in pass/fail test result lines |
| ADPCM fact discrepancy numbers (0042: 167_853 vs 168_822; 0102: 128_000 vs 128_524) | :21-24 | REAL-UNPROVEN | consistent with adpcm_corpus_differential.rs:23 and containers/wav.rs:467; not executable here |

## 7. Dead / duplicated / unreachable code
None dead; the skip paths are the file's dominant executed path in this
environment.

## 8. Dependencies & flags
Integration test; dev-deps testkit/input/demux/media-types/plugin-api;
`mod common` (shared MD5 + open_via_input). No feature gates. Corpus via
LUMEN_CORPUS env or /tmp default.

## 9. Open questions for the next agent
- Whoever audits crates/codecs/tests/adpcm_corpus_differential.rs and
  apps/cli/src/containers/wav.rs: same corpus-absence caveat applies to
  their differential lanes.
- Is there any committed archive of the "2026-10-24 instance" table the
  doc references? If it exists outside the repo, the doc should say so.
