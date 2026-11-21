# AUDIT: crates/codecs/src/video/h264/parser/nal.rs#part2 (lines 562–1046)
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 485 (of 1046) / 12 (9 production items + 8 test fns) |
| File verdict | UNREACHABLE (chunk-dominant BitReader subtree has no shipped caller; the chunk's one shipped-reachable helper `read_counted_nal` is REAL and executed in baseline) |
| Reachable from shipped entry point? | SPLIT — `read_counted_nal`: YES, chain `lumen open` → apps/cli/src/pipeline_host.rs:346 `H264SwDecoder::new` → sw.rs:237 `AvcConfig::parse` → nal.rs:521/539 `read_counted_nal`; executed in baseline (`audit/runs/baseline/mp4-smoke-open-session-420.txt`: sample-420.mp4 plays — its `avcC` extradata parses through this helper). `BitReader` (incl. `more_rbsp_data`): NO — production callers are h264/decode.rs (itself unreachable, see my decode.rs audit) + hw/vaapi/session.rs + h264/fuzz.rs + tests only. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "more_rbsp_data"` (whole repo) → h264 method has ZERO callers; `sed -n` + `od -c` quote verification (nal.rs 687-699; hevc nal.rs 828-846; sw.rs 236-248); `rg -n -F "SeqParameterSet::parse"` → h264 prod caller is decode.rs:368 only; `rg -n "h264::parser"` outside crates/codecs → none; baseline: mp4-smoke-open-session-420.txt, test-workspace.log.txt (nal tests ok) |

## Chunk boundary note
Assigned part2 ≈ lines 523–1046; I audited 562–1046 so that `AvcConfig::parse` (494–550) and `looks_like_avcc` (557–559) stay whole in part1 (W06-A09). My chunk: `read_counted_nal` (563–588), `BitReader` + impl (598–740), `mod tests` (742–1046). Cross-boundary note: `AvcConfig::parse` calls MY `read_counted_nal` at 521/539 — that call site is the shipped-reachability bridge for this chunk.

## 1. One-paragraph verdict
This chunk is the tail of the NAL layer: the avcC counted-NAL reader, the shared Exp-Golomb BitReader, and the file's 300-line test module. `read_counted_nal` is small, correct, bounds-checked, and genuinely exercised by every shipped MP4/H.264 open (via the software decoder's avcC seeding — baseline sample-420.mp4 play session proves the chain end-to-end). The BitReader is careful work (typed truncation everywhere, correct §9.1/§9.2 Exp-Golomb arithmetic — hand-verified below, correct `finish()` trailing-bit verification) but is compiled-in dead weight in shipped builds: only the unreachable hardware path and the fuzz harness call it. The one real defect is `BitReader::more_rbsp_data`: a naive any-1-bit scan that counts the RBSP stop bit as data — spec-wrong, dead, untested, and explicitly flagged as wrong by the HEVC sibling's own doc comment, which says the naive scan "would count the stop bit as data — noted for the h264 sibling card's identical helper." The note was written; the fix was never applied.

## 2. What actually works (verified)
— `read_counted_nal`: executed in the shipped lane — baseline `mp4-smoke-open-session-420.txt` shows sample-420.mp4 (h264 CB 4:2:0, avcC extradata) reaching `Buffering → Playing`; the call chain pipeline_host.rs:346 → sw.rs:236-237 → AvcConfig::parse → read_counted_nal is code-traced. Bounds: `len == 0` refused, slice bounds checked before every index (read).
— Exp-Golomb arithmetic hand-verified against §9.1/§9.2: ue "1"→0, "010"→1, "011"→2 (matches test :951-955); se mapping k=0→0, 1→+1, 2→−1, 3→+2, 4→−2 — code `k.div_ceil(2)`, even k → negative (nal.rs:676-685) is exactly the spec table; ue prefix capped at 31 zeros → max value 2³²−2, no overflow (read + arithmetic).
— `finish()` mask arithmetic hand-verified: stop at bit_off 0..7, mask `(1<<(7-bit_off))-1` checks exactly the post-stop bits of the stop byte; `data[byte+1..]` non-zero scan catches junk tail bytes (nal.rs:724-737); empty-slice edge (stop bit = last bit of last byte) safe.
— All nal.rs tests `ok` in baseline run #3 (test-workspace.log.txt, codecs suite 643 tests / 0 fail per matrix evidence line).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/h264/parser/nal.rs:687-699
Code:
```rust
    /// Is there at least one more `1` bit in the unread data (the
    /// `more_rbsp_data()` predicate, §7.2)?
    #[must_use]
    pub fn more_rbsp_data(&self) -> bool {
        let mut i = self.bit_pos;
        while i < self.data.len() * 8 {
            if (self.data[i / 8] >> (7 - (i % 8))) & 1 == 1 {
                return true;
            }
            i += 1;
        }
        false
    }
```
What is wrong: this is the naive any-1-bit scan. Per §7.2, `more_rbsp_data()` must be FALSE when only `rbsp_trailing_bits()` remain — but the trailing bits always contain the `rbsp_stop_one_bit`, a `1`, so this scan returns TRUE for exactly-trailing-bits remainders. The HEVC sibling implements the correct semantics and documents this exact defect in the h264 helper's direction (crates/codecs/src/video/hevc/parser/nal.rs:830-836):
```rust
    /// Is there more RBSP data before the `rbsp_trailing_bits()`?
    /// True iff at least two `1` bits remain unread: the stop bit is
    /// the *last* `1` in the RBSP (§7.2), so a lone remaining `1` is
    /// exactly the trailing bits and not data. (This is stricter than
    /// a naive "any 1 bit" scan, which would count the stop bit as
    /// data — noted for the h264 sibling card's identical helper.)
```
Why it matters: the method is pub, spec-misnamed (`§7.2` in its own doc), and a landmine for the first future caller — e.g. an SPS VUI parse using it would treat every VUI-less SPS as VUI-bearing, read the stop bit as `vui_parameters_present_flag`, and misparse or reject the stream. It is also dead (`rg` shows zero callers) and untested (no test in 742-1046 touches it). Today's blast radius is zero because nothing calls it — hence S3, not S2.
What correct looks like: the HEVC twin's ones-count ≥ 2 scan (or locate the last 1 bit once and compare positions).
Confidence: HIGH — Verification: read + spec reasoning + HEVC cross-evidence + rg callers + rg test coverage.

### F-002 [S3]
Where: crates/codecs/src/video/h264/parser/nal.rs:598-740 (the BitReader subtree)
Code:
```rust
pub struct BitReader<'a> {
    data: &'a [u8],
    bit_pos: usize,
}
```
What is wrong: the chunk's principal export is unreachable from shipped entry points. Production callers of the h264 in-house syntax parsers that construct BitReaders (sps.rs/pps.rs/slice.rs/sei.rs) are only: h264/decode.rs:368 (`SeqParameterSet::parse` in `store_sps` — the unreachable hardware decoder, my decode.rs audit F-001), hw/vaapi/session.rs:96, and h264/fuzz.rs. The shipped software decoder (sw.rs) feeds NALs to rust_h264 and uses that engine's `parse_sps_info` (sw.rs:90) — it never runs the in-house SPS/PPS/slice parsers. `rg -n "h264::parser"` outside crates/codecs returns nothing.
Why it matters: ~140 lines of correct-but-unreachable parsing plus the whole sps/pps/slice/sei subtree they serve compile into every shipped binary; the parser's conformance corpus (matrix row evidence "corpus-verified 19/19") exercises code no user can reach, while the shipped decode path depends on the external engine's parser instead. Duplication of parsing truth across in-house and engine parsers is drift surface (the two must agree on damage accounting).
What correct looks like: either sw.rs consumes the in-house parser's outputs (the original "framing/EPB/damage accounting stay OURS" intent extended to parameter-set gating) or the unreachable subtree is feature-gated until the hw lane activates.
Confidence: HIGH — Verification: read + rg call graph + sw.rs imports (line 90 vs 93).

### F-003 [S4]
Where: crates/codecs/src/video/h264/parser/nal.rs:635-646
Code:
```rust
    pub fn bits(&mut self, n: u32) -> Result<u32, H264Error> {
        if n == 0 || n > 32 {
            return Err(H264Error::Invalid {
                code: "h264.bits.width",
            });
        }
        let mut acc: u64 = 0;
        for _ in 0..n {
            acc = (acc << 1) | u64::from(self.bit()?);
        }
```
What is wrong: `bits(n)` performs n separate bounds-checked `bit()` calls (up to 32) where a single bounds check plus a byte/shift accumulation would do — per-call overhead in the slice-header hot path (every slice header of every frame on the hw/fuzz lanes).
Why it matters: hygiene-level perf; unreachable today (F-002), so no user-visible cost.
What correct looks like: one `bit_pos + n <= total_bits` check, then accumulate from whole bytes.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/codecs/src/video/h264/parser/nal.rs:579-584
Code:
```rust
    if len == 0 || bytes.len() < *pos + len {
        return Err(H264Error::Truncated {
            needed: (*pos + len.max(1)) as u64,
            have: bytes.len() as u64,
        });
    }
```
What is wrong: for the `len == 0` violation the reported `needed` is `pos + 1` — the length-prefix byte position, not a meaningful "bytes the syntax required" (a NAL needs at least a header byte past the prefix, i.e. `pos + len_bytes + 1`). The diagnostic mixes two error classes (zero-length NAL vs. truncation) into one message.
Why it matters: error-diagnostic precision only; both paths are typed Truncated and callers treat them identically.
What correct looks like: separate `Invalid { "h264.avcc.zero_nal" }` for len == 0, keep Truncated for the real shortage.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part2: lines 562–1046)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `read_counted_nal` | 563-588 | REAL | S4 | shipped-reachable + baseline-executed (§2); bounds-checked; F-004 diagnostic |
| `BitReader` struct | 598-601 | REAL | S3 | F-002 unreachable in shipped builds |
| `BitReader::new` | 606-608 | REAL | — | |
| `total_bits` | 610-612 | REAL | — | u64 widening, no overflow |
| `bit` | 618-629 | REAL | — | bounds-checked, typed Truncated |
| `bits` | 635-646 | REAL | S4 | F-003 per-bit checks; n∈1..=32 validated |
| `ue` | 655-670 | REAL | — | §9.1 verified; 31-zero cap → max 2³²−2 |
| `se` | 676-685 | REAL | — | §9.2 mapping hand-verified (k=0→0,1→+1,2→−1,3→+2,4→−2) |
| `more_rbsp_data` | 687-699 | FALSE-semantics/UNREACHABLE | S3 | F-001: stop bit counted as data; dead; untested |
| `finish` | 708-739 | REAL | — | trailing-bit mask verified; empty-tail edge safe |
| tests: `nal_classification_covers_table_7_1` | 746-760 | REAL | — | roundtrip all 32 ids |
| tests: `header_split_checks_forbidden_bit_and_truncation` | 762-774 | REAL | — | |
| tests: `annexb_splits_three_and_four_byte_prefixes` | 776-787 | REAL | — | part1 subject |
| tests: `emulation_removal_and_typed_violations` | 789-812 | REAL | — | incl. buffer-reuse path |
| tests: `avc_length_prefixed_split_with_typed_truncation` | 814-855 | REAL | — | typed truncation, zero-pad vs junk tail |
| tests: `avcc_parse_roundtrip_and_error_paths` | 857-912 | REAL | — | covers my read_counted_nal error paths |
| tests: `avcc_accepts_ffmpeg_writer_reality` | 914-944 | REAL | — | real ffmpeg-written avcC byte shape (0003 corpus) |
| tests: `bitreader_exp_golomb_and_trailing_rules` | 946-986 | REAL | — | ue/se/bits/finish — but NO more_rbsp_data case (F-001 gap) |
| tests: `property_mutations_never_panic_or_hang` | 988-1044 | REAL | — | 3000-round xorshift mutations; termination+no-panic contract only (by design) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| bitreader_exp_golomb_and_trailing_rules | YES | pins exact ue/se/bits values and finish() verdicts incl. both error codes |
| avcc_parse_roundtrip_and_error_paths | YES | exact error variants + parsed fields; mutation of read_counted_nal bounds breaks the `cut.truncate(9)` case |
| avcc_accepts_ffmpeg_writer_reality | YES | real-world writer shape (bare PPS count byte, trailing extension bytes ignored) |
| property_mutations_never_panic_or_hang | PANIC/HANG class only | asserts termination and absence of panic; Ok-vs-typed-Err distinctions deliberately not asserted (documented contract) |
| all part1-subject tests (classification/annexb/emulation/avc) | YES | exact-value assertions |
Gap: zero test coverage for `more_rbsp_data` — the one spec-wrong method is also the one never tested (F-001).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "All reads are bounds-checked and return typed errors; the reader never panics and never reads past the buffer" | nal.rs :595-596 (BitReader docs) | REAL | read bit/bits/ue/se/finish; property test 3000 rounds no-panic |
| "the `more_rbsp_data()` predicate, §7.2" | nal.rs :687-688 | FALSE | forward any-1-bit scan counts the stop bit — violates §7.2 semantics (F-001) |
| "steady-state parse path allocates nothing" (RBSP reuse) | nal.rs :16-22 (module docs, part1) | REAL for this chunk | `read_counted_nal` borrows; BitReader borrows; test :807-811 pins buffer reuse |
| matrix row codec-video-h264 evidence "cards 5.F1a/5.F1b clean-room parser corpus-verified 19/19" | docs/feature-matrix.toml:415 | REAL-UNPROVEN for shipped reachability | corpus tests exist and pass (baseline codecs suite), but the parser's SPS/PPS/slice surface is unreachable in shipped builds (F-002); only the framing half (part1) is shipped-live via sw.rs |

## 7. Dead / duplicated / unreachable code
- `more_rbsp_data` — dead + spec-wrong (F-001).
- `BitReader` subtree — unreachable from shipped entry points (F-002); kept alive by hw path + fuzz harness.
- HEVC twin duplication: `BitReader`/`more_rbsp_data`/`finish` are re-implemented in crates/codecs/src/video/hevc/parser/nal.rs with the CORRECT more_rbsp_data — two copies of the same primitive that already disagree (one wrong, one right) is textbook drift.

## 8. Dependencies & flags
No external deps; std/core only. Not feature-gated — compiled unconditionally into the codecs crate (and thus shipped builds) despite the unreachable consumers (F-002).

## 9. Open questions for the next agent
- W06-A09 (nal.rs part1): `annexb_nals` (the Vec-collecting variant at :310) — who calls it? If only fuzz.rs, it is a second dead export in the same file.
- Whoever audits sw.rs: confirm whether the engine's `parse_sps_info` and the in-house `SeqParameterSet` can disagree on 4:4:4/10-bit detection — the shipped refusal (`h264.sw.unsupported`) rides the ENGINE's parse, while the docs cite the in-house parser's vocabulary checks.
- Fix owner for F-001: copy the HEVC ones-count implementation; add the missing test.
