# AUDIT: crates/codecs/src/audio/flac/seek.rs
| Field | Value |
|---|---|
| Auditor | W07-A08 |
| Date | 2026-11-23 |
| Lines / functions | 646 / 12 (+7 tests, 1 helper) |
| File verdict | REAL-UNPROVEN for the shipped product: real, executed, correct code — but `SeekPlanner::resolve` (the file's whole purpose) is UNREACHABLE from any shipped entry point; only `parse_header_at` ships (decoder pts). |
| Reachable from shipped entry point? | NO (resolve/table_candidate/bisect/planner: no shipped caller). YES for `parse_header_at` only — crates/codecs/src/audio/flac/decoder.rs:249 `super::seek::SeekPlanner::parse_header_at(packet, 0)` ← `FlacDecoder::decode` ← pipeline_host.rs:253-260 ("flac" row, flac-in-wav/mp4/mkv lanes). The consumer the module was built for — the native-FLAC demuxer — refuses to exist: apps/cli/src/containers/es.rs:258-271 `open_flac` returns "the raw-FLAC elementary-stream demuxer has not landed". |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/flac_seek-f28325ba6212a26f --test-threads=2` → 4 passed, exit 0 (corpus row SKIPs: verified via `--nocapture` "corpus: LUMEN_CORPUS unset ... — SKIP"); claxon 0.4.3 source inspected at ~/.cargo/registry/.../claxon-0.4.3/src/frame.rs:773 |

## 1. One-paragraph verdict
This is the best-engineered file in my assignment: a spec-faithful frame-header parser (number-first field order, CRC-8 verified, reserved codes rejected), a fail-safe SEEKTABLE resolver that verifies every advisory point against the real header before trusting it, and a bisection fallback with a proven-terminating bracket. The test suite is hostile and real — I ran it: both resolvers must agree, containment must hold, and bytes from the plan offset must decode to exactly the full decode's suffix; a corrupted seek-table row must degrade to the same answer. It is also dead code for the product: the native-FLAC demuxer that would call `resolve()` "has not landed" (typed refusal at open), no other lane uses the planner, so the SEEKTABLE/bisection machinery executes only under test and fuzz. "Exact-sample seeking" is, by design, deferred to a consumer that does not exist; the only shipped use of this file is the per-packet `first_sample` header parse for pts.

## 2. What actually works (verified)
— `./target/debug/deps/flac_seek-f28325ba6212a26f --test-threads=2` → `test result: ok. 4 passed` (seektable_and_binary_search_agree_across_the_stream runs ~25 targets × {table, no-table} over a real encoder fixture with the decode-suffix equality proof; corrupted_seektable_degrades_to_bisection; past_end_and_frameless typed errors).
— claxon tail-frame claim VERIFIED against the vendored source: claxon-0.4.3/src/frame.rs:773 `BlockTime::FrameNumber(fnr) => header.block_size as u64 * fnr as u64` multiplies by the frame's OWN (tail-shortened) block size; this file's `first_sample` override (`number × max_block_size` for fixed streams, seek.rs:105-106) is the libFLAC/ffmpeg-correct rule.
— Field order (number before block-size/sample-rate escapes) verified against claxon's read order (frame.rs:244-261 reads `block_time` before the escape bytes).

## 3. Findings
F-001 [S3]
Where: seek.rs:166-190 (resolve/resolve_inner) + apps/cli/src/containers/es.rs:262-270
Code:
```
    pub fn resolve(&self, target_sample: u64) -> Result<SeekPlan, EngineError> {
        self.resolve_inner(target_sample)
            .map_err(FlacError::into_engine_error)
    }
```
Code (the missing consumer, es.rs:265-268):
```
        "format `flac` probes (the fLaC marker) but the raw-FLAC elementary-stream demuxer \
         has not landed: crates/demux/src/es covers h264/hevc/mp3/aac (card 5.I3a's slice) — \
```
What is wrong: not a code defect — an unreachability fact. `SeekPlanner::resolve` (with `table_candidate`, `bisect`, `frame_at_or_after`, the `planner` guard) has zero shipped callers: only tests/flac_seek.rs and flac/fuzz.rs call it. A native .flac file cannot even be opened (`open_flac` refuses), and the FLAC-in-WAV/MP4/MKV/Ogg lanes seek by their containers, never through this planner. The feature-matrix note "SEEKTABLE seeking + binary-search fallback" rides the `codec-audio-flac` **implemented** row (docs/feature-matrix.toml:709) — the code exists and is tested, but no user can reach a FLAC seek through it. (The matrix's own `container-flac-native` row honestly says not_started on all platforms, line 241.) Why it matters: real, maintained, tested code with no product path = drift risk and inflated evidence. Confidence: HIGH — Verification: read (exhaustive caller search) + ran the tests that do exercise it.

F-002 [S3]
Where: seek.rs:214-219 (with 199-204)
Code:
```
            if header.end_sample(self.facts) > target {
                return Some(SeekPlan {
                    byte_offset: absolute,
                    first_sample: header.first_sample(self.facts),
                });
```
What is wrong: the SEEKTABLE fast path bails to bisection whenever the last point ≤ target is a frame that does NOT contain the target (variable-blocksize streams where the next seek point's frame is the container, or a target inside the inter-point span when the point's frame is short). In a variable-blocksize stream whose table points are sparse, most targets fall past their point frame's end → `table_candidate` returns None → full O(log n) bisection, silently discarding the table. Not a wrong-seek risk (fail-safe), but the "SEEKTABLE path" degenerates to the fallback exactly in the streams tables were made for. What correct looks like: bisect from `absolute` as the new `lo` bracket (the comment at 210-213 even describes this refinement) instead of re-bisecting the whole stream. Confidence: MEDIUM — Verification: read (no variable-blocksize-with-table fixture exists to execute it; the committed carrier is fixed-blocksize).

F-003 [S4]
Where: seek.rs:299-301
Code:
```
        let limit = u64::from(self.facts.max_frame_size.unwrap_or(u32::MAX))
            .min(SCAN_WINDOW)
            .min(data_len - start);
```
What is wrong: when STREAMINFO declares `max_frame_size` (most encoder output does), the scan window shrinks to that value — correct for consecutive frames; but when the declaration is a LIE (corrupt or under-declaring encoder), a probe mid-huge-frame scans `max_frame_size` bytes, finds nothing, and sets `hi = mid` — converging, so only a perf/behavior note, no wrong answer. Documented as bounded work; acceptable. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: seek.rs:367-372
Code:
```
        let first = at(cursor)?;
        let mut additional = first.leading_ones().min(6) as u64;
        if additional == 1 {
            return None; // 0b10xxxxxx leading byte: invalid start
        }
        additional = additional.saturating_sub(1);
```
What is wrong: a lead byte of 0xFE/0xFF (leading_ones 7/8) is clamped to the 6-byte form and ACCEPTED (0xFF & 0x03 ≠ 0 passes the overlong check), though 0xFE/0xFF are invalid UTF-8 lead bytes in the FLAC coding; the 6-byte mask `0x7f >> 5` = 0x03 also grabs a bit (bit 1) that belongs to the lead pattern, so such inputs decode to a number with a spurious bit 31. Harmless in practice (CRC-8 still gates; frame numbers are ≤ 31 bits for fixed blocking and rejected at 390), but the parser is more permissive than the coding it claims to enforce. Confidence: HIGH — Verification: read (bit arithmetic).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ParsedFrameHeader::first_sample | 102-110 | REAL | — | ffmpeg/libFLAC tail rule; claxon quirk verified in vendored source |
| ParsedFrameHeader::end_sample | 115-118 | REAL | — | containment bound |
| SeekPlanner::new | 144-156 | REAL | — | |
| resolve / resolve_inner | 166-190 | REAL (UNREACHABLE in shipped build) | S3 | past_end typed; table→bisect dispatch |
| table_candidate | 196-222 | REAL (UNREACHABLE) | S3 | verified fail-safe; F-002 fallback degeneration |
| bisect | 245-287 | REAL (UNREACHABLE) | — | termination argument in doc verified by reading the three shrink branches |
| frame_at_or_after | 296-311 | REAL (UNREACHABLE) | S4 | bounded scan; F-003 |
| header_at / parse_header_at | 315-317 / 327-439 | REAL (SHIPPED via decoder.rs:249) | S4 | full structural rule set + CRC-8; F-004 UTF-8 permissiveness |
| crc8_flac | 448-461 | REAL | — | poly 0x07 init 0, test vectors |
| planner (guard) | 469-485 | REAL (UNREACHABLE — fuzz/test only) | — | frames_start range check |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seektable_and_binary_search_agree_across_the_stream | Yes | decode-suffix equality against the full decode is the strongest possible pin; both resolvers must agree on ~25 targets incl. tail frame and seek-point ±1 |
| corrupted_seektable_degrades_to_bisection | Yes | damages a real row's offset; same targets must still resolve identically |
| past_end_and_frameless_streams_are_typed_errors | Yes | asserts `flac.seek.past_end` / `flac.seek.no_frame` codes |
| corpus_stream_seeks_by_bisection_alone | VACUOUS here | corpus absent → prints "SKIP" and passes; exercised targets incl. 88.2 kHz/24-bit only when /tmp/my-project/corpus-v4 exists (I verified the SKIP path fires) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "SEEKTABLE seek-point resolution with a sample-accurate binary-search fallback" | seek.rs:1-2 | REAL (code+tests) / UNREACHABLE (product) | tests executed pass; no shipped caller (F-001) |
| "claxon's Block::time() uses the frame's own size and misplaces the tail" | seek.rs:53-55 | REAL | claxon-0.4.3/src/frame.rs:773 verbatim: `BlockTime::FrameNumber(fnr) => header.block_size as u64 * fnr as u64` |
| "the header CRC-8 … is verified" | seek.rs:46-49 | REAL | seek.rs:425-432; test crc8_guards_false_syncs flips a payload byte and the parse fails |
| "a corrupt/unsorted/mismatched row must degrade to the fallback, never produce a wrong seek" | seek.rs:21-23 | REAL | corrupted_seektable_degrades_to_bisection executed, passed |
| "The consumer (the future native-FLAC demuxer card)" | seek.rs:11-12 | honest future-tense | open_flac refusal confirms it has not landed |
| feature-matrix note "SEEKTABLE seeking + binary-search fallback" on the implemented codec-audio-flac row | docs/feature-matrix.toml:709 | MISLEADING | the machinery has no shipped caller (F-001); matrix-wide 0/252 verification failure already on record (audit/runs/baseline/xtask-parity-report.txt) |

## 7. Dead / duplicated / unreachable code
- `resolve`, `table_candidate`, `bisect`, `frame_at_or_after`, `planner`: unreachable from shipped entry points (tests + fuzz only).
- `parse_header_at` / `first_sample`: shipped (decoder pts).

## 8. Dependencies & flags
No new deps. Depends on super::error, super::metadata (SeekIndex/StreamInfo), media-types. The FLAC decode engine is claxon =0.4.3 (Cargo.toml:86) — symphonia is NOT used for FLAC (the matrix note "symphonia-bundle-flac" at line 709 is a name error in the doc; the actual dep is claxon — recorded honestly in the Cargo comment).

## 9. Open questions for the next agent
- Does any plan exist to land the native-FLAC demuxer (card 5.I3a follow-up) that would wire `resolve()`? Until then this file's maintenance cost buys nothing shippable.
- flac/fuzz.rs claims a `+FB` fuzz wiring — verify the registered fuzz target actually drives `planner.resolve` (I confirmed the in-tree calls at fuzz.rs:71-78,113-119 exist; whether the cargo-fuzz registry target is built/run in CI is unverified here).
