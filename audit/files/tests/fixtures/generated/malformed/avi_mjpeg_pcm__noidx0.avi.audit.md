# AUDIT: tests/fixtures/generated/malformed/avi_mjpeg_pcm__noidx0.avi
| Field | Value |
|---|---|
| Auditor | W16-A10 |
| Date | 2026-11-24 |
| Lines / functions | binary; 423588 bytes; N/A (data fixture) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test-only input; consumed by demux/playback-engine/first-vertical/soak/media-probe/codecs suites |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |

## 1. One-paragraph verdict
Genuine Class-B malformed derivative of `avi_mjpeg_pcm.avi` produced by the `drop-idx1` op: my byte-diff against the valid parent shows exactly 2 changed bytes at offsets 4-5 (the RIFF size field, 425516->423588) and a length delta of -1928 (= 8-byte idx1 header + 1920-byte index). It is pinned in MANIFEST.toml (sha256+size match on disk) and has a matching expectation TOML that pins the contract ("typed-error-or-bounded-recovery", never panic/hang). The demuxer-side test that consumes it passes in the committed baseline.

## 2. What actually works (verified)
- `python3 audit/scratch/W16-A10/batch.py` -> size=423588, sha256=5752528d3d03d478d097e81c8201731c56c7ced191b1a9324fcb45523cb4f295; RIFF `AVI ` with 2 LIST chunks (hdr + movi); PAIRS: vs valid parent "diff bytes=2; offsets[:10]=[4, 5]" and "delta -1928".
- MANIFEST pin check (python) -> manifest sha256/size MATCH on-disk file.
- Expectation `tests/fixtures/expectations/malformed/avi_mjpeg_pcm__noidx0.avi.toml` -> `sha256 = "5752528d..."` matches; `[derivation] op = "drop-idx1"` with `note = "idx1 dropped, RIFF size patched; avih still claims AVIF_HASINDEX (doc 03 s7)"` — consistent with my byte-diff.
- Execution evidence: `audit/runs/baseline/cargo-test-workspace.txt:2254` `test noidx0_demuxes_fully_via_the_movi_walk ... ok` (demux fixtures_class_b) and `:2755` `test class_b_noidx ... ok` (first-vertical).

## 3. Findings
F-001 [S3]
Where: tests/fixtures/expectations/malformed/avi_mjpeg_pcm__noidx0.avi.toml:24-27
Code: (verbatim)
```
[error]
# v0: the union contract — the specific flavor (typed-error |
# bounded-recovery | rejected) is pinned by demuxer conformance
```
What is wrong: the committed expectation pins a UNION ("typed-error-or-bounded-recovery") instead of the actual behavior; the actual pinned outcome lives in per-crate conformance suites, so this file alone cannot fail a test by itself. Why it matters: "or" contracts let regressions flip between two behaviors silently at the expectation layer. What correct looks like: one specific outcome per fixture. Confidence: MEDIUM — the union is resolved elsewhere (demux tests) per the comment; the file itself proves nothing when run standalone.
Confidence: MEDIUM — Verification: read.

## 4. Per-function table
N/A (binary data file).

## 5. Test honesty
Consumers are honest for this fixture: `noidx0_demuxes_fully_via_the_movi_walk` would fail if the AVI demuxer could not walk a index-less movi; baseline shows it ok.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "idx1 dropped, RIFF size patched" | expectation TOML [derivation] | REAL | byte-diff: only offsets 4,5 changed; size delta -1928 |
| avih still claims AVIF_HASINDEX | expectation TOML note | UNVERIFIED (would need header parse at avih dwFlags; not parsed here) | — |

## 7. Dead / duplicated / unreachable code
None for this file.

## 8. Dependencies & flags
None (data). Consumers: crates/demux/tests/{fixtures_class_b,adversarial_gaps}.rs, crates/playback-engine/tests/scenarios_b/scenario_09_corrupt.rs, tests/integration/first-vertical/tests/recovery_matrix.rs + src/scenarios.rs, crates/media-probe/tests/fixtures_matrix.rs, crates/codecs/tests/fuzz_survival.rs, tests/soak/scenarios*.toml, transcripts/corrupt-noidx.json, crates/demux/src/avi/{index,mod}.rs (code paths it exercises).

## 9. Open questions for the next agent
corrupt-noidx.json transcript and the first-vertical scenario that uses this fixture — the baseline's first-vertical evidence suite FAILED (2 tests); check whether corrupt-noidx drift is among the causes.
