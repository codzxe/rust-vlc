# AUDIT: crates/codecs/src/video/h264/parser/nal.rs#part1
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Chunk boundary audited | **lines 1-561** (part1 of 2; ends after `AvcConfig::looks_like_avcc` at 559. Part2 (W06-A01) = 562-1046: `read_counted_nal`, `BitReader`, tests) |
| Lines / functions | 561 / 13 (H264Error, NalUnitType+2, NalHeader+split, next_start_code, trim_trailing_zeros, AnnexBNals+next, annexb_nals, unescape_rbsp_into, unescape_rbsp, AvcNals+new+next, AvcConfig+parse+looks_like_avcc) |
| File verdict (this part) | REAL |
| Reachable from shipped entry point? | YES — the H.264 software lane's framing/EPB/avcC layer: `lumen open sample-420.mp4` → apps/cli/src/pipeline_host.rs:346 `H264SwDecoder::new` → sw.rs consumes `AvcConfig::parse` (avcC extradata), `AvcNals` (MP4 length-prefixed samples), `AnnexBNals` (Annex-B lanes), `unescape_rbsp_into` (the EPB bridge, codecs/Cargo.toml:169-173). Executed: baseline `mp4-smoke-open-session-420.txt` (plays) + h264_sw_differential 17/17 ok (test-workspace.log.txt:1063-1080, lanes A–D × both entry forms + hostile sweeps) + the 9 in-file nal tests (:596-607) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | baseline log greps (nal tests :596-607, h264_sw_differential :1063-1080); sed -n quote verification; hand-trace of the Annex-B end-index arithmetic for 3- and 4-byte prefixes; overflow analysis of AvcNals on 32/64-bit; read apps/web/Cargo.toml:79-110 (wasm codec gating) |

## 1. One-paragraph verdict
This is the NAL layer the shipped H.264 software decoder actually runs on, and
the baseline proves it end-to-end: the 4:2:0 MP4 plays, and the differential
suite decodes six ffmpeg-generated lanes in BOTH entry forms through exactly
these splitters. The Annex-B splitter's index arithmetic is correct for 3- and
4-byte prefixes (I hand-traced both, including the leading-zero and
adjacent-start-code edges — the trailing-zero trim closes the 4-byte-prefix
overlap exactly), the emulation-prevention walker is spec-strict with two
typed violation codes, and the avcC parser documents and handles the ffmpeg
PPS-count deviation honestly. One real defect: `AvcNals::next`'s length
arithmetic (`len + self.length_size`, and the slice end
`self.length_size + len`) overflows on 32-bit targets for length prefixes near
2^32 — wrapping into a `start > end` slice that panics even in release builds;
the shipped builds are 64-bit or wasm-decoder-less, so it is latent, but the
crate compiles for opt-in wasm32 codec builds with no guard and the
property-mutation test cannot see it on a 64-bit host. One S4: the iterator's
doc claims truncated NAL bodies are "yielded as-is" while the code trims
trailing zeros from them.

