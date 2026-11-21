# AUDIT: crates/codecs/src/video/h264/parser/sei.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2026-11-21 |
| Lines / functions | 597 / 15 (incl. 6 tests, 1 test helper) |
| File verdict | UNREACHABLE (real, correct parser; zero production callers) |
| Reachable from shipped entry point? | NO — `parse_sei` called only by `crates/codecs/src/video/h264/fuzz.rs:59` (fuzz harness) and the file's own `#[cfg(test)]` tests. rg `parse_sei\|SeiMessage\|ParsedSei\|X264_UUID` across repo → only sei.rs, h264/fuzz.rs, and an unrelated `subtitles/src/cc/cc_data.rs` (its own GA94 parser). The reachable H.264 lanes (`sw.rs` SW decoder, `decode.rs` HW decoder) never touch SEI. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | rg `parse_sei\|SeiMessage` (callers: none in production); sed -n quotes (221-234, 313-329, 336-356); ls /tmp/my-project/corpus-v4/media → does not exist; find repo for `0019-raw-video.h264` → absent; baseline test log lines 608-613. |

## 1. One-paragraph verdict
A clean, genuinely defensive SEI message parser: bounded loops (MAX_MESSAGES=64 doubles as the complexity bound — ≤65 `more_rbsp_data` scans keep parse_sei linear), checked arithmetic on payload bounds, typed errors on every failure path, no unwrap/expect/index panics in production code. But it is dead weight in every shipped build: no production code path calls it (only the fuzz harness does), and three of its six tests silently self-skip because the corpus they read lives at a hard-coded Lead-sandbox path that does not exist in a fresh checkout — the baseline run counts those vacuous passes as "ok".

## 2. What actually works (verified)
— Hand-traced `parse_sei` bounds logic for hostile inputs: `[0x05,0x40,0xAB]` → TruncatedPayload; `[0x05,0xFF,0xFF]` (size run to EOF) → TruncatedPayload; `[0x05]` → UserDataTooShort; `[]`/`[0x80]` → Ok, 0 messages. `pos` never exceeds `rbsp.len()`, so `rbsp[pos..]` cannot panic. Verified by reading lines 221-296; the error-path test at 525-551 asserts the same shapes and passed for real in baseline (`test-workspace.log.txt:609 error_paths_are_typed ... ok` — synthetic inputs, no corpus needed).
— `synthesized_payloads_parse_and_round_trip_their_fields` (baseline :611 ok) and `trailing_bit_forms_are_tolerated_and_counted` (:612 ok) run on in-test literals — real, non-vacuous assertions.

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/video/h264/parser/sei.rs:221 (+ crates/codecs/src/video/h264/fuzz.rs:59)
Code:
```rust
pub fn parse_sei(rbsp: &[u8]) -> Result<ParsedSei, SeiError> {
    let mut messages = Vec::new();
    let mut pos: usize = 0;
    while more_rbsp_data(&rbsp[pos..]) {
```
What is wrong: `parse_sei` (and with it the whole 297-line production half of this file: `SeiMessage`, `UserDataUnregistered`, `RecoveryPoint`, `FillerPayload`, `X264_UUID`, `FFMPEG_LAVC_PREFIX`) has no production caller. The only non-test caller is the in-repo fuzz harness (`fuzz.rs:59 let _ = super::parser::sei::parse_sei(&rbsp_bytes);`). Neither the shipped SW decoder (`sw.rs`) nor the HW decoder (`decode.rs`) nor any demux/probe layer consumes SEI.
Why it matters: ~300 lines of compiled dead code in the shipped `lumen` binary; the module docs' rich "card 5.F1b" scope suggests wired functionality that no runtime path exercises. `recovery_point` in particular is the classic SEI payload a real decoder uses for seek/latency hints — none of that happens.
What correct looks like: either a consumer (e.g. sw.rs skipping to a recovery point after damage) or the module moved behind a fuzz/test-only cfg until one exists.
Confidence: HIGH — Verification: read + rg callers (listed in header).

