# AUDIT: crates/codecs/tests/aac_corpus_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-23 |
| Lines / functions | 215 / 4 helpers + 1 test (support module tests/aac/mod.rs verified: real ffmpeg subprocesses, real ADTS decode) |
| File verdict | REAL (live differential verified) — but VACUOUS-PASS by default (corpus-root guard returns before the committed-fixture rows) |
| Reachable from shipped entry point? | Test: NO. AacDecoder under test: YES — pipeline_host.rs:288 `"aac" \| "aac-lc" =>` in the shipped CLI. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | (1) `./target/debug/deps/aac_corpus_differential-7843d789557f43b7 --nocapture` (default, no corpus) → `SKIP aac corpus differential: corpus root absent` + `test result: ok. 1 passed` — zero rows compared. (2) With `LUMEN_CORPUS=/tmp/w06a03/corpus` (empty media dir) → the two committed-fixture rows executed the LIVE ffmpeg differential: `sine441_lc_stereo.aac rate=44100 ch=2 samples=134144 differing=84 (0.0313%) max_delta=1`, `sine48_lc_mono.aac ... samples=145408 differing=53 (0.0364%) max_delta=1` → 1 passed. |

## 1. One-paragraph verdict
A genuine external-oracle differential when it runs: Lumen's AAC-LC decode and ffmpeg's decode of the same ADTS bytes are compared sample-locked (equal byte length, per-sample |Δ| ≤ 1 LSB), with corpus rows additionally pinned to recorded per-file MD5s so byte drift fails loudly, a documented stream-copy bridge for m4a lanes, and an honest typed-refusal row for 6-channel (engine caps at stereo). I verified the live machinery on the two committed fixtures (max_delta=1, ~0.03% differing samples — real lossy-rounding agreement with ffmpeg). The defect is structural: the corpus-root guard returns BEFORE the committed-fixture rows, so in the default environment (no /tmp corpus — this machine, and any corpus-less CI lane) the single test passes having compared nothing, despite ffmpeg being present and the fixtures being in-tree. The header documents the SKIP, but the baseline pass count includes this vacuous pass.

## 2. What actually works (verified)
- Live oracle on committed fixtures (corpus dir present, corpus files absent): both rows pass with max_delta=1 within the ±1 LSB bar; MD5s of both sides printed for the worklog table.
- ffmpeg subprocess oracle is real (tests/aac/mod.rs:113-189: `ffmpeg_decode_s16le`, `ffmpeg_extract_adts` with `-c:a copy`, `ffmpeg_decode_s16le_adts_bytes` with `-f aac` forced input — the documented probe quirk).
- Recorded per-file expectations (`expect()`) fail loudly on corpus-byte drift (asserted at L136-149).

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/aac_corpus_differential.rs:121-124 (vs L183-185)
Code:
```rust
    let Some(corpus) = corpus_dir() else {
        eprintln!("SKIP aac corpus differential: corpus root absent");
        return;
    };
```
What is wrong: the guard exits the whole test before the committed-fixture rows (L183-185: `sine441_lc_stereo.aac`, `sine48_lc_mono.aac`) which need neither the corpus nor anything absent — ffmpeg IS on PATH (checked first) and the fixtures ARE in-tree. Default-state result: `1 passed` with zero comparisons (verified). This both inflates the baseline pass count and throws away runnable live-oracle coverage on every corpus-less machine. What correct looks like: move the fixture rows before the corpus guard (or give them their own test fn). Confidence: HIGH — Verification: ran (both modes).
F-002 [S4]
Where: crates/codecs/tests/aac_corpus_differential.rs:84-113
Code:
```rust
fn expect(row: &Row, key: &str) -> String {
    match row.file.as_str() {
        "0059-audio-only.aac" => [
```
What is wrong: only the corpus rows carry pinned MD5 constants; the fixture rows are held to the ±1 LSB bar and the live oracle only — no recorded pin, so a fixture swap changes nothing detectable in the ffmpeg-present lane. (Same class as av1 F-002.) Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Row | 41-50 | REAL | — | verdict record |
| run_row / run_row_bytes | 52-80 | REAL | — | byte-length lock + per-sample delta stats; executed live |
| expect | 84-113 | REAL | S4 | F-002; corpus rows pinned, fixture rows not |
| corpus_differential_vs_ffmpeg (test) | 115-214 | REAL | S3 | F-001 guard order; live rows verified; typed-refusal row for 0094 read-verified (corpus file absent here) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_differential_vs_ffmpeg (corpus present) | YES | byte-length lock + ±1 LSB bar + recorded MD5 pins (corpus rows) |
| same (default, corpus absent) | NO | early return; passes vacuously (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decoded BOTH by Lumen and by the external ffmpeg (the oracle, never a dependency), compared on the SAME frame window" | header L1-4 | REAL (when it runs) | live rows: max_delta=1, equal lengths |
| "SKIP-with-message when the corpus root or ffmpeg is absent" | header L27 | REAL | default run prints the SKIP — but reports `ok` (F-001) |
| "0094 ... the row asserts the TYPED refusal" | header L23-25 | REAL-UNPROVEN | corpus file absent in this sandbox; code path read (asserts `aac.channel_count`) but not executed here |
| m4a bridge: stream copy, no re-encode; probe quirk recorded | header L13-18 | REAL-UNPROVEN | helper read (ffmpeg -c:a copy, -f aac); 0029 row not executable here (file absent) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Workspace crates + external ffmpeg (probed; three distinct subprocess shapes in the helper module). Env knob LUMEN_CORPUS. No feature gates.

## 9. Open questions for the next agent
- tests/aac/mod.rs (support) and crates/codecs/src/audio/aac/decoder.rs are not in my assignment — the ±1 LSB tolerance's justification (AAC rounding modes) lives there.
- If a corpus-imaged CI lane exists, the recorded corpus MD5s (c18f85b0..., c1bb4de6..., 6becaacf...) should be re-verified there; UNVERIFIED here (files absent).
