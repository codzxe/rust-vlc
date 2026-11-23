# AUDIT: crates/demux/tests/ts_corpus.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 258 / 2 tests + 2 helpers + 5-row truth table |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — integration test; baseline :2409-2415 shows both "ok" (as skips — corpus verified absent) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `test -d /tmp/my-project/corpus-v4/media` → DIR MISSING (same check as corpus_wav.rs); python3 reads; sed quote checks; baseline citation :2409-2415 |

## 1. One-paragraph verdict
Identical evidence shape to corpus_wav.rs: a strict, well-designed
ffprobe-differential (program count, PMT/PCR PIDs, stream types, SDT
names, packet totals, CRC/sync/TEI health) over five corpus-v4 TS lanes —
none of which can run here because the corpus directory is verified
missing. Both tests took the skip path in the orchestrator baseline and
still report `ok` (baseline :2413-2414, 0.02s), so the "real-corpus MPEG-TS
differential" of card 5.I1a has zero executed evidence in this
environment. Unlike corpus_wav.rs, no dead worklog pointer — but the
module doc's "pre-Rust byte walk confirmed every section's CRC-32/MPEG-2
hash to zero" is an unverifiable prior-session claim. The test structure
explicitly passes when `ran == 0`.

## 2. What actually works (verified)
- Baseline compiled and ran the file: cargo-test-workspace.txt:2409-2415
  (`corpus_ts_programs_streams_and_names_match_ffprobe ... ok`,
  `corpus_ts_pes_seam_accounting ... ok`) — as skips (corpus absent,
  `test -d` verified).
- The assertion set is genuinely strict where the corpus exists: PID-level
  equality against ffprobe, descriptor strings, service names, exact
  packet totals (408/331/723/621/96), zero CRC failures / sync errors /
  TEI, positive PES counts (read).
- Sibling TS suites that DO run on committed bytes pass: ts_fuzz_survival
  3/3 (baseline :2417-2423) and ts::pes unit tests (:2076-2078).

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/ts_corpus.rs:110-123 and 216-223
Code:
```
    let dir = corpus_dir();
    let mut ran = 0;
    for truth in &TRUTHS {
        let path = dir.join(truth.file);
        if !path.exists() {
            eprintln!(
                "SKIP (loud): corpus lane {} absent at {} — differential not run",
```
```
    if ran == 0 {
        eprintln!(
            "SKIP (loud): no corpus lanes found under {} — the whole \
             TS differential skipped; set LUMEN_CORPUS to point at the \
```
What is wrong: the differential test structurally PASSES when zero of its
five lanes ran — `ran == 0` only eprintlns; no assert, no #[ignore], no
failure. In the orchestrator baseline (corpus verified absent in this
sandbox) both tests are green skips indistinguishable from passing
differentials (:2413-2414). The card 5.I1a "real-corpus TS differential"
therefore has no executed evidence anywhere I can verify. Why it matters:
a green baseline line that proves nothing; the same fabricated-evidence
shape as corpus_wav.rs F-001. What correct looks like: assert!(ran > 0)
behind the corpus env, or #[ignore = "corpus"] with an explicit opt-in
lane. Confidence: HIGH — Verification: ran (`test -d` missing) + read;
differential itself UNVERIFIED (no corpus bytes).
F-002 [S4]
Where: crates/demux/tests/ts_corpus.rs:256-258
Code:
```
// Keep the Path import referenced for the override-doc path helper.
#[allow(dead_code)]
fn _type_check(_p: &Path) {}
```
What is wrong: vestigial dead function that exists only to justify an
import — lint-dodge debris; the comment's "override-doc path helper" names
nothing in this file. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CorpusTruth + TRUTHS (5 lanes) | 31-96 | REAL-UNPROVEN | S3 | pinned ffprobe values; not executable here |
| corpus_dir | 98-103 | REAL | — | LUMEN_CORPUS override else /tmp default (unconditional PathBuf — per-file existence checks downstream) |
| corpus_ts_programs_streams_and_names_match_ffprobe | 108-224 | REAL-UNPROVEN | S3 | strict asserts; green-when-empty structure (F-001) |
| corpus_ts_pes_seam_accounting | 228-245 | REAL-UNPROVEN | — | 0091 lane seam accounting; skipped here |
| drain | 247-254 | REAL | — | `.must()` on every event — honest (fails loudly on error) |
| _type_check | 256-258 | DEAD | S4 | F-002 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_ts_programs_streams_and_names_match_ffprobe | YES *where the corpus exists*; NO here | ran==0 passes vacuously (F-001) |
| corpus_ts_pes_seam_accounting | YES *where the corpus exists*; NO here | single-lane skip returns early |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every pinned value below was measured this session from the corpus-v4 instance" | :8-10 | REAL-UNPROVEN | not re-executable (corpus absent); values internally consistent |
| "a pre-Rust byte walk over the corpus confirmed every section's CRC-32/MPEG-2 hash to zero" | :15-17 | REAL-UNPROVEN | no artifact of that walk exists in the repo; the assert (:198-202) would re-check it if the corpus existed |
| "this suite **skips loudly**" | :19-20 | PARTIAL | loud on stderr only; silent as a green pass/fail line in the baseline |

## 7. Dead / duplicated / unreachable code
_type_check (F-002). Skip paths are the executed paths in this
environment.

## 8. Dependencies & flags
Integration test; dev-deps testkit/input/demux/plugin-api; `mod common`.
No feature gates. Same LUMEN_CORPUS convention as the other corpus suites.

## 9. Open questions for the next agent
- The ts demuxer itself (crates/demux/src/ts/) is another lane's file; the
  pinned surface here (programs(), TsStats, descriptor()) is its API —
  cross-check that those accessors exist and are honest.
- Same corpus-absence caveat will apply to every other `*_corpus.rs` suite
  in crates/demux/tests/ (adts_corpus, es_corpus, mkv_corpus, mp4_corpus,
  ogg_corpus, ps_corpus …) — recommend one workspace-wide finding.