### F-002 [S2]
Where: crates/codecs/src/video/h264/parser/sei.rs:337-355 (+ parser/slice.rs:1399-1404)
Code:
```rust
        let data = std::fs::read(path).unwrap_or_default();
        annexb_nals(&data)
```
```rust
        let seis = corpus_seis("0019-raw-video.h264");
        if seis.is_empty() {
            eprintln!("corpus: 0019-raw-video.h264 not present — skipping");
            return;
        }
```
What is wrong: three of six tests (`corpus_0019_…`, `corpus_0014_…`, and the 200-round `mutated_sei_streams_are_typed_or_ok_never_panics` property) depend on `corpus_path()`, whose default root is the hard-coded Lead-sandbox literal `/tmp/my-project/corpus-v4/media` (slice.rs:1401). That directory does not exist in this sandbox (`ls` → No such file or directory) and `0019-raw-video.h264` is nowhere in the repo (find → only the A–F differential fixtures under crates/codecs/tests/fixtures/h264/). The tests therefore return early and report "ok": baseline `test-workspace.log.txt:608,610,613` records all three as passing, proving nothing about the corpus claims.
Why it matters: swallowed test failure — the suite's real coverage (the x264 build-string parse, the mkv lane, the mutation property) silently evaporates in every environment that is not the original author's machine; "1681 passed" overstates the evidence. Same defect class as full_gate.sh's hard-coded `/home/z/my-project` root already on the ledger (baseline INDEX.md, tooling gaps).
What correct looks like: corpus fixtures committed (or generated by `cargo xtask fixtures`) under the repo; on absence, `panic!`/fail, or mark `#[ignore]` — never a silent early return; env default removed or repo-relative.
Confidence: HIGH — Verification: read + ls/find in sandbox + baseline log lines 608-613.

### F-003 [S3]
Where: crates/codecs/src/video/h264/parser/sei.rs:313-329
Code:
```rust
/// `payloadType` / `payloadSize`: accumulate `0xFF` continuation
/// bytes then the final `< 0xFF` byte (§7.3.2.3). `pos` advances
/// past the read; on an exhausted remainder this returns the last
/// accumulation (the caller's payload-bounds check turns it into a
/// typed truncation).
```
What is wrong: the doc claim is wrong for the exhausted case. If the `payloadType` header consumes the entire remainder (e.g. RBSP `[0x03]`), `read_accumulated` for `size` returns 0 without reading, the bounds check passes (end == pos == len), and the parser emits a phantom size-0 message: `[0x03]` → `Ok([Unparsed{3,0}])`, `[0x0C]` → `Ok([Filler{size:0, all_ff:true}])` (vacuous truth on zero bytes). Not a typed truncation. Benign for `[0x05]`/`[0x06]`/`[0x00]` (empty-payload bit reads fail typed), and `[0x01]` yields `PicTiming{payload_len:0}`.
Why it matters: contradicts the module's own damage model ("truncated payloads" → typed `SeiError`); a truncated SEI header is accepted as a successful parse for 4 of 6 payload types. No crash, no hang; blast radius is a wrong Ok on malformed input in a parser that nothing currently consumes (F-001).
What correct looks like: `read_accumulated` returns `Option<u64>` (None when it read zero bytes / hit exhaustion without a terminating byte), mapped to `TruncatedPayload`; doc updated.
Confidence: HIGH — Verification: read + hand-trace of the four cases above.

