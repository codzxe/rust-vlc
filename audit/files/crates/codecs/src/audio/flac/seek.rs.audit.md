# AUDIT: crates/codecs/src/audio/flac/seek.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-21 |
| Lines / functions | 647 / 16 (+6 tests + 1 test helper) |
| File verdict | REAL-UNPROVEN — seek-resolution core (resolve/bisect/table_candidate/frame_at_or_after/planner) UNREACHABLE from shipped entry points; only `parse_header_at` is production-reachable |
| Reachable from shipped entry point? | PARTIAL: YES for `parse_header_at` (chain: `lumen open` → apps/cli/src/pipeline_host.rs:253-259 `HostDecoderProvider::decoder_for("flac")` → `FlacDecoder::decode` → flac/decoder.rs:249 `super::seek::SeekPlanner::parse_header_at`); NO for `SeekPlanner::resolve`/`planner` (callers: only crates/codecs/tests/flac_seek.rs + flac/fuzz.rs; grep `SeekPlanner|seek::planner` over the whole repo shows zero production callers) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 1 |
| Commands run | `rg -n "SeekPlanner|seek::planner|SeekPlan|resolve_inner"` (callers: tests/fuzz/docs only) · `rg -n '"flac"' crates/demux/src` (wav/ogg/mkv/mp4 emit `flac` tracks → decoder reachable) · `sed -n '60,80p' flac/mod.rs`, `sed -n '230,275p' flac/decoder.rs` (production use of parse_header_at for pts) · `rg -n '#ust_use'` (false alarm — bash pipeline eats `[m`; Read tool shows `#[must_use]`) · python3 CRC-8 reimplementation (all 4 test vectors match: 0x93/0x8c/0x24/0xc5; 0xFB & 0xFC == 0xF8 → mask accepts reserved-bit-set headers) · baseline test-workspace.log.txt (1681 passed incl. flac suites) |

## 1. One-paragraph verdict
The FLAC seek resolver is genuine, carefully engineered code — a verified-SEEKTABLE resolver plus a livelock-fixed bisection with per-probe bounded sync scans and CRC-8-verified header parses — and its unit/integration suites are real (containment + decode-suffix properties over ffmpeg-authored fixtures; all CRC vectors independently reproduced by this auditor in python). But its core (`SeekPlanner::resolve` and everything under it) is dead code in the shipped build: the module's own docs name "the future native-FLAC demuxer card" as the consumer, the matrix row `container-flac-native` is honestly `not_started`, and no demuxer calls it. What survives in production is only `parse_header_at` (pts arithmetic in the decode path). The parser itself has three real defects: a 64 KiB scan cap that overrides a declared `max_frame_size` (seek fails outright on legal hi-res multichannel files), the byte-1 reserved bit is never checked (doc and a test comment falsely claim it is), and the spec's 7-byte/36-bit sample-number coding is misparsed as a 6-byte form.

## 2. What actually works (verified)
- CRC-8 implementation and all four in-file test vectors — verified by python3 reimplementation (poly 0x07, init 0): 0x93, 0x8c, 0x24, 0xc5 all match.
- Unit + integration seek suites execute in the baseline run — `audit/runs/baseline/INDEX.md` #3: `cargo test --workspace` 1681 passed / 1 failed (failure is `apps/desktop/tests/zero_alloc_transport.rs`, unrelated).
- `parse_header_at` production reachability — verified by call chain: pipeline_host.rs:253-259 (`"flac" => self.seeded(track, config, FlacDecoder::new)`) → decoder.rs:249.
- Bisect termination — reasoned through all three branches (`lo = offset+1`, `hi = offset`, `hi = mid`): each strictly shrinks the bracket; `end = (start+limit).min(bound)` guarantees `offset < hi` in the `hi = offset` arm. No livelock path found.
- UTF-8 payload masks `0x7f >> additional` — verified equal to the spec payload masks for every length 2..6 (the extra prefix bit is always 0 in each canonical leading byte: 110/1110/11110/111110/1111110).

