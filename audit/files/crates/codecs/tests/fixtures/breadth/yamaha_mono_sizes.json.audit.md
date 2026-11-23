# AUDIT: crates/codecs/tests/fixtures/breadth/yamaha_mono_sizes.json (data fixture)
| Field | Value |
|---|---|
| Auditor | W06-R3 (credit W06-A08) |
| Date | 2026-11-23 |
| Size / type | 108 bytes; valid JSON flat array (python3 json: list, 18 entries, all 1024, sum 18 432) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture); consumed by crates/codecs/tests/adpcm_breadth_differential.rs (Case `adpcm_yamaha` mono, :56-61; parsed by `parse_sizes`, :131-133) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 json validation (18/18×1024, sum 18432); `ls` payload (18 432 bytes — exact match); `md5sum` -> 1f931748e2da6cdc3dd41ce38110851e; consuming suite executed `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b --test-threads=1` -> 4 passed, exit 0 |

## 1. One-paragraph verdict
The ffprobe `packet=size` capture for the `adpcm_yamaha` mono-22050 breadth lane: a flat JSON array of 18 packet sizes, all 1024, pairing with `yamaha_mono_payload.bin` (18 432 bytes = exactly the array sum). The consuming test parses it as plain UTF-8 JSON (no serde — by documented design, :131 "the ffprobe `packet=size` capture needs no real JSON machinery") and drove its lane green in my executed run (4/4, exit 0): geometry assert `("yamaha_mono_payload.bin", 18, 1024)` plus the bit-exact MD5 lane `aeddc262b3dd010955f9c73c6cfc86ad` both passed with this file in the loop.

## 2. What actually works (verified)
- JSON validity + shape: python3 `json.load` -> list, 18 entries, all 1024, sum 18 432.
- Payload pairing: `yamaha_mono_payload.bin` is 18 432 bytes — the sizes array exactly describes it.
- Execution: consuming suite run by me from the prebuilt binary -> 4 passed, 0 failed (the mono-yamaha Case is one lane of `breadth_corpus_is_bit_exact_against_ffmpeg_reference`).

## 3. Findings
None. Checked: JSON validity/shape, count-vs-payload-size consistency (18 × 1024 = 18 432 exact), consumer wiring (Case + parse_sizes), executed test run green, citations (adpcm_breadth_differential.rs:17 module-doc table row; docs/worklog/archive-2026-10.md breadth table).

## 4-9. (data fixture — per-function/test sections not applicable)
Oracle note (same as qt_payload.bin audit): the pinned MD5 is a past ffmpeg-7.1.5-session record; this suite is pin-only (no live ffmpeg spawn) — carried in the consuming test's existing audit (adpcm_breadth_differential.rs.audit.md).