### F-004 [S4]
Where: crates/codecs/src/video/h264/parser/sei.rs:53-61
Code:
```rust
pub const X264_UUID: [u8; 16] = [
    0xdc, 0x45, 0xe9, 0xbd, 0xe6, 0xd9, 0x48, 0xb7, 0x96, 0x2c, 0xd8, 0x20, 0xd9, 0x23, 0xee, 0xef,
];

pub const FFMPEG_LAVC_PREFIX: [u8; 4] = *b"Lavc";
```
What is wrong: two `pub` consts exported from the crate's public surface with no consumer outside this file's tests (rg: only sei.rs). Public-API surface that exists only for test assertions.
Why it matters: API debt; anyone relying on `codecs::video::h264::parser::sei::X264_UUID` is relying on test scaffolding.
What correct looks like: `#[cfg(test)]` (or used by a real consumer per F-001).
Confidence: HIGH — Verification: rg for both names.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SeiError` (enum) + `Display` + `Error` + `From<BitReadError>` | 68-120 | REAL | — | trivial, correct; all variants carry context |
| `SeiMessage` (enum) | 123-154 | REAL | — | data-only; matches documented subset |
| `UserDataUnregistered` + `is_x264`/`is_ffmpeg_lavc`/`as_str_lossy` | 158-182 | REAL | — | lossy decode keeps mutated input panic-free; allocates per call (fine at this call volume) |
| `RecoveryPoint` (struct) | 185-195 | REAL | — | data-only |
| `FillerPayload` (struct) | 198-204 | REAL | — | `all_ff` vacuous-true on empty payload (F-003) |
| `ParsedSei` (struct, Default) | 207-214 | REAL | — | derive only |
| `parse_sei` | 221-296 | REAL | S3 | no production caller (F-001); bounds/overflow handling verified (checked_add, try_from, end>len guard); linear time (64-msg budget caps scan count) |
| `more_rbsp_data` | 302-311 | REAL | — | early-exit popcount; correct ≥2-set-bits rule |
| `read_accumulated` | 318-329 | REAL | S3 | exhausted-remainder → phantom size-0 message (F-003); u64 addition overflow needs >2^56 bytes — unreachable |
| tests: `corpus_seis` helper | 337-347 | REAL (test-only) | S2 | unwrap_or_default swallows fs errors (F-002) |
| test `corpus_0019_…` | 350-388 | VACUOUS in baseline | S2 | skipped (corpus absent); assertions real when corpus present |
| test `corpus_0014_…` | 391-411 | VACUOUS in baseline | S2 | same |
| test `synthesized_payloads_…` | 414-501 | REAL | — | in-test literals; would fail on any parse regression (mutation reasoning: change `read_ue` marker-bit handling → rp/bp asserts fail) |
| test `trailing_bit_forms_…` | 504-522 | REAL | — | covers empty/bare/padded tails |
| test `error_paths_are_typed` | 525-552 | REAL | — | asserts exact error variants |
| test `mutated_sei_streams_…` | 557-596 | VACUOUS in baseline | S2 | skips without corpus; `assert!(errs > 20)` bite-check is good when it runs |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| corpus_0019_carries_the_x264_build_string_and_a_recovery_point | NOT IN THIS ENVIRONMENT | early-return when corpus missing (F-002); with corpus present: yes (exact string + field asserts) |
| corpus_0014_mkv_lane_carries_the_same_x264_uuid | NOT IN THIS ENVIRONMENT | same early-return |
| synthesized_payloads_parse_and_round_trip_their_fields | YES | byte-exact expected messages; any decoder drift fails |
| trailing_bit_forms_are_tolerated_and_counted | YES | zero_tail_bytes counts asserted |
| error_paths_are_typed | YES | exact variant equality (`TruncatedPayload{…}`, `UserDataTooShort`) |
| mutated_sei_streams_are_typed_or_ok_never_panics | NOT IN THIS ENVIRONMENT | corpus-gated; asserts oks+errs==200 and errs>20 when present |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "verified against the corpus's SEI NALs (both 0019-raw and the 0014 mkv lane…)" | sei.rs:50-52 | REAL-UNPROVEN | corpus not in repo; baseline passes are vacuous (F-002) |
| "No panics, no hangs" | sei.rs:43 | REAL | hand-trace: no unwrap/expect/index-out-of-bounds path; loop consumes ≥1 byte/iteration + 64-msg budget |
| "each SEI NAL RBSP is a sequence of (payloadType, payloadSize, payload)" | sei.rs:4-6 | REAL | code matches §7.3.2.3 order |
| buffering_period "minimal: seq_parameter_set_id only" | sei.rs:9-13, 125-131 | REAL | honest scope note; only ue(v) read (line 253) |

## 7. Dead / duplicated / unreachable code
Whole production half of the file is unreachable from shipped entry points (F-001). No duplication found (`more_rbsp_data` here vs `h264/parser/nal.rs:219 next_start_code` are different functions; mpeg12/startcode.rs is a separate MPEG domain).

## 8. Dependencies & flags
Depends only on `super::slice::{BitReadError, RbspBitReader}` (in-crate). No feature gates; compiled unconditionally in every build including wasm (`apps/web` pulls codecs with `default-features = false` and still compiles this module). No unsafe.

## 9. Open questions for the next agent
- W06-A07 (slice.rs parts): does `RbspBitReader::read_bits` mask correctly for `read_bits(2) as u8` (line 278)? Assumed yes; verify.
- If SEI ever gets a production consumer, F-003's phantom-message behavior becomes a real decode-correctness input.
