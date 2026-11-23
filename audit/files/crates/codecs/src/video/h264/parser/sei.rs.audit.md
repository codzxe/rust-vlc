# AUDIT: crates/codecs/src/video/h264/parser/sei.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-02-14 |
| Lines / functions | 597 (330 source + 267 tests) / 8 non-test fns + 6 tests |
| File verdict | REAL (correct parse-level SEI subset; crafted-vector tests genuine) — but production-unreachable (F-001) and half its baseline "ok" evidence is vacuous in this sandbox (F-002) |
| Reachable from shipped entry point? | NO — `parse_sei` callers: its own tests + fuzz.rs:59 (result discarded, absence-of-panic only). No decode-path (sw/hw) consumer |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | Read of full file; `Grep parse_sei|sei::` whole-repo caller map; `ls /tmp/my-project/corpus-v4/media` (absent); manual bit-decode of 5 test vectors; baseline citation audit/runs/baseline/cargo-test-workspace.txt:705-710 |

## 1. One-paragraph verdict
Genuine clean-room SEI message-layer parser: the §7.3.2.3 accumulated type/size headers, five payload types, typed truncation/UUID/golomb errors, a 64-message budget, and a documented tolerance for `0x80`-only and all-zero tails. I hand-decoded every crafted test vector (buffering_period `00 01 80 80`, recovery_point `06 01 50 80` and corpus comment `06 06 01 c4 80`, filler, Lavc user data, the 0xFF-continuation 256-byte size) and the asserted field values are all bit-correct. The problems are around the code, not in it: nothing in any shipped path consumes SEI at all (the parser is called only by tests and a fuzz harness that discards results), and three of its six tests silently pass with zero assertions when the external corpus directory `/tmp/my-project/corpus-v4/media` is missing — which it is in this sandbox, so the baseline "ok" lines for those three prove nothing verifiable.

## 2. What actually works (verified)
— Manual bit-decode of the 5 crafted vectors in `synthesized_payloads_parse_and_round_trip_their_fields` + `trailing_bit_forms_are_tolerated_and_counted` + `error_paths_are_typed`: all asserted values (ue decodings, flag bits, 2-bit idc, zero-tail counts, TruncatedPayload payload sizes) reproduce exactly.
— Baseline: `sei::tests::*` 6/6 ok (audit/runs/baseline/cargo-test-workspace.txt:705-710) — of which the 3 non-corpus tests (706, 708, 709) are real evidence; the 3 corpus-gated ones (705, 707, 710) are vacuous in a corpus-less environment (F-002).
— `read_accumulated` continuation arithmetic (0xFF chains) verified against the 256-byte multi-message vector: 0xFF+0x01 = 256 ✓.

