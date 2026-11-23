# AUDIT: crates/codecs/src/video/h264/parser/nal.rs
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| Lines / functions | 1045 / 18 (+9 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `AnnexBNals`/`AvcNals`/`AvcConfig`/`unescape_rbsp_into`/`BitReader` ← `h264/sw.rs:93,236-249,267,472-473` (H264SwDecoder) ← `apps/cli/src/pipeline_host.rs:346 "h264"|"avc1" => self.seeded(.., H264SwDecoder::new)`; `codec-h264-sw` is in codecs default features and apps/cli depends on codecs with defaults (apps/cli/Cargo.toml:51) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 h264::parser::nal` → 9 passed / 0 failed (exit 0) |

## 1. One-paragraph verdict
The NAL layer is real and tight. Annex-B start-code splitting correctly handles 3/4-byte prefixes, leading garbage, trailing-zero attribution (`trim_trailing_zeros`) and empty NALs (yielded, typed downstream); emulation-prevention removal implements §7.4.1 exactly, with both violation classes typed (`bad_suffix`, `dangling`) and the dangling-`00 00 03`-at-EOF case actually caught (bounds condition verified by hand); NAL type dispatch matches Table 7-1 element-for-element (checked all 32 ids); the Exp-Golomb reader's ue/se mappings were verified against §9.1/§9.2 by hand (k=0..4 → 0,+1,−1,+2,−2 correct); `avcC` parsing documents and tests the ffmpeg bare-PPS-count deviation byte-for-byte. Defects are peripheral: one 32-bit-only unchecked arithmetic in `AvcNals`, a dead duplicated predicate, and two doc/error-taxonomy nits. I ran all 9 tests (pass).

## 2. What actually works (verified)
— verified by `./target/debug/deps/codecs-bae0f1756cc0a809 h264::parser::nal` → "9 passed; 0 failed" (includes `property_mutations_never_panic_or_hang`: 3000 xorshift-mutation rounds over Annex-B+AVCC walks, zero panics). Binary newer than source (18:07 vs 17:35).

## 3. Findings
F-001 [S3]
Where: nal.rs:436-445
Code:
```
        let total = len + self.length_size;
        if total > rest.len() {
```
What is wrong: `len` is up to 2^32−1 (4 length bytes) added to `length_size` in usize with no checked_add. On 64-bit targets this cannot overflow; on 32-bit targets (wasm32 = usize u32) `len ≥ 0xFFFFFFFD` wraps: release wrap makes `total` tiny so the guard passes and `&rest[self.length_size..self.length_size + len]` wraps to a start>end range → panic (slice index); debug builds panic on the overflow itself. Why it matters: this is unchecked arithmetic on parsed input, exactly the checklist's S-class defect; the only thing saving the shipped lanes is that (a) native is 64-bit and (b) the wasm32 module tree-shakes the never-dispatched h264 parser (apps/web/Cargo.toml:79-96). What correct looks like: `len.checked_add(self.length_size).ok_or(Truncated)` or a u64 total. Confidence: HIGH — Verification: read (arithmetic traced; wasm32 reachability confirmed absent in shipped module per apps/web/Cargo.toml comments — hence S3 not S1).
F-002 [S3]
Where: nal.rs:688-699
Code:
```
    /// Is there at least one more `1` bit in the unread data (the
    /// `more_rbsp_data()` predicate, §7.2)?
    #[must_use]
    pub fn more_rbsp_data(&self) -> bool {
```
What is wrong: dead public API — zero callers repo-wide (rg over crates/ finds only this definition, a *private duplicate* `fn more_rbsp_data` in sei.rs:302 used by sei.rs:224, and the HEVC twin). No test in this file exercises it, so the nal.rs copy is both dead and untested while an identical private copy carries the real workload. Duplication + dead code + untested predicate in a parser layer. Confidence: HIGH — Verification: read (rg 'more_rbsp_data' across crates/).
F-003 [S4]
Where: nal.rs:67-69 vs 501-505
Code:
```
    /// A syntactically parseable feature this parser deliberately
    /// does not model (FMO slice groups, `avcC` version != 1).
    Unsupported {
```
What is wrong: the doc comment on `H264Error::Unsupported` claims `avcC version != 1` is modeled as Unsupported (deliberately-not-modeled); the code returns `Invalid { "h264.avcc.version" }`. The taxonomy documentation contradicts the code; callers matching on Unsupported for avcC version drift would be wrong. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: nal.rs:579-584
Code:
```
    if len == 0 || bytes.len() < *pos + len {
        return Err(H264Error::Truncated {
            needed: (*pos + len.max(1)) as u64,
            have: bytes.len() as u64,
        });
    }
```
What is wrong: a zero-length NAL inside an `avcC` record is reported as `Truncated` with `needed = pos+1 ≤ have` — a truncation error claiming fewer bytes were needed than were present. It is an error either way, but the diagnosis (truncation) is wrong (it's structural: zero-length) and the numbers are self-contradictory. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: nal.rs:519-541
Code:
```
        let mut sps = Vec::with_capacity(num_sps as usize);
        for _ in 0..num_sps {
            let nal = read_counted_nal(bytes, &mut pos, 2)?;
            sps.push(nal.to_vec());
```
What is wrong: the avcC slots are not type-checked here — a PPS NAL in the SPS slot is accepted by `AvcConfig::parse`. Mitigated: downstream `sps.rs:177-179` / `pps.rs:76-78` refuse with typed `wrong_nal_type` errors (verified), so no wrong decode; the avcC layer itself just defers validation. Cosmetic layering note only. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| H264Error + Display/Error | 28-92 | REAL | — | typed, no panics |
| NalUnitType::from_id / id | 136-179 | REAL | — | Table 7-1 verified element-by-element (1..15 named, 16-23 Reserved, 0/24-31 Unspecified); round-trip test covers all 32 |
| NalHeader::split | 199-213 | REAL | — | forbidden_zero_bit enforced (§7.4.1 "shall discard"), split_first no-panic |
| next_start_code | 219-230 | REAL | — | `end = len-2` guard keeps i+2 in bounds; 4-byte prefix found at last 00 00 01 — correct |
| trim_trailing_zeros | 236-245 | REAL | — | trailing-zero attribution to next start code; spec-correct (RBSP stop bit) |
| AnnexBNals::next | 282-304 | REAL | — | leading garbage skip, empty NALs yielded typed, no infinite loop (pos strictly advances or done), defensive clamp on slice |
| annexb_nals | 310-312 | REAL | — | convenience collect |
| unescape_rbsp_into | 326-357 | REAL | — | §7.4.1 exact: `00 00 03 {0-3}` collapse, bad suffix + dangling typed; dangling bound condition `i+2 < len` verified correct (03-as-last-byte → nal.get(i+3)==None → error); no double-processing after 4-byte skip |
| unescape_rbsp | 363-367 | REAL | — | one-shot wrapper |
| AvcNals::new/next | 390-447 | REAL | F-001 | zero-padding tail tolerated, zero-length refused (no spin), typed truncation; 32-bit overflow hazard |
| AvcConfig::parse | 494-550 | REAL | F-005 | version/reserved/count guards; ffmpeg bare-PPS-count deviation documented AND tested byte-for-byte; trailing extension bytes deliberately unread (documented) |
| AvcConfig::looks_like_avcc | 557-559 | REAL | — | heuristic consistent with parse's acceptance |
| read_counted_nal | 563-588 | REAL | F-004 | bounds-checked; zero-length misreported |
| BitReader::bit/bits | 618-646 | REAL | — | MSB-first, typed truncation, n∈1..=32 enforced |
| BitReader::ue/se | 655-685 | REAL | — | ue cap 31 leading zeros; se mapping hand-verified §9.2 (k=0→0, 1→+1, 2→−1, 3→+2, 4→−2) |
| BitReader::more_rbsp_data | 690-699 | DEAD | F-002 | no callers; sei.rs:302 has its own copy |
| BitReader::finish | 708-739 | REAL | — | stop-bit scan + same-byte mask + tail-bytes-zero check, all correct (traced bit_off 0..7) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| nal_classification_covers_table_7_1 (747) | YES | round-trips all 32 ids both directions |
| header_split_checks_forbidden_bit_and_truncation (763) | YES | exact error equality |
| annexb_splits_three_and_four_byte_prefixes (777) | YES | asserts exact NAL bytes incl. trailing-zero trim + garbage skip + no-start-code → None |
| emulation_removal_and_typed_violations (790) | YES | exact outputs and exact error codes incl. dangling |
| avc_length_prefixed_split_with_typed_truncation (815) | YES | split + Truncated{needed:13,have:5} + padding/junk discrimination + length_size=5 rejection |
| avcc_parse_roundtrip_and_error_paths (858) | YES | field equality + 4 typed error paths |
| avcc_accepts_ffmpeg_writer_reality (915) | YES | byte-for-byte corpus CodecPrivate shape; would fail if count masking or trailing-byte tolerance regressed |
| bitreader_exp_golomb_and_trailing_rules (947) | YES | exact ue/se values and trailing-bit accept/refuse matrix |
| property_mutations_never_panic_or_hang (989) | PARTIAL | proves termination/no-panic for 3000 mutation rounds, but only smoke-parses SPS/PPS (ue+finish), not full syntax; does not cover the 32-bit length-overflow shape (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Both splitters are iterators of borrowed slices (zero copy...)" | nal.rs:17-19 | REAL | read: both yield `&'a [u8]` |
| "the steady-state parse path allocates nothing" | nal.rs:21-22 | REAL-UNPROVEN | reserve-only-if-needed pattern is right; no allocation probe run (cargo-build-budget) |
| "avcC version != 1" is Unsupported | nal.rs:68 | FALSE (doc bug) | code L501-505 returns Invalid; F-003 |
| ffmpeg writes bare PPS count | nal.rs:478-481 | REAL | test 915 pins the exact bytes; corpus fixture cross-check claimed in test comment |

## 7. Dead / duplicated / unreachable code
- `BitReader::more_rbsp_data` (L690): dead, duplicated in sei.rs:302 (F-002).
- Everything else is wired: sw.rs + decode.rs + sps/pps/slice/sei consume the rest.

## 8. Dependencies & flags
No unsafe. No feature gate on the parser module itself (`pub mod h264` unconditional, video/mod.rs:29); the *decode engine* behind it is `codec-h264-sw` (codecs default features; apps/cli uses defaults). wasm32: codecs is default-features=false + optional there, parsers ride along but tree-shake (F-001 latent there).

## 9. Open questions for the next agent
- CROSS-FILE (not mine): `apps/cli/src/host.rs:1638-1653` `engine_info()` returns a HARD-CODED features/decoders list ("first-vertical:avi-mjpeg-pcm", decoders: mjpeg, pcm_s16le) while `pipeline_host.rs:242-346` actually dispatches mjpeg/flac/mp3/mp2/vorbis/opus/aac/alac/adpcm/av1/vp9/dnxhd/h264/pcm. engine-info under-reports the real registry — the "which registry lies" thread from the baseline (audit/runs/baseline/mp4-smoke.md:99) resolves to: engine-info lies by hardcoding. Owner of apps/cli/src/host.rs should file this.
- sw.rs owner: confirm the "bit-exact vs ffmpeg 7.1.5 on six lanes" differential claim (tests/h264_sw_differential.rs) — asserted in a comment at pipeline_host.rs:334-338, not verified by me.
