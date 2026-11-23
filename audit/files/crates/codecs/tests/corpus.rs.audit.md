# AUDIT: crates/codecs/tests/corpus.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 649 / 14 (2 tests) |
| File verdict | REAL-UNPROVEN (measurement lanes real + driver wiring verified; no corpus run executed here) |
| Reachable from shipped entry point? | N/A test asset — wired to `tools/corpus/run_harness.py` (verified: driver invokes `cargo test --release -p codecs --test corpus -- --ignored` with per-test filters and the exact env trio, run_harness.py:575-646) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `./target/debug/deps/corpus-a99dcfd79be7ba9f --ignored --nocapture` (no env) → 2 passed vacuously in 0.00s (exit 0); read of run_harness.py |

## 1. One-paragraph verdict
This is the real randomized-corpus measurement lane, not a fake: it opens corpus AVIs through the production `input`+`AviDemuxer` stack, decodes every MJPEG packet via the free `decode_image` entry, streams the ffmpeg rawvideo reference frame-by-frame (bounded memory), and computes PSNR-min/mean (millidB), max-abs, exact-byte fraction and throughput; PCM is compared byte-exact with mismatch/max-abs/length-note columns. The driver wiring is genuine (run_harness.py:640-644 calls these exact test names with the env trio). What keeps it honest is also what limits it: the tests are `#[ignore]`d and env-gated, and without the env trio they pass having asserted NOTHING (I executed this vacuous path). The measurement math I checked is sound (zip-min comparison, ceil-half chroma frame size, saturating PSNR of INF edge). Four hygiene-level defects, nothing structural.

## 2. What actually works (verified)
— verified by executing the no-env path (vacuous pass, output above) and by reading run_harness.py:575-646 (driver sets LUMEN_CORPUS/CASES/RESULTS, runs each test with a single-test filter to avoid TSV append interleaving). The actual corpus measurement NOT executed here (needs generated corpus + ffmpeg reference extractions + release test build — cargo-build-budget). UNVERIFIED: any committed results TSV proving a completed harness run.

