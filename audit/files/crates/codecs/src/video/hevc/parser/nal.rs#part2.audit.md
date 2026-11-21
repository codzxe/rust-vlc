# AUDIT: crates/codecs/src/video/hevc/parser/nal.rs#part2
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 759–1005 audited (unique range; nominal 3-way split ≈506–1010, but W06-A05's part1 already covered 1–758 under its 2-way reading and W06-A06's part3 covered 1008–1517 — per A06's header "the test harness `Bits`/`corpus_path`/`splitmix64` at 893–1005 are part2's … not re-audited". I audit exactly the gap: `BitReader` read methods 759–891 + the shared test harness 893–1005) / 12 items |
| File verdict | UNREACHABLE (the production code in this range is transitively dead: no shipped entry point constructs `HevcDecoder` — established by W06-A05 F-001 and W06-A06 header, independently re-verified below) |
| Reachable from shipped entry point? | NO. My own verification: `rg -n "more_rbsp_data\|BitReader" crates/codecs/src/video/hevc` → the sibling parsers (sps.rs:214, pps.rs:121, vps.rs:331) use only `finish()`, never the dead helper; the hevc tree's sole `Decoder` impl (`HevcDecoder`, decode.rs) has zero non-test constructors (`rg "HevcDecoder" apps/` → no hits); baseline INDEX.md corrected matrix: `lumen open sample-hevc.mp4` → Opening → Error → Idle (no HEVC decoder). The tests in this file DID execute (baseline test-workspace.log.txt:632-641 — `video::hevc::parser::nal::tests` 10 ok). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `Read` nal.rs 440–1064 (range + boundaries both sides); `grep -n "^pub fn\|^impl\|^#\[cfg(test)\]"` structure map; `rg -n "more_rbsp_data" crates/codecs/src` (call sites: h264 sei.rs:224/302 private copy + hevc test at 1313 — ZERO hevc production callers); `rg -n "finish\(\)" hevc/parser/{sps,pps,vps,slice}.rs` (parameter-set parsers use finish only); `sed -n '632,641p'`-style greps of baseline test log (hevc nal tests 10 ok, hevc fuzz_lite ok); hand-verification of se/ue arithmetic (mapping table below); `sed -n` + earlier `od -c` on quoted ranges. |

## 1. One-paragraph verdict
My range is the `BitReader` half of the HEVC NAL file plus the shared test harness. The arithmetic primitives are correct where it matters: `bit`/`bits` are bounds-checked and MSB-first exact, `ue` cannot overflow (max return 2³²−2 in u64) and caps 31-zero prefixes, `se` implements the §9.2 mapping exactly (k=0→0, 1→1, 2→−1, 3→2, 4→−2 — hand-verified), and `finish()`'s stop-bit/mask arithmetic is right. But the range's one distinctive primitive is both dead and wrong: `more_rbsp_data` has zero production call sites in the HEVC tree, and its "at least two 1-bits" rule misclassifies an all-zero data tail (`0…01`) as "no more data", silently licensing parsers to drop trailing zero-valued syntax elements — a divergence from the canonical last-set-bit-position rule that the same repo's production-live H.264 SEI copy (sei.rs:302) inherits. The test harness (`Bits` writer, real x265 constants, splitmix64) is a faithful inverse — I hand-verified the ue/se encodings round-trip — but its corpus gate is wired to a different env var than the sibling corpus binary and defaults to a sandbox-only path, and A06 proved the corpus is absent in this environment.

## 2. What actually works (verified)
— Bounds safety, by hand: `bit()` (760-771) checks `bit_pos >= len*8` before every read (typed `Truncated`); `bits(n)` (778-789) rejects n=0/n>32 and accumulates in u64 so the `as u32` cast cannot truncate; `finish()` (859-890) re-checks the end inside its scan loop.
— `se()` mapping verified against §9.2 by enumeration: k=0→−0=0 ✓, k=1→+1 ✓, k=2→−1 ✓, k=3→+2 ✓, k=4→−2 ✓; magnitude ≤ 2³¹−1 for k ≤ 2³²−2, so the `as i64` and negation cannot overflow.
— `ue()` overflow analysis: leading-zero cap at 31 → value before decrement ∈ [2³¹, 2³²−1] → returns ≤ 2³²−2; u64 arithmetic cannot wrap. (This answers W06-A05's part1 open question: the clamping is SAFE — the doc's *justification* is wrong, see F-002.)
— The `Bits` test writer is a true inverse: `ue(v)` emits width−1 zeros + the width-bit encoding of v+1 (v=0→"1", v=5→"00110" ✓); `se` maps v≤0→−2v, v>0→2v−1, exactly inverting the reader; `nal()` builds the two-byte header `[(type<<1)&0x7e, 0x01]` — correct forbidden-0/type/layer/tid+1 geometry matching `NalHeader::split` (part1, A05-verified).
— Tests executed: baseline test-workspace.log.txt:632-641 — all 10 `video::hevc::parser::nal::tests` ok incl. the bitreader roundtrip at 1298-1313 (which exercises THIS range's `bit`/`bits`/`ue`/`se`/`more_rbsp_data`/`finish`) and fuzz_lite.

## 3. Findings

### F-001 [S3] `more_rbsp_data` is dead in the HEVC tree AND its "≥ 2 one-bits" rule silently drops all-zero data tails
Where: crates/codecs/src/video/hevc/parser/nal.rs:830-850
Code:
```
    /// Is there more RBSP data before the `rbsp_trailing_bits()`?
    /// True iff at least two `1` bits remain unread: the stop bit is
    /// the *last* `1` in the RBSP (§7.2), so a lone remaining `1` is
    /// exactly the trailing bits and not data. (This is stricter than
    /// a naive "any 1 bit" scan, which would count the stop bit as
    /// data — noted for the h264 sibling card's identical helper.)
    #[must_use]
    pub fn more_rbsp_data(&self) -> bool {
```
What is wrong: Two defects stacked. (1) Reachability: `rg` over the hevc tree — sps.rs/pps.rs/vps.rs call only `r.finish()`; slice.rs neither; the sole caller of `more_rbsp_data` is the unit test at 1313. It is exported `pub` dead code inside an already-unreachable module. (2) Semantics: the doc's rule ("a lone remaining 1 is exactly the trailing bits") is false when zero-valued data precedes the stop bit: for remaining bits `0…01`, the zeros ARE syntax elements (e.g. a trailing `u(1)` flag equal to 0), and the canonical predicate — "the position of the last set bit (the stop bit) is greater than the current position" — returns TRUE. This implementation counts one-bits, gets 1, and returns FALSE, telling the caller there is no more data while an unread data bit remains. The same rule is copied into the production-live H.264 SEI loop (crates/codecs/src/video/h264/parser/sei.rs:302, used at :224), where a legal `payloadType`/`payloadSize` byte of 0x00 before the stop bit would be silently dropped.
Why it matters: This is the shared "is there another syntax element" primitive of every HEVC parser the module claims to provide; a future consumer (the hw lane, the Phase-7 adapters) inherits a predicate that ends loops one element early on the most common shape (a final zero flag before trailing bits). It already mis-serves the h264 SEI parser in production.
What correct looks like: `more_rbsp_data = (index of last 1 bit in data) > bit_pos` — position comparison, not population count; delete or wire the helper.
Confidence: HIGH on deadness (rg); MEDIUM on the canonical-rule divergence (HM/ffmpeg position-based semantics recalled, spec text not consultable in-sandbox — the code's own doc admits it deviates from "any 1 bit", it just deviates from the position rule too).

### F-002 [S4] `ue`'s overflow-cap doc justification is false — the capped codes ARE u64-representable
Where: crates/codecs/src/video/hevc/parser/nal.rs:795-797
Code:
```
    /// `Invalid { "hevc.ue.overflow" }` for a prefix longer than 31
    /// zero bits (cannot encode a u64-representable field).
```
What is wrong: A 32-zero prefix followed by its 33-bit code encodes values in [2³², 2³³−2] — comfortably u64-representable. The cap at 31 is a range *policy* (fields in the sibling parsers are u32-bounded), not a representability limit; the doc states a false reason for a correct behavior.
Why it matters: Docs that give wrong reasons get "fixed" by well-meaning edits (e.g. someone raising the cap to 63 "since u64 can hold it") without checking the caller-side range checks the cap actually protects.
What correct looks like: "…for a prefix longer than 31 zero bits (HEVC syntax fields are u32-bounded; callers range-check, and the parser refuses absurd codes)."
Confidence: HIGH — Verification: read + arithmetic above.

### F-003 [S4] `finish()` accepts trailing zero bytes after the stop-bit byte for every RBSP — legal only for slice RBSPs (cabac_zero_words)
Where: crates/codecs/src/video/hevc/parser/nal.rs:884-889
Code:
```
        if self.data[byte + 1..].iter().any(|&b| b != 0) {
            return Err(HevcError::Invalid {
                code: "hevc.rbsp.tail",
            });
        }
        Ok(())
```
What is wrong: Per §7.3.2.11, `rbsp_trailing_bits()` ends the RBSP at the stop bit's byte; trailing zero *words* are only legal in `rbsp_slice_trailing_bits` (cabac_zero_words) — not in VPS/SPS/PPS/SEI RBSPs. This `finish` is the shared validator of all parameter-set parsers (sps.rs:214, pps.rs:121, vps.rs:331), and it accepts any all-zero tail, so a parameter-set NAL with arbitrary zero padding past the stop-bit byte passes as valid.
Why it matters: Lenient acceptance in the validator whose only job is strictness; on an unreachable path today, but it is the seam the hw lane would consume.
What correct looks like: A `finish_strict()` (no bytes after the stop-bit byte) for parameter sets, and a `finish_slice()` allowing zero words — or a flag parameter.
Confidence: MEDIUM — Verification: read; §7.3.2.11 cabac_zero_words semantics recalled, spec not consultable in-sandbox.

### F-004 [S4] Test-harness corpus gate: env-var name disagrees with the sibling corpus binary; default path is sandbox-only
Where: crates/codecs/src/video/hevc/parser/nal.rs:991-996
Code:
```
    pub(crate) fn corpus_path(name: &str) -> Option<std::path::PathBuf> {
        let dir = std::env::var("LUMEN_HEVC_CORPUS")
            .unwrap_or_else(|_| "/tmp/my-project/corpus-v4/media".to_string());
        let p = std::path::Path::new(&dir).join(name);
        if p.exists() { Some(p) } else { None }
    }
```
(with `lumen-allow-literal` annotations on both literals)
What is wrong: The integration corpus binary `tests/hevc_parser_corpus.rs` reads env `LUMEN_CORPUS` (per W06-A06's sed of its lines 35-41) while this in-module gate reads `LUMEN_HEVC_CORPUS` — one user exporting a corpus root runs one half of the corpus lanes and silently skips the other. The default `/tmp/my-project/corpus-v4/media` is a Lead-sandbox layout (same literal as mpeg12's corpus_dir), and A06 proved the corpus tree is absent in this environment — so these lanes skip in every recorded run.
Why it matters: Two env knobs for one corpus is exactly how "3/3 corpus lanes PASS" evidence (docs/formats/video-hevc.md) ends up unfalsifiable from the baseline.
What correct looks like: One shared `corpus_path` helper (single env var name) in testkit, used by both the in-module lanes and the integration binary.
Confidence: HIGH — Verification: read; A06's cross-examination of the sibling file and the absent corpus directory cited.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `BitReader::bit` | 756-771 | REAL (unreachable) | — | typed Truncated past end; MSB-first exact |
| `BitReader::bits` | 773-789 | REAL (unreachable) | — | n∈1..=32 gate; u64 accumulate, cast safe; per-bit loop (no word reads) — fine for header parsing |
| `BitReader::ue` | 791-813 | REAL (unreachable) | S4 | arithmetic safe (≤2³²−2); cap policy doc false → F-002 |
| `BitReader::se` | 815-828 | REAL (unreachable) | — | §9.2 mapping exact (enumerated); no overflow for k ≤ 2³² |
| `BitReader::more_rbsp_data` | 830-850 | REAL (unreachable) | S3 | zero production callers; ≥2-ones rule drops zero-data tails → F-001 |
| `BitReader::finish` | 852-890 | REAL (unreachable) | S4 | stop-bit scan + intra-byte mask verified; accepts zero tail bytes for all RBSPs → F-003 |
| test `Bits::new`/`put`/`flag` | 899-932 | REAL | — | assert-guarded writer; n≤64 & value range checked |
| test `Bits::ue`/`se` | 934-944 | REAL | — | exact inverse of the reader (hand-verified: v=0→"1", v=5→"00110") |
| test `Bits::finish`/`nal` | 946-969 | REAL | — | trailing-bits + two-byte NAL header geometry correct |
| test constants `VPS_0100`/`SPS_0100`/`PPS_0100` | 972-987 | REAL | — | real x265 bytes; provenance claim testable only with corpus present (absent — A06) |
| test `corpus_path` | 989-996 | REAL | S4 | env-var mismatch + sandbox default → F-004 |
| test `splitmix64` | 998-1005 | REAL | — | standard constants (0x9e3779b97f4a7c15, 0xbf58476d1ce4e5b9, 0x94d049bb133111eb) ✓ |

## 5. Test honesty (the tests *exercising* this range live at 1298-1340 — W06-A06's chunk; executed evidence cited)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| bitreader roundtrip (1298-1313, baseline ok) | YES | reads back bits(2)/bit/bits(5)/ue(0)/ue(5)/ue(255)/se(−3)/se(4) written by the independent `Bits` writer — any bit-order or ue/se defect fails the equality |
| more_rbsp_data assert at 1313 | WEAK | asserts only `!r.more_rbsp_data()` after consuming all data — pins the "exhausted" side, NOT the zero-data-tail misclassification (F-001); a mutation to `return true` on ones==1 still passes this test |
| fuzz_lite (baseline :781 ok) | YES for panics | 10k random + 5k mutated buffers through the parsers — panic-freedom proof, not semantic |
Mutation check (reasoned): flipping `ue`'s `value - 1` to `value` fails the roundtrip (ue(0)→1); flipping se's parity fails se(−3)/se(4); breaking `bit`'s bounds check panics → fuzz_lite fails. Breaking F-001's rule to the canonical position-based rule: NO test fails (no coverage of `0…01` tails) — that gap is why the bug is invisible to the suite.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "All reads are bounds-checked and return typed errors; the reader never panics and never reads past the buffer" | nal.rs:737-738 | REAL | verified by read of 756-890 + fuzz_lite executed |
| "True iff at least two 1 bits remain unread … a lone remaining 1 is exactly the trailing bits and not data" | nal.rs:831-833 | FALSE (for zero-data tails) | F-001 — the zeros before the stop bit are data |
| "> 31 zero bits (cannot encode a u64-representable field)" | nal.rs:796-797 | FALSE (justification) | F-002 |
| "shared primitive of every hevc::parser syntax reader (VPS/SPS/PPS and the slice-segment header)" | nal.rs:732-735 | REAL | imports in vps/sps/pps/slice confirmed — but `more_rbsp_data` itself is called by none of them |
| "(−1)^k+1 · ⌈k/2⌉ — mapped without overflow for k ≤ 2^32" | nal.rs:821 | REAL | enumerated k=0..4; magnitude bound verified |

## 7. Dead / duplicated / unreachable code
- `more_rbsp_data`: dead in this tree (F-001); duplicated rule in h264/parser/sei.rs:302 (production-live) — cross-file duplication of a flawed predicate.
- The whole range is transitively unreachable (module-level, A05 F-001 + my re-verification).

## 8. Dependencies & flags
- No feature gates; unconditional in-house module compiled into every shipped build (runs never — see reachability).
- W06-A05's part1 §8 answered the wave-6 hostile question (H.264 sw IS reachable: apps/cli/Cargo.toml `codecs` default-features → codecs/Cargo.toml `codec-h264-sw` default → h264/mod.rs:55,61 → apps/cli/src/pipeline_host.rs:346 `"h264" | "avc1" => … H264SwDecoder::new`; baseline sample-420.mp4 PLAYS). I independently verified the same chain plus the tauri lane (see my hevc/mod.rs audit §8): release.yml:219 builds apps/desktop-tauri (→ apps/desktop → lumen-cli → same provider) with no feature overrides. HEVC has no such lane — the asymmetry between the two modules is real.

## 9. Open questions for the next agent
- h264/parser/sei.rs owner: the production SEI loop inherits the ≥2-ones rule (F-001) — does any legal SEI shape (payloadType byte 0x00 + stop-bit byte) hit the dropped-data path in practice? The corpus lanes that would answer this skip in this sandbox (A06 F-001).
- hevc/slice.rs owner (if ever wired): slice RBSPs are the one legal consumer of zero-word tails (F-003) — split `finish` before wiring.