## 2. What actually works (verified)
— Executed, shipped lane: `audit/runs/baseline/mp4-smoke-open-session-420.txt`
(h264 CB 4:2:0 MP4: Buffering → Playing → Stopped → Idle) — the avcC parse,
AvcNals framing and EPB unescape of that file's samples all ran through this
file.
— `h264_sw_differential` 17/17 ok (baseline :1063-1080): lanes A (baseline),
B (Main CABAC), C (B-frames), D (High IPB) each in BOTH annexb and mp4 entry
forms, plus `hostile_garbage_packets_recover`, `hostile_truncation_sweep_is_
panic_free`, `contract_damaged_extradata_refuses_typed` — the splitters and
avcC paths are pinned against ffmpeg output bit-exactly.
— Annex-B end-index math hand-traced: for `00 00 01` at i, stop=i+3,
end_raw=i (body excludes both prefix zeros); for `00 00 00 01` at i, the scan
matches at i+1, stop=i+4, end_raw=i+1 (body includes ONE prefix zero),
which `trim_trailing_zeros` (236-245) strips — exact closure for both prefix
forms, and `end_raw.max(start).min(len)` (302) makes adjacent start codes
yield an empty body that fails header parsing typed (documented 252-255).
— `unescape_rbsp_into` clears the caller buffer first (327) — this answers
the cross-question in my sps.rs audit (HEVC reuses the same convention via
its own nal.rs); `00 00 03` at the exact end of the NAL is the typed
`h264.ep.dangling` (332-339); suffix > 3 is the typed `h264.ep.bad_suffix`
(345-349) — and that strictness is SPEC-CORRECT: a conforming encoder always
escapes an RBSP `00 00 03`, so a valid NAL never carries `00 00 03` + suffix
> 3 (the module's own claim at 320-325 holds).
— `AvcConfig::parse` reserved-bit gate (`bytes[4] & 0xfc != 0xfc`), version
gate, zero-count gates, and the documented ffmpeg PPS-count masking all read
correctly; `looks_like_avcc` (557-559) cannot false-positive on Annex-B
`CodecPrivate` (those start with a 0x00 start code, not 0x01).

## 3. Findings

### F-001 [S2] AvcNals::next length arithmetic overflows on 32-bit targets — release-mode panic via a start>end slice
Where: nal.rs:425-445
Code:
```rust
        let mut len: usize = 0;
        for &b in &rest[..self.length_size] {
            len = (len << 8) | b as usize;
        }
```
```rust
        let total = len + self.length_size;
        if total > rest.len() {
```
```rust
        self.pos += total;
        Some(Ok(&rest[self.length_size..self.length_size + len]))
```
What is wrong: with `length_size = 4`, `len` can be up to 0xFFFFFFFF. On a
32-bit `usize` target, `len + self.length_size` wraps (e.g.
0xFFFFFFFF + 4 = 3): the `total > rest.len()` guard passes for any sample
with ≥3 bytes left, and the slice `&rest[4..4 + 0xFFFFFFFF mod 2^32]` =
`&rest[4..3]` panics — slice range start>end panics in release builds too
(no overflow-check dependency). The `needed: total as u64` in the typed
Truncated error would also report the wrapped value.
Why it matters: hostile/corrupt MP4 sample data (a 4-byte length prefix near
2^32) crashes the decode worker on any 32-bit build. NOT reachable in the
shipped artifacts today — the wasm32 web host is deliberately decoder-less
(apps/web/Cargo.toml:81-92: codecs is `optional`, `default-features = false`,
"the SHIPPED module enables none of them") and the CLI/desktop/tauri builds
are 64-bit (where `usize` is 64-bit and the arithmetic cannot overflow). But
the crate compiles for the opt-in `builtin-codecs` wasm32 configurations with
no target guard, and the property-mutation test (:607) runs on a 64-bit host
where the bug is invisible. S2 (panic on malformed input in compilable
configurations), not S1, because no shipped build reaches it.
What correct looks like: compute in u64 (`let total = u64::from(len) +
self.length_size as u64;` compare against `rest.len() as u64`) or
`checked_add` with a typed Truncated error — the file's own discipline
elsewhere.
Confidence: HIGH — Verification: read + arithmetic trace; 64-bit safety
confirmed (max total 0x1_0000_000_03 fits); 32-bit panic path traced to the
slice expression.

### F-002 [S4] Doc/code mismatch: "truncated … yielded as-is" vs unconditional trailing-zero trim
Where: nal.rs:250-255 (doc) vs 302-303 (code)
Code:
```rust
/// codes) are yielded and fail header parsing downstream — typed, not
/// silently dropped.
```
```rust
        let body = &self.data[start..end_raw.max(start).min(self.data.len())];
        Some(trim_trailing_zeros(body))
```
What is wrong: the doc promises a NAL body truncated by end-of-data is
yielded AS-IS, but `trim_trailing_zeros` runs unconditionally — a truncated
body ending in real `0x00` payload bytes has them stripped before its parser
sees it. A truncated NAL fails typed anyway (missing rbsp stop bit), so the
impact is a shifted diagnostic position, never a wrong success.
Why it matters: doc-vs-code drift on the hostile-input path the fuzz corpus
specifically targets.
What correct looks like: trim only when a following start code exists (the
zeros provably belong to its prefix), or fix the doc to name the trim.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| H264Error (6 variants + Display + Error) | 29-92 | REAL | — | every failure names its rule; payloads carry needed/have |
| NalUnitType::from_id / id | 136-156 / 160-179 | REAL | — | Table 7-1 exact; reserved/unspecified preserved verbatim (pinned by test :603) |
| NalHeader::split | 199-213 | REAL | — | empty → Truncated{1,0}; forbidden bit typed; field extraction exact (test :602) |
| next_start_code | 219-230 | REAL | — | bounded (i < len−2), no OOB; handles ≥3 leading zeros |
| trim_trailing_zeros | 236-245 | REAL | S4 | unconditional trim → F-002 doc drift |
| AnnexBNals::new / Iterator::next | 269-305 | REAL | — | leading-garbage skip, 3/4-byte prefix closure (hand-traced §2), adjacent-start-code empty body typed; `?` on no-start-code ends iteration cleanly |
| annexb_nals | 310-312 | REAL | — | collect convenience |
| unescape_rbsp_into | 326-357 | REAL | — | clears buffer; two typed violation codes; spec-strict suffix rule verified correct; executed by differential + fuzz |
| unescape_rbsp | 363-367 | REAL | — | one-shot wrapper |
| AvcNals::new | 390-401 | REAL | — | 1..=4 length gate typed |
| AvcNals::Iterator::next | 407-447 | REAL | S2 | terminal-on-error discipline ✓ (zero-length and truncation both end the walk); 32-bit overflow F-001; 64-bit paths all typed |
| AvcConfig::parse | 494-550 | REAL | — | version/reserved/count gates; ffmpeg PPS-count deviation documented AND handled; masked 5-bit counts accept both writer forms (test :598-599) |
| AvcConfig::looks_like_avcc | 557-559 | REAL | — | heuristic; no Annex-B false positive (first byte 0x00 ≠ 0x01) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| annexb_splits_three_and_four_byte_prefixes (:596) | Yes | exact splits for both prefix forms |
| emulation_removal_and_typed_violations (:601) | Yes | roundtrip + both violation codes pinned |
| avc_length_prefixed_split_with_typed_truncation (:597) | Yes on 64-bit | truncation/zero-length typed; CANNOT catch F-001 (runs on 64-bit host) |
| avcc_parse_roundtrip_and_error_paths / avcc_accepts_ffmpeg_writer_reality (:598-599) | Yes | both writer realities pinned |
| property_mutations_never_panic_or_hang (:607) | Yes for 64-bit-visible mutations | mutation sweep; blind to the 32-bit overflow (F-001) |
| h264_sw_differential 17 lanes (:1063-1080) | Yes, strongly | bit-exact vs ffmpeg through THIS file's splitters in both entry forms; hostile garbage/truncation sweeps panic-free |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Both splitters are iterators of borrowed slices (zero copy, no allocation)" | 16-22 | REAL | read: no allocation in the iterators |
| "NAL bodies truncated by end-of-data are yielded as-is" | 251-253 | FALSE for zero-ending bodies | F-002 |
| "no legal RBSP produces this" (bad_suffix) | 320-322 | REAL | spec escape obligation verified (§2) |
| ffmpeg PPS-count deviation "verified on every corpus avcC" | 473-486 | REAL | test :599 pins the writer reality |
| "All reads are bounds-checked and return typed errors; the reader never panics" (BitReader — part2) | 595-596 | (part2 owner's claim) | A01's audit |

## 7. Dead / duplicated / unreachable code
- Nothing dead in part1; every item is on the shipped h264-sw path (sw.rs
  consumes all four public surfaces).
- Note: `AnnexBNals` is also used by demux/src/es/annexb.rs's h264 lane —
  double-shipped through both the decode lane and the raw-ES open lane.

## 8. Dependencies & flags
- Unconditional in `codecs` (h264 module, video/mod.rs:27-29); apps/cli
  default features; the wasm32 edge is optional and default-off
  (apps/web/Cargo.toml:90). No unsafe, no external deps.

## 9. Open questions for the next agent
- part2 (W06-A01) owns `BitReader` — my sps.rs/vp9 audits assume
  `bits(n)`/`ue()`/`se()` typed-truncate; their audit confirms.
- sw.rs owner: does the EPB bridge re-unescape NALs that demux already
  unescaped (double-unescape risk)? The Cargo comment
  (codecs/Cargo.toml:169-173) says the crate feeds through
  AnnexBNals/AvcNals/unescape_rbsp_into — one unescape, but worth a
  call-site check in sw.rs.
- If a 32-bit build lane ever ships (wasm `builtin-codecs` or armv7), F-001
  must be fixed first.