## 3. Findings
F-001 [S4]
Where: corpus.rs:515-525 (and 585-595)
Code:
```
#[test]
#[ignore = "randomized-corpus harness: run via tools/corpus/run_harness.py"]
fn corpus_mjpeg_decode_measures_against_oracle() {
    let (Some(corpus), Some(cases_path), Some(results_path)) = (
```
What is wrong: run under `--ignored` without the env trio, the test prints "no-op" and PASSES having measured and asserted nothing (executed by me: "2 passed ... finished in 0.00s"). A green `cargo test -- --ignored` run therefore proves zero about decode fidelity. Mitigations are real (ignore reason documents the driver; eprintln is loud; the driver always sets the env), so blast radius is a human misreading a manual run. What correct looks like: `panic!("corpus harness requires LUMEN_CORPUS env trio")` or at minimum an explicit skipped marker. Confidence: HIGH — Verification: ran.
F-002 [S4]
Where: corpus.rs:479-494
Code:
```
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 5 {
            continue;
        }
```
What is wrong: malformed case-TSV lines are silently skipped — a truncated manifest yields fewer measured cases with no error; only a fully-empty list trips the assert. Same in parse_pcm_cases (L497-513). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: corpus.rs:397-412
Code:
```
        if audio_ids.contains(&packet.track) {
            payload.extend_from_slice(packet.data.as_slice());
        }
```
What is wrong: all audio tracks' payloads are concatenated into one buffer, but the case schema carries ONE descriptor/rate/channels per file — a multi-audio-track corpus file would interleave tracks into a bogus byte-exact comparison (recorded as mismatch, misattributed to the decoder). Harness-generated corpus presumably has one audio stream per file; nothing here enforces it. Confidence: MEDIUM — Verification: read.
F-004 [S4]
Where: corpus.rs:413-421
Code:
```
    let sample = match PcmFormat::from_descriptor(&case.descriptor) {
        Some(f) => f,
        None => return PcmOutcome::failed("pcm.unknown-descriptor".to_owned()),
    };
```
What is wrong (scope limitation, self-documented at L350-356): the PCM lane's decode config is harness-supplied, NOT derived from the file's strf header — so this suite does not validate the AVI strf→decoder wiring, only the decoder against a known config. The doc comment states this honestly ("Provenance, stated honestly (D-5)") and delegates the cross-check to the harness-vs-ffprobe comparison. Recorded as a coverage boundary, not a hidden lie. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| open_via_input | 58-62 | REAL | — | real access stack, typed URI refusal propagated |
| RefReader::new/next_frame | 73-97 | REAL | — | streaming, short-tail frame returned partial (honest), read-0 handling correct |
| diff_frame | 103-127 | REAL | — | zip-min (documented), i32 diff no overflow, exact-byte count |
| psnr_millidb | 129-135 | REAL | — | mse<=0 → 999_999 sentinel; INF edge saturates to i64::MIN via `as` (unreachable in practice: n==0 only for empty frames) |
| format_snake | 201-206 | REAL | — | 2-variant match, exhaustive |
| run_mjpeg_case | 208-313 | REAL | — | real demux+decode loop; shape-gated comparison; concealed frames counted separately; terminal demux damage breaks with counts intact (documented) |
| frame_bytes | 316-320 | REAL | — | ceil-half chroma, correct I420 size incl. odd dims |
| run_pcm_case | 377-473 | REAL | F-003/F-004 | byte-exact compare with length note; dropped-tail column |
| parse_mjpeg_cases / parse_pcm_cases | 479-513 | REAL | F-002 | silent skip of malformed lines |
| corpus_mjpeg_decode_measures_against_oracle | 515-583 | REAL-UNPROVEN | F-001 | env-gated; row schema 15 columns matches MjpegOutcome; `-` markers for gaps |
| corpus_pcm_decode_is_byte_exact_or_recorded | 585-649 | REAL-UNPROVEN | F-001 | same pattern, 11 columns |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_mjpeg_decode_measures_against_oracle | ONLY with env trio: fails on panic/missing files; decode quality is *reported*, not asserted (by design — "verdict quality is the aggregator's judgement", L31-32). Without env: never fails (F-001). | executed no-op path |
| corpus_pcm_decode_is_byte_exact_or_recorded | same | same |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "real ffmpeg-authored media, decoded and measured, with ffmpeg's own decode as the external oracle" | corpus.rs:1-3 | REAL-UNPROVEN | pipeline code is real (input+demux+decode_image all production paths); an actual oracle run not executed by me |
| "no synthetic fixture anywhere in the path" | corpus.rs:8-9 | REAL | read: corpus files + raw refs from disk; nothing inline |
| "The suite fails only on contract violations (panic, missing files)" | corpus.rs:31-32 | REAL | only asserts are non-empty cases + file existence; metrics are write-only |
| PCM "any mismatch is a real defect signal" | corpus.rs:17-19 | REAL | byte-exact zip compare, mismatch column |

## 7. Dead / duplicated / unreachable code
`MjpegCase.frames` is `#[allow(dead_code)]` — carried for row-schema fidelity, documented (L146-150). No other dead code.

## 8. Dependencies & flags
Cross-crate test deps: codecs (mjpeg free entry + pcm decode), demux AviDemuxer, input FileAccess, plugin-api, testkit Must, media-types. `#[ignore]` by design. No unsafe.

## 9. Open questions for the next agent
- Is there a committed mjpeg-results.tsv / pcm-results.tsv from a completed harness run anywhere (harness output dir)? I found none in-tree (tools/corpus has only .py) — the "randomized corpus directive" evidence trail is UNVERIFIED. Owner: tools/corpus or whoever cites the corpus in docs/matrix rows.