## 3. Findings
F-001 [S3]
Where: whole file; evidence = caller map
Code: (verbatim, sei.rs:216-221)
```
/// Parse an SEI NAL RBSP (the NAL header byte already removed,
/// emulation prevention already stripped — card 5.F1a's layer).
///
/// # Errors
/// Typed [`SeiError`]s — never a panic, never a hang.
pub fn parse_sei(rbsp: &[u8]) -> Result<ParsedSei, SeiError> {
```
What is wrong: `parse_sei` has no production consumer. Whole-repo search: the only callers are sei.rs's own tests and fuzz.rs:59 (`let _ = super::parser::sei::parse_sei(&rbsp_bytes);` — result discarded, absence-of-panic contract only). Neither the software decode path (sw.rs) nor the hw path (decode.rs) nor any probe consumes SEI messages — recovery_point, the x264 build string, everything this file extracts lands nowhere. The module header presents the payload scope as card deliverables; as shipped behavior it is dead code. What correct looks like: at minimum wire recovery_point into the sw damage/recovery accounting the module docs gesture at, or mark the module pub-for-tests.
Confidence: HIGH — Verification: `Grep parse_sei` whole repo.
F-002 [S3]
Where: sei.rs:337-355, 391-395, 557-561 (test skip gates); baseline evidence lines 705/707/710
Code: (verbatim, sei.rs:350-355)
```
    fn corpus_0019_carries_the_x264_build_string_and_a_recovery_point() {
        let seis = corpus_seis("0019-raw-video.h264");
        if seis.is_empty() {
            eprintln!("corpus: 0019-raw-video.h264 not present — skipping");
            return;
        }
```
What is wrong: Three of six tests (corpus_0014…, corpus_0019…, mutated_sei_streams…) early-return as PASSING tests when the corpus dir is absent. `corpus_path` resolves to `/tmp/my-project/corpus-v4/media` (slice.rs:1399-1404) which does not exist in this sandbox (`ls` -> No such file or directory). cargo swallows eprintln of passing tests, so the baseline "ok" for these three (baseline:705,707,710) is indistinguishable from a real pass and, in a corpus-less environment, proves zero assertions ran (including the 200-round mutation property and its `errs > 20` bite check). Whether the baseline environment had the corpus is UNVERIFIABLE from the committed log. What correct looks like: `#[ignore]` + explicit corpus gate with a counted skip (test harness reports ignored), or commit the two tiny SEI NALs as fixtures.
Confidence: HIGH on the mechanism; UNVERIFIED whether the baseline run had the corpus (baseline log records no env).
F-003 [S4]
Where: crates/codecs/src/video/h264/fuzz.rs:57-60 (cross-file)
Code: (verbatim, fuzz.rs:58-60)
```
        if let Ok(rbsp_bytes) = super::parser::nal::unescape_rbsp(nal) {
            let _ = super::parser::sei::parse_sei(&rbsp_bytes);
        }
```
What is wrong: `unescape_rbsp(nal)` unescapes the FULL NAL including the one-byte header (nal.rs:363-367), but `parse_sei` documents its input as "the NAL header byte already removed" (sei.rs:216). The fuzz SEI walk therefore parses misaligned data — the NAL header byte is consumed as the first payloadType byte, so the real SEI framing (type/size/payload) is never fuzzed in alignment. The fuzz contract is absence-of-panic, so nothing breaks, but the module doc's claim that the harness runs "the SEI walk over the unescaped RBSP" overstates what is exercised.
Confidence: HIGH — Verification: read both signatures.
F-004 [S4]
Where: sei.rs:250-255
Code: (verbatim)
```
            0 => {
                let mut r = RbspBitReader::new(payload);
                SeiMessage::BufferingPeriod {
                    seq_parameter_set_id: r.read_ue()?,
                }
            }
```
What is wrong: `seq_parameter_set_id` is not range-checked to the spec's 0..=31 (the SPS parser checks this; here a mutated stream yields arbitrary u32 values into a public struct field). Parse-level recording only, no downstream consumer to corrupt — hygiene.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| X264_UUID / FFMPEG_LAVC_PREFIX consts | 53-61 | REAL | — | corpus claims unverifiable here (F-002) |
| SeiError Display / Error / From | 91-120 | REAL | — | complete typed taxonomy |
| UserDataUnregistered::{is_x264, is_ffmpeg_lavc, as_str_lossy} | 166-182 | REAL | — | lossy decode, no panic |
| parse_sei | 221-296 | REAL | S4 | accumulated headers ✓, checked bounds ✓, budget ✓; F-004 |
| more_rbsp_data | 302-311 | REAL | — | "≥2 set bits" = spec predicate modulo the documented zero-tail tolerance |
| read_accumulated | 318-329 | REAL | — | 0xFF chains verified; exhaustion → typed truncation via caller |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| synthesized_payloads_parse_and_round_trip_their_fields | YES | 6 crafted vectors with exact expected messages (I re-decoded all by hand) |
| trailing_bit_forms_are_tolerated_and_counted | YES | zero-tail counts asserted exactly |
| error_paths_are_typed | YES | exact error payloads incl. GolombOverflow match |
| corpus_0014… / corpus_0019… / mutated_sei… | NOT VERIFIABLE | corpus-gated early-return pass; corpus absent in this sandbox (F-002); mutation test's `errs > 20` bite check never runs without corpus |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "verified against the corpus's SEI NALs (both 0019-raw and the 0014 mkv lane carry the x264 build string)" | sei.rs:50-52 | REAL-UNPROVEN | historical claim; corpus not present to re-verify (UNVERIFIED) |
| "No panics, no hangs" (damage model) | sei.rs:43 | REAL for crafted paths (error_paths + 3 real baseline tests); the 200-round mutation property is corpus-gated and did not demonstrably run | F-002 |
| "every message consumes ≥ 2 bytes — the budget is the allocation bound" | sei.rs:41-43 | REAL | read: type+size minimum, MAX_MESSAGES=64 caps Vec growth |

## 7. Dead / duplicated / unreachable code
The entire module is production-unreachable (F-001): exercised by unit tests (3 real, 3 corpus-gated) and one discard-result fuzz call.

## 8. Dependencies & flags
`super::slice::{BitReadError, RbspBitReader}` (in-crate). No feature gates. The corpus gate reads env `LUMEN_H264_CORPUS` with default `/tmp/my-project/corpus-v4/media` (slice.rs:1400-1401, cfg(test) only).

## 9. Open questions for the next agent
1. slice.rs auditor: the same corpus-vacuity pattern (corpus_path early-return passes) applies to the slice-header corpus suites — same F-002 caveat.
2. fuzz.rs auditor: F-003 header-byte misalignment also matters for the SPS/PPS fuzz pass? (No — parse_with_buffer does its own header split; only the SEI call is misaligned.)