## 3. Findings

### F-001 [S2]
Where: crates/codecs/src/audio/flac/seek.rs:296-311 (esp. 299-301), contradicting module doc 59-61
Code:
```rust
        let limit = u64::from(self.facts.max_frame_size.unwrap_or(u32::MAX))
            .min(SCAN_WINDOW)
            .min(data_len - start);
```
What is wrong: `.min(SCAN_WINDOW)` is applied unconditionally, so a stream that *declares* `max_frame_size` (STREAMINFO, 36-bit field) larger than 64 KiB still gets a 64 KiB scan window. The module doc (lines 59-61) claims "a sync scan bounded by the stream's `max_frame_size` (or the [`SCAN_WINDOW`] cap when unknown)" — the code caps in both cases; the declared bound is dead.
Why it matters: legal FLAC frames exceed 64 KiB (block size up to 65 535 × up to 8 channels × up to 32 bit; a 96 kHz/24-bit/5.1 stream's frames are routinely >100 KiB). For such files `frame_at_or_after` returns `None` for probes landing mid-frame with the next header >64 KiB away; `bisect` then only ever executes the `None => hi = mid` arm, converges on the first frame, and `resolve` returns `flac.seek.no_frame` — seeking is broken file-wide (SEEKTABLE-covered targets survive; every inter-point target fails). The hi-res-multichannel case is FLAC's core audience.
What correct looks like: `let limit = u64::from(self.facts.max_frame_size.unwrap_or(SCAN_WINDOW)).min(SCAN_WINDOW_IF_UNSET…)` — i.e. use the declared bound when present, fall back to `SCAN_WINDOW` only when `max_frame_size` is `None`; and add a fixture with >64 KiB frames to tests/flac_seek.rs.
Confidence: HIGH — Verification: read (termination/None-arm trace above) + `sed -n '296,311p'`; no >64 KiB-frame fixture exists in the suite (tests use a 5 s 44.1 kHz/16-bit stereo carrier).

### F-002 [S3]
Where: crates/codecs/src/audio/flac/seek.rs:335 (check), 570-571 (false-attribution test), mod.rs:69-70 (doc claim)
Code:
```rust
        if b1 & 0b1111_1100 != 0b1111_1000 {
            return None;
        }
```
and the test that claims to cover the reserved bit:
```rust
        // Sync ok, reserved bit set (0xFB).
        assert!(SeekPlanner::parse_header_at(&[0xFF, 0xFB, 0x70, 0x00, 0xFF, 0xAA], 0).is_none());
```
What is wrong: spec byte 1 is `111110 | reserved(0) | blocking-strategy`. The mask `0b1111_1100` covers bits 7..2 (the sync remainder `111110` incl. bit 2 = 0) but never tests bit 1 (the reserved bit). `0xFB`/`0xFA` pass the sync mask (verified: `0xFB & 0xFC == 0xF8`). The 570-571 test *passes only by accident*: the array is truncated (6 bytes) and the 0xFF "continuation" byte also fails the UTF-8 check — the parse dies on `at(6)` returning `None`, not on the reserved bit. A structurally valid, CRC-correct header with the reserved bit set would be accepted.
Why it matters: the module doc (mod.rs:69-70 "sync, reserved rules, UTF-8 number, escapes and CRC-8 all verified") and parse_header_at's own doc (lines 321-322 "The full structural rule set is enforced (sync code, reserved bits and codes …)") are false for this bit; the test comment misattributes its own pass condition, so a future edit that "fixes" the truncation would keep the suite green while the spec violation remains. Practical decode impact is small (CRC-8 guards), but a spec-strict demuxer would never emit such headers.
What correct looks like: `if b1 & 0b1111_1110 != 0b1111_1000 { return None; }` (covering the reserved bit), plus a test with a CRC-fixed header carrying b1 = 0xFA asserting rejection.
Confidence: HIGH — Verification: read + python mask computation (`0xFB & 0b11111100 == 0xF8` → mask accepts).

### F-003 [S3]
Where: crates/codecs/src/audio/flac/seek.rs:366-372 (implementation) vs 375-379 (comment) and 28-29 (module doc)
Code:
```rust
        let mut additional = first.leading_ones().min(6) as u64;
        if additional == 1 {
            return None; // 0b10xxxxxx leading byte: invalid start
        }
        additional = additional.saturating_sub(1);
        let mut number: u64 = u64::from(first & (0x7f >> additional)) << (6 * additional);
```
What is wrong: the FLAC spec's number coding defines a **7-byte** form (leading byte `0xFE`, six continuations, 36 payload bits). `min(6)` caps the leading-byte run at 6, so `0xFE` is decoded as a 6-byte form: payload `0xFE & 0b11 = 2` (spec payload is 0), one continuation byte short → the escape bytes are read from the wrong offset → CRC-8 fails → the header is rejected. Net effect: sample numbers ≥ 2^31 (the only values needing the 7-byte form) cannot be parsed — in variable-blocksize streams every frame past ~6.2 h @ 96 kHz (13.4 h @ 44.1 kHz) becomes invisible to seeking, and the decode path (decoder.rs:249) silently falls back to claxon's `Block::time()`, which the module itself documents as misplacing frames.
Why it matters: the in-code comment (375-379) claims "the coding is defined for numbers up to 2^36; claxon applies the same rejection" — the code does not implement 36 bits, it rejects them; the module doc (28-29) advertises "UTF-8-coded frame/sample number" without qualification. Long-duration variable-blocksize FLAC is spec-legal and the seek surface silently fails on it.
What correct looks like: `min(7)` with a 7-byte branch (payload mask 0, six continuation bytes, 36-bit value), plus a 0xFE-headed test vector.
Confidence: HIGH — Verification: read + python simulation (`0xFE` leading_ones = 7 → min(6) → 6-byte path; payload computed 2 ≠ 0).

### F-004 [S3]
Where: crates/codecs/src/audio/flac/seek.rs:166-190, 196-222, 245-287, 469-485 — vs docs/feature-matrix.toml:709 and docs/formats/audio-flac.md:27-28
Code:
```rust
    pub fn resolve(&self, target_sample: u64) -> Result<SeekPlan, EngineError> {
        self.resolve_inner(target_sample)
            .map_err(FlacError::into_engine_error)
    }
```
What is wrong: the entire resolution surface (`resolve`, `resolve_inner`, `table_candidate`, `bisect`, `frame_at_or_after`, `header_at`, `planner`, `SeekPlan`) has no production caller. `rg -n "SeekPlanner|seek::planner|SeekPlan"` over the repo returns only: this module, flac/decoder.rs (parse_header_at only), flac/mod.rs re-exports, tests/flac_seek.rs, flac/fuzz.rs, and docs. No demuxer resolves FLAC seeks through it (demux/src has no native-FLAC demuxer; ogg/mkv/wav/mp4 seek by their own container indexes; matrix row `container-flac-native` = `not_started` on all platforms).
Why it matters: the matrix row `codec-audio-flac` (feature-matrix.toml:709) notes "SEEKTABLE seeking + binary-search fallback" as part of the "implemented" claim, and docs/formats/audio-flac.md:27-28 states "Seek: native SEEKTABLE when present; verified binary-search fallback over frames (flac_seek.rs)" — a reader concludes seeking works in the shipped player. It cannot: nothing reachable calls it. ~300 lines of dead-in-production code whose only exercisers are the test suite and the fuzz harness.
What correct looks like: either wire it (the native-FLAC demuxer card) or mark the seek surface and its doc/matrix claims as pending with an explicit "no consumer yet" note, the way `container-flac-native` honestly does.
Confidence: HIGH — Verification: read + rg caller search (listed above).

### F-005 [S4]
Where: crates/codecs/src/audio/flac/seek.rs:374-381
Code:
```rust
        // The first byte's payload width must not overflow the coding
        // (overlong forms are invalid per spec — e.g. 0xC0/0xC1).
        if additional >= 1 && first & (0x7f >> additional) == 0 && first & 0x80 != 0 {
```
What is wrong: the overlong guard only fires when the payload is exactly 0, which rejects leading byte `0xC0` but not `0xC1` (payload 1, value 64..127 — codable in one byte). The comment claims both are rejected. `0xC1`-headed numbers are accepted as non-canonical 2-byte forms (test coverage only pins 0xC0).
Why it matters: lenient acceptance of non-canonical coding can only cost false-sync rate (CRC still guards), but the comment/test pair documents behavior the code does not have.
What correct looks like: reject when the decoded value fits a shorter form (`number < (1u64 << (6 * additional + 6 - additional))`… or simply compare against the shorter form's max), or fix the comment to "0xC0 only".
Confidence: HIGH — Verification: read (`0xC1 & 0x3F == 1 ≠ 0` → guard does not fire).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SCAN_WINDOW` const | 69-74 | REAL | — | 64 KiB cap; doc/code conflict via F-001 |
| `ParsedFrameHeader` (struct) | 77-89 | REAL | — | plain data |
| `ParsedFrameHeader::first_sample` | 101-110 | REAL | — | saturating muls; fixed-stream rule matches ffmpeg/libFLAC; pinned by test at 601-620 |
| `ParsedFrameHeader::end_sample` | 115-118 | REAL | — | saturating add |
| `SeekPlan` (struct) | 123-129 | UNREACHABLE (prod) | S3 | F-004 |
| `SeekPlanner` (struct) | 131-138 | UNREACHABLE (prod, except parse_header_at) | S3 | F-004 |
| `SeekPlanner::new` | 143-156 | REAL | — | trivial constructor |
| `SeekPlanner::resolve` | 158-169 | REAL / UNREACHABLE | S3 | thin taxonomy wrapper; no prod caller |
| `SeekPlanner::resolve_inner` | 171-190 | REAL / UNREACHABLE | S3 | past-end gate only when `total_samples` set (None → no gate, acceptable) |
| `SeekPlanner::table_candidate` | 196-222 | REAL / UNREACHABLE | S3 | partition_point assumes ascending (metadata.rs doc claims spec order; unsorted hand-crafted tables would mis-pick, but the `first_sample == point.sample` verification degrades to fallback — fail-safe holds) |
| `SeekPlanner::bisect` | 245-287 | REAL / UNREACHABLE | S3 | termination verified (3 arms all shrink); `best`-tracking correct for the mid-frame-0 case; fails `flac.seek.no_frame` (typed) on non-contained best |
| `SeekPlanner::frame_at_or_after` | 296-311 | REAL / UNREACHABLE | S2 | F-001 (64 KiB cap overrides declared bound); no underflow (`data_len - start` with start clamped) |
| `SeekPlanner::header_at` | 313-317 | REAL | — | thin wrapper |
| `SeekPlanner::parse_header_at` | 319-439 | REAL (prod-reachable) | S3/S3 | F-002 (reserved bit), F-003 (36-bit), F-005 (overlong); block-size table verified against spec (192/576·2ⁿ/256·2ⁿ/+1 escapes); sr-escape byte widths correct for all 16 codes; CRC-8 verified by independent python reimplementation |
| `crc8_flac` | 442-461 | REAL | — | bitwise, ≤16-byte headers, cost justified; 4 vectors independently reproduced |
| `planner` (guard fn) | 463-485 | REAL / UNREACHABLE | S3 | bounds check correct (`>` allows empty region → typed no_frame downstream) |
| test `block_size_escape_is_one_based_and_number_comes_first` | 498-518 | REAL | — | would fail on order regression (swapped-bytes CRC assert) |
| test `eight_bit_escape_and_variable_blocking_parse` | 522-545 | REAL | — | pins 8-bit escape + 2-byte UTF-8 |
| test `crc8_guards_false_syncs` | 551-564 | REAL | — | single-byte flip → CRC reject |
| test `garbage_offsets_do_not_parse_as_headers` | 566-594 | SHALLOW (one case) | S3 | 0xFB case passes for the wrong reason (F-002) |
| test `fixed_stream_tail_frame_uses_the_stream_block_size` | 600-620 | REAL | — | mutation: frame-own-size rule → 184 428 ≠ 216 576 fails |
| test `planner_refuses_out_of_range_frames_start` | 622-631 | REAL | — | typed refusal |
| test helper `test_facts` | 633-645 | REAL | — | fixture facts |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| block_size_escape_is_one_based… | YES | swapped-order bytes fail CRC assert |
| eight_bit_escape_and_variable_blocking_parse | YES | exact block_size/number/first_sample equality |
| crc8_guards_false_syncs | YES | byte flip → `is_none` |
| garbage_offsets_do_not_parse_as_headers | PARTIALLY | 7 of 8 asserts are direct; the 0xFB assert passes via truncation, not the reserved bit (F-002) |
| fixed_stream_tail_frame_uses_the_stream_block_size | YES | exact first_sample/end_sample values |
| planner_refuses_out_of_range_frames_start | YES | typed-error match |
| tests/flac_seek.rs (integration, 9 targets) | YES | containment + decode-suffix equality over a real fixture; runs in baseline (#3, 1681 passed) — but only for ≤64 KiB frames and <2^31 samples (F-001/F-003 uncovered) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Seek: native SEEKTABLE when present; verified binary-search fallback over frames (flac_seek.rs)" | docs/formats/audio-flac.md:27-28 | UNREACHABLE | rg: no production caller of `SeekPlanner::resolve`; `container-flac-native` matrix row = not_started (feature-matrix.toml:240) |
| "SEEKTABLE seeking + binary-search fallback" inside implemented FLAC row | docs/feature-matrix.toml:709 | UNREACHABLE (inflated) | same rg proof; decode half of the row is REAL (baseline tests), seek half has no consumer |
| "sync, reserved rules, UTF-8 number, escapes and CRC-8 all verified" | flac/mod.rs:69-70 (doc for frame_header_at) | FALSE (partially) | F-002: reserved bit unchecked; F-003: 36-bit coding unsupported |
| "bounded by the stream's max_frame_size (or the SCAN_WINDOW cap when unknown)" | seek.rs:59-61 (module doc) | FALSE | F-001: `.min(SCAN_WINDOW)` applied unconditionally at seek.rs:300 |
| "the coding is defined for numbers up to 2^36" | seek.rs:377-379 | FALSE | F-003: `min(6)` caps at the 6-byte/31-bit form |

## 7. Dead / duplicated / unreachable code
- `resolve`, `resolve_inner`, `table_candidate`, `bisect`, `frame_at_or_after`, `planner`, `SeekPlan` — dead in production (F-004); exercised only by tests/flac_seek.rs and flac/fuzz.rs (`fuzz_codec_flac`, a registered fuzz target per baseline #18, not a shipped entry point).
- `ParsedFrameHeader::end_sample` used by bisect/table_candidate only (dead-in-prod transitively) but also exported and tested.

## 8. Dependencies & flags
- Module compiled under `codec-flac` (codecs/Cargo.toml default set — shipped workspace build includes it; baseline #1 build CLEAN).
- No `unsafe`, no panics on the audited paths (all indexing via `data.get`, `checked_add`); clippy clean per baseline #5.

## 9. Open questions for the next agent
- flac/metadata.rs `SeekIndex::from_body` trusts ascending order (no sort) — if the FLAC spec's sort requirement is a MUST, a hostile file with an unsorted SEEKTABLE yields wrong `table_candidate` picks that degrade to bisection (fail-safe), but metadata.rs's own doc row should be checked by its auditor.
- flac/decoder.rs:249 fallback to `decoded.time()` (claxon) — for 36-bit-number frames this silently produces the tail-misplaced pts the module itself documents as wrong; decoder.rs's auditor should confirm the fallback's blast radius.
