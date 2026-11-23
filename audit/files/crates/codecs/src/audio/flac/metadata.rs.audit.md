# AUDIT: crates/codecs/src/audio/flac/metadata.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-23 |
| Lines / functions | 472 / 8 prod fns (is_flac, SeekIndex::from_body, is_fixed_blocksize, parse_streaminfo, parse_streaminfo_inner, parse_stream, parse_stream_inner, read_u24); 6 tests |
| File verdict | REAL (hand-rolled spec parser, bounds-checked, typed errors; unit + fuzz-survival tests executed, pass) |
| Reachable from shipped entry point? | YES — apps/cli/src/pipeline_host.rs:249 `"flac"` → `FlacDecoder` whose configure/seed path calls `parse_streaminfo` (crates/codecs/src/audio/flac/decoder.rs:462,540); probe path via flac/mod.rs:90,119; seek path via flac/seek.rs:136 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p codecs --lib 'audio::flac::metadata' --offline` -> 6 passed / 0 failed; `cargo test -p codecs --test flac_fuzz_survival --offline` -> 2 passed / 1 ignored (deep run); sed verifications |

## 1. One-paragraph verdict
A clean, genuinely spec-driven FLAC metadata parser: `fLaC` probe, 34-byte STREAMINFO with correct 20+3+5+36 bit unpacking, a metadata-chain walk that is bounds-checked at every step (`data.get(..)` with typed truncation errors, `pos` strictly advances, no loop risk), SEEKTABLE multiple-of-18 gate with placeholder filtering, duplicate-STREAMINFO and type-127 refusals. The "zero panics" claim (lines 5-8) survives hostile reading: every index is behind a length gate or `get`. The two defects are documentation-precision issues: the field comments assert spec bounds (block-size floor 16, rate ceiling) that the validation does not apply and one of which is numerically wrong, and `SeekIndex`'s "points in ascending sample order" is an input-contract claim the parse neither sorts nor checks — safe today only because the sole consumer verifies each candidate against the real frame header before use.

## 2. What actually works (verified)
— `cargo test -p codecs --lib 'audio::flac::metadata' --offline` -> `test result: ok. 6 passed; 0 failed` (packed-field decode, truncation/geometry refusals, placeholder filtering, walk + frames_start, magic gate).
— `cargo test -p codecs --test flac_fuzz_survival --offline` -> `corpus_dirty_and_garbage_survive ... ok`, `fixtures_survive_truncation_flips_splices_and_fact_lies ... ok` (1 ignored deep run) — the mutation sweep asserts this module's error codes `flac.metadata.header_truncated/body_truncated/duplicate_streaminfo/seektable_length/block_type/streaminfo_missing` (flac_fuzz_survival.rs:102-107), so the error branches are exercised, not just the happy path.
— Verified by read: loop termination (`pos` advances by `4 + length ≥ 4`, bounded by `data.len()` via the `get` checks — lines 242-265, 308); `as_chunks::<18>()` drops any tail remainder (line 73) and the walk pre-rejects non-multiple lengths (281-289); the packed-field masks (174-177) match the spec layout; the sole sortedness consumer verifies candidates (`table_candidate`, seek.rs:196-216: header check `header.first_sample(self.facts) == point.sample` before acceptance, else binary-search fallback).

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/flac/metadata.rs:86-99 vs 182-199
Code (88-89, 98):
```
    /// Minimum block size in inter-channel samples (spec floor 16).
    pub min_block_size: u16,
```
```
    /// Sample rate in Hz (spec ceiling 65 535; 0 is invalid).
    pub sample_rate: u32,
