# AUDIT: crates/codecs/src/video/mpeg12/facts.rs
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Lines / functions | 546 / 17 (11 prod fns/methods + BitWriter, 5 tests, 1 helper) |
| File verdict | REAL (scan logic verified by execution) — UNREACHABLE from shipped entry points; module doc claims consumers that do not exist |
| Reachable from shipped entry point? | NO — `Mpeg12Facts::scan` / `descriptor_gate` / `decode_refusal` have zero production callers; only crates/codecs/tests/mpeg12_corpus.rs uses them. Demuxers classify mpeg1/2 with their own inline probes (demux/src/ps/mod.rs:764-787, ts/mod.rs:351-352); media-probe has no mpeg12 reference at all |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/codecs-bae0f1756cc0a809 video::mpeg12::facts` -> 6 passed / 0 failed, 0.18s; `./target/debug/deps/mpeg12_corpus-e7a93b397405d06f` -> 3 passed / 0 failed, 1.12s (includes live-ffprobe cross-check of this scan on 4 fixtures + 6 corpus lanes); rg for production callers -> none |

## 1. One-paragraph verdict
The scan itself is real, careful, and verified: a total one-pass start-code walk (first-header-wins ffprobe identity, damage counted not fatal, mid-stream sequence changes flagged), correct 12+2-bit dimension extension math, extension-refined frame rate, and a 6-test unit suite plus the mpeg12_corpus differential suite that cross-checks it against live ffprobe on the repo fixtures — I ran both, all green. The problem is the file's reason to exist: nothing shipped consumes it. The module doc says the scan feeds "probe/track-info" (facts.rs:3) and mod.rs:40-42 says "probe/track-info consume" it — no such consumer exists; media-probe implements its own ES probing with zero mpeg12 imports, the demuxers have their own inline MPEG-1/2 classification peeks, and the "codec-api gate" (`descriptor_gate`/`decode_refusal`) is called only from tests (apps/cli/src/containers/es.rs has its own unrelated local `decode_refusal`). Secondary: `slices_with_mb_address` silently caps at ~65 parsed slices while its doc presents it as a census, and the corpus row printer then reports the cap (64) as if it were the stream's count.

## 2. What actually works (verified)
— `./target/debug/deps/codecs-bae0f1756cc0a809 video::mpeg12::facts` -> 6/6 pass (0.18s): tiny-stream exact facts, damage-tolerant walk, total-on-garbage, MPEG-1 classification, sequence-change flag, 8000-round fuzz-lite invariants.
— `./target/debug/deps/mpeg12_corpus-e7a93b397405d06f` -> 3/3 pass (1.12s): `fixture_raw_es_facts_match_ffprobe` runs THIS scan on 4 in-repo fixtures and compares profile/level/chroma/rate/interlace/dimensions against live ffprobe (row output verified earlier, e.g. mpeg2-main-ipb.m2v: PASS 320x240 Main/8 yuv420p 25/1, 25 pics, 375 slices).
— Dimension math verified: `seq.horizontal_size_value + (ext.horizontal_size_extension << 12)` (the shift is inside the closure, L167-176) — correct §6.2.2.1 extension semantics; frame-rate delegation to `frame_rate_with_ext` handles the 30000/1001 NTSC case (mpeg1 test asserts exactly that).

## 3. Findings
F-001 [S3]
Where: facts.rs:1-6 (module doc) and crates/codecs/src/video/mpeg12/mod.rs:40-42
Code:
//! MPEG-1/2 elementary-stream buffer start code by start code and
//! collect the sequence-layer facts probe/track-info report —
What is wrong: the doc claims the facts feed probe/track-info. Repo-wide rg for `Mpeg12Facts` outside mpeg12 itself finds ONLY `crates/codecs/tests/mpeg12_corpus.rs`. crates/media-probe has no mpeg12 import (its ES signature module handles mpeg *audio* only); the demuxers classify MPEG-1/2 video with their own inline probes (demux/src/ps/mod.rs:764-787 peeks 64 bytes for `00 00 01 B5`; ts/mod.rs:351-352 hardcodes stream-type ids). The claim of a consumer is false in the current tree. Why it matters: 546 lines of real parser are dead shipped code dressed as an active pipeline stage; a reader auditing "what does track-info report for an .m2v" would be misled. What correct looks like: either wire it (media-probe or the demux host) or state "reserved for the demux/track-info card; test-verified only". Confidence: HIGH — Verification: rg across crates/ and apps/.
F-002 [S4]
Where: facts.rs:59-61 (doc) vs 139-153 (cap)
Code:
    /// Slice headers whose first macroblock VLC matched a modeled
    /// code (macroblock-addressing evidence).
    pub slices_with_mb_address: u32,
...
                StartCode::Slice(n) => {
                    facts.slice_count += 1;
                    if facts.slice.is_none() || facts.slice_count <= 64 {
What is wrong: slices are parsed only while `slice_count <= 64` (plus the first), so `slices_with_mb_address` is capped near 65 regardless of stream length; the doc presents it as an unqualified census. On the 375-slice mpeg2-main-ipb.m2v fixture the corpus row prints `mb_addr=64` — the cap, not the count. Why it matters: the metric is silently saturated; anyone comparing it against a real slice census gets a wrong number with no indication. What correct looks like: document the cap in the field doc (or count matches without full parses). Confidence: HIGH — Verification: read + observed printed row (mb_addr=64 on a 375-slice stream).
F-003 [S4]
Where: facts.rs:257-262
Code:
    /// The typed decode refusal for this stream's descriptor (the
    /// codec-api gate; see [`decode_refusal`]).
    #[must_use]
    pub fn decode_refusal(&self) -> media_types::error::EngineError {
        decode_refusal(self.descriptor())
What is wrong: described as "the codec-api gate", but no codec-api or app code calls it — the CLI's mpeg2video tracks get the generic provider refusal (apps/cli/src/pipeline_host.rs:376 landed-list error), and apps/cli/src/containers/es.rs:655 defines its own unrelated local `decode_refusal`. Unused-in-production helper presented as wiring. Confidence: HIGH — Verification: rg (`decode_refusal` -> mpeg12 error.rs def, mpeg12 tests, es.rs local fn only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Mpeg12Facts::scan | 78-160 | REAL | — | total one-pass walk; damage counted; first-wins; ran green on fixtures + ffprobe cross-check |
| dimensions | 165-178 | REAL | — | 12+2-bit extension math correct |
| frame_rate / frame_rate_string | 182-197 | REAL | — | NTSC 30000/1001 asserted by test |
| is_mpeg2 / profile / level / chroma / interlaced | 202-238 | REAL | — | sequence_ext-gated, ffprobe-parity verified live |
| has_motion_vectors | 242-244 | REAL | — | mask & 0b0110 (P/B) |
| descriptor | 249-255 | REAL | — | mpeg1/mpeg2 split matches demux vocabulary |
| decode_refusal (method) | 260-262 | REAL, test-only | S4/F-003 | no production caller |
| is_video_stream | 267-269 | REAL | — | sequence presence |
| kind_as_index | 273-281 | REAL | — | I/P/B/D → 0..3 |
| descriptor_gate | 287-289 | REAL, test-only | — | thin re-export of accepts |
| tests: BitWriter | 356-386 | REAL | — | MSB-first writer, flush correct |
| tests: tiny_mpeg2_stream | 298-354 | REAL | — | hand-built conforming stream (verified by ffprobe-parity suite usage) |
| tests: 5 #[test] fns | 388-545 | REAL (ran) | — | exact facts, damage, totality, mpeg1, seq-change, 8000-round fuzz-lite |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| tiny_stream_facts | YES (ran) | ~15 exact fact assertions on a hand-built stream |
| damage_is_counted_not_fatal | YES (ran) | corrupt seq header → walk continues, bad_headers ≥ 1 |
| empty_and_garbage_inputs_are_total | YES (ran) | start_codes == 0 on empty/garbage/zeros/stub |
| mpeg1_stream_classifies_as_mpeg1 | YES (ran) | exact dimensions + 30000/1001 + profile None |
| sequence_change_is_flagged | YES (ran) | count 2, changes true, first-wins dimensions |
| fuzz_lite_scan_is_total | YES (ran) | 8000 rounds, monotone counters, bounded mask |
| (corpus) fixture_raw_es_facts_match_ffprobe | YES (ran) | live ffprobe parity on 4 fixtures — strongest evidence |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "facts probe/track-info report" (consumer exists) | facts.rs:2-4, mod.rs:40-42 | FALSE | rg: no production caller; media-probe/demux use own probes (F-001) |
| "Bounded and total: one pass, no allocation beyond the fact struct, no panic on any input" | facts.rs:74-76 | REAL | fuzz-lite 8000 rounds passed; walk is single-pass with no heap use |
| "the codec-api gate" for decode_refusal | facts.rs:257-258 | FALSE (unused in prod) | rg: only mpeg12 tests + unrelated es.rs local fn (F-003) |
| docs/formats/video-mpeg12.md "probe/track facts still flow" | docs/formats/video-mpeg12.md:50-53 | UNVERIFIED for shipped probe | facts exist and are correct, but no shipped probe surface consumes them in-tree |

## 7. Dead / duplicated / unreachable code
- Whole module is production-unreachable (test-only consumers) — see F-001.
- Duplication: demux/src/ps/mod.rs:764-787 implements a parallel MPEG-1/2 classification (64-byte B5 peek) with DIFFERENT semantics than this scan (full-walk extension detection); ts/mod.rs:351-352 hardcodes ids. Two classification truths for the same question.

## 8. Dependencies & flags
In-workspace only (super::error/headers/startcode). No features, no unsafe. Test corpus knob LUMEN_CORPUS affects only mpeg12_corpus lanes, not this file's own tests.

## 9. Open questions for the next agent
- mpeg12/headers.rs + startcode.rs audit: the slice-header VLC "modeled code" table (Table B-1 subset) — how many of the 11 macroblock_address_increment codes are modeled, and does the ≤64-parse cap interact with `slice.is_none()` when all parses fail (first-parse-failure keeps `slice` None → every slice parsed → bad_headers grows unbounded relative to cap; confirm intended).
