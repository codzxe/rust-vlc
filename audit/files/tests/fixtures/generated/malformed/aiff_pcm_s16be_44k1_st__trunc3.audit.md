# AUDIT: tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__trunc3
| Field | Value |
|---|---|
| Auditor | W16-A10 |
| Date | 2026-11-24 |
| Lines / functions | binary; 17667 bytes; N/A (Class-B malformed fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by crates/demux/tests/wav_aiff_fixtures.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. One-paragraph verdict
Genuine mid-sound truncation of the valid AIFF parent (35334 -> 17667 bytes): the FORM size field still claims 35326 and the SSND chunk still declares 35288 bytes, so every declared extent runs past the actual stream — exactly the damage classes the consuming test pins (`form-extends-past-stream`, `ssnd-extends-past-stream`, `comm-frames-past-ssnd`). That test passes in the committed baseline.

## 2. What actually works (verified)
- `python3 audit/scratch/W16-A10/batch.py` -> size=17667, sha256=bd6e87ed2f9d5d69c0a58723c1b2401726140d52fb19e5e312ed2a9b33dba780; aiff parse: FORM/AIFF, `form_size 35326, match False`, chunks `[('COMM', 18), ('SSND', 35288)]` — declared extents exceed the 17667-byte file.
- PAIRS vs valid parent: sizes 17667 vs 35334, 0 common-prefix diff bytes (pure prefix truncation, no re-patching — correct for a truncation op).
- Consumer: crates/demux/tests/wav_aiff_fixtures.rs:538-546 pins trunc3 with `bytes: 17_612` (whole frames) and reasons `["form-extends-past-stream", "ssnd-extends-past-stream", "comm-frames-past-ssnd", "ssnd-not-frame-aligned", ...]`; the fuzz-harness totality test (line 702-716) feeds its bytes to `demux::aiff::fuzz::fuzz_demux_aiff`.
- Execution evidence: audit/runs/baseline/cargo-test-workspace.txt:2436-2452 — `aiff_mutated_family_outcomes_are_pinned_exactly ... ok` (8/8 in that suite).

## 3. Findings
F-001 [S3]
Where: tests/fixtures/generated/MANIFEST.toml (28 entries; aiff family absent)
What is wrong: not covered by the manifest drift gate (same gap as the wav family — only avi/mp4 mjpeg_pcm derivatives are pinned). Silent drift undetectable. Confidence: HIGH — Verification: ran (python manifest cross-check: NOT IN MANIFEST).

## 4. Per-function table
N/A (binary data file).

## 5. Test honesty
Honest: the pinned outcome (typed truncation after whole frames + damage reasons) would fail if the AIFF demuxer's truncation handling regressed. Baseline ok.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Mid-sound cut at 17667: 17612 whole frames deliver." | crates/demux/tests/wav_aiff_fixtures.rs:538 | REAL | file size 17667 verified; (17667-55 header)=17612 frame bytes arithmetic consistent |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
None.