```
Code (194-197):
```
    if min_block_size == 0 || max_block_size < min_block_size {
        return Err(unsupported(
            "flac.streaminfo.blocksize",
            format!("block sizes {min_block_size}..={max_block_size} violate min<=max, min>=1"),
```
What is wrong: the comments state spec bounds the validation does not enforce: (a) block-size floor 16 — code enforces only ≥1 and min≤max; (b) sample-rate ceiling — code rejects only rate 0, accepting any nonzero 20-bit value up to 1 048 575 Hz, while the comment's "ceiling 65 535" is itself the wrong number (the STREAMINFO rate field is 20 bits; the spec's valid maximum is 655 350 Hz — frame headers cannot encode more). (c) The spec requires STREAMINFO to be the FIRST metadata block; the walk accepts it anywhere (only duplicates refused, lines 269-278), so `PADDING…STREAMINFO` orderings invalid per spec parse clean here. All are leniencies on malformed input only; the claxon engine gates real decode. What correct looks like: validate `min_block_size >= 16`, `sample_rate <= 655_350`, and require `block_type == 0` for the first block — or fix the comments to state what is actually enforced. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/codecs/src/audio/flac/metadata.rs:57-64 vs 71-81
Code (57-60):
```
/// The parsed SEEKTABLE: points in ascending sample order per spec,
/// placeholders (sample `u64::MAX`) dropped — they carry no position
/// and only ever padded the table so an encoder could rewrite it in
/// place.
```
Code (71-79):
```
    pub(crate) fn from_body(body: &[u8]) -> SeekIndex {
        let mut points = Vec::new();
        for row in body.as_chunks::<18>().0 {
            let sample = u64::from_be_bytes(row[0..8].try_into().unwrap_or([0u8; 8]));
            let offset = u64::from_be_bytes(row[8..16].try_into().unwrap_or([0u8; 8]));
            if sample != u64::MAX {
                points.push(SeekPoint { sample, offset });
            }
```
What is wrong: the struct doc asserts "points in ascending sample order" but `from_body` pushes rows in file order — no sort, no ordering validation. An unsorted SEEKTABLE (spec violation) is accepted verbatim. Safe today only because the sole consumer, `table_candidate` (seek.rs:202-216), verifies each candidate against the actual frame header (`header.first_sample(self.facts) == point.sample`) and falls back to bisection on mismatch — but the doc invites a future consumer to binary-search `points` trusting an invariant that was never established. What correct looks like: either validate ascending order in `from_body` (drop or refuse unsorted rows) or change the doc to "file order; consumers must verify". Confidence: HIGH — Verification: read (consumer verification traced in seek.rs:196-216).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `is_flac` | 134-137 | REAL | — | starts_with marker |
| `SeekIndex::from_body` | 71-81 | REAL | S4 | Placeholder filter correct; ascending-order claim unenforced (F-002) |
| `StreamInfo::is_fixed_blocksize` | 113-117 | REAL | — | min==max |
| `parse_streaminfo` | 151-153 | REAL | — | Taxonomy-mapping wrapper |
| `parse_streaminfo_inner` | 157-211 | REAL | S4 | Correct 20/3/5/36 unpack; rate/bps/blocksize gates; upper bounds not enforced (F-001) |
| `parse_stream` | 223-225 | REAL | — | Wrapper |
| `parse_stream_inner` | 229-326 | REAL | S4 | Bounded walk, typed truncations, duplicate/127/seektable-length gates; STREAMINFO-not-first accepted (F-001) |
| `read_u24` | 329-332 | REAL | — | 0 → unknown per spec |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_accepts_only_the_marker | YES | 1 positive + 3 negative probes |
| streaminfo_parses_the_packed_field | YES | Hand-built packed field; all 8 fields + md5 edges asserted |
| streaminfo_rejects_short_bodies_and_bad_geometry | YES | Codes + error kinds asserted for short body and rate 0 |
| seek_index_drops_placeholders_and_keeps_points | YES | Placeholder row dropped, real points exact |
| parse_stream_walks_blocks_and_finds_frames_start | YES | frames_start anchored at the byte after the last block |
| parse_stream_rejects_magic_and_truncation_typed | YES | Magic, header-truncation, body-truncation codes asserted |
| (external) flac_fuzz_survival | YES | Mutation sweep over fixtures/corpus asserting this module's typed codes — ran: 2 passed |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bounds-checked slices, typed errors on every violation, zero panics" | metadata.rs:5-8 | REAL | Full read: every index behind a length gate/`get`; unit + fuzz-survival runs pass |
| "points in ascending sample order per spec" | metadata.rs:57 | FALSE (as an enforced property) | F-002: no sort/order check in from_body; safe only via consumer-side verification |
| "spec floor 16" / "spec ceiling 65 535" field bounds | metadata.rs:88-92, 98 | FALSE (validation enforces neither; the 65 535 figure is also wrong) | F-001: code checks min≥1, rate≠0 only |
| "Truncations, bad block types (127 …), non-multiple SEEKTABLE lengths, and duplicate STREAMINFO blocks are typed FlacError::Corrupts" | metadata.rs:24-28 | REAL | Lines 269-304; fuzz survival asserts the codes |

## 7. Dead / duplicated / unreachable code
- None. All functions have live callers (mod.rs, decoder.rs, seek.rs). `unwrap_or([0u8; 8])` on `try_into` of exact-size slices (74-75, 173) is dead-defensive but harmless.

## 8. Dependencies & flags
- No external deps — pure std parsing; sits behind the `codec-flac` feature (claxon engine) which is in the codecs default set; shipped in the release lane.

## 9. Open questions for the next agent
- seek.rs's `table_candidate` verification is the only guard for unsorted tables (F-002): if any future caller (e.g. a duration-from-seektable path) reads `points` without header verification, order assumptions silently break.
- Does claxon independently reject the lenient cases F-001 accepts (rate > 655 350, block size < 16, STREAMINFO not first)? If not, those values flow into `AudioFormat.rate` and duration math downstream.
