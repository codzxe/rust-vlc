# AUDIT: crates/codecs/src/video/mpeg12/bitreader.rs
| Field | Value |
|---|---|
| Auditor | W06-A08 |
| Date | 2026-11-21 |
| Lines / functions | 167 / 5 methods (new, remaining_bits, read_bits, read_bit, read_marker, peek_bits) + 6 tests |
| File verdict | UNREACHABLE (real, correct, test-executed reader; the whole mpeg12 tree has no production consumer) |
| Reachable from shipped entry point? | NO. Repo census: `rg "mpeg12\|Mpeg12"` production sources (crates/ + apps/, excl. tests/comments) → ZERO consumers outside the module itself (module-internal chain: facts.rs `Mpeg12Facts::scan` → headers.rs parsers (:159, :283, :379, :446, :532, :651) → this reader; tests/mpeg12_corpus.rs + in-crate tests only). The CLI has no mpeg arm (pipeline_host.rs dispatch census: mjpeg/av1/vp9/dnxhd/h264 only). Cross-ref: the mpeg12 module-root audit (mod.rs.audit.md) established the same production-dead verdict with the full census. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Read tool full file (verbatim authority); `rg -n "peek_bits\|read_bits\(\|read_marker\(\|BitReader::new"` over the mpeg12 module (consumer + call-shape census); `rg -n "peek_bits" crates/codecs/tests/mpeg12_corpus.rs` → no match (dead-code proof); `rg -n "mpeg12"` baseline test log (:742-775 module tests, :1183-1187 corpus 3/3 ok); `ls fuzz/fuzz_targets/ | wc -l` → 35 (no mpeg2 target) + `rg fuzz_codec_mpeg2` repo-wide (worklog "deferred to admission", docs/formats "target arrives only with a decoder"); hand-verification of remaining_bits arithmetic (5-of-8 → 3; 13-of-16 → 3); Read h264 nal.rs:635-646 (the twin fixed-width loop); `sed -n` quote verification bitreader.rs:1-9,30-34,42-62,76-89 |

## 1. One-paragraph verdict
A small, correct, honestly-tested MSB-first header reader: the fixed-width read is exactly the `uimsbf`/`bslbf` convention §6.2 demands, width and truncation are typed gates with exact-`Err` test pins, the partial-byte `remaining_bits` arithmetic checks out by hand, and all six unit tests plus the three real-bytes corpus lanes (ffmpeg-shaped sequence headers, TS/PS transports, ffprobe-fact lanes) ran green in the baseline. It is also a third copy of the same MSB loop the h264 and hevc readers already contain, behind a "sibling not share" rationale that only holds for the Exp-Golomb half; it carries a `peek_bits` method whose only caller in the entire repository is its own unit test while its doc describes a dispatch-steering usage contract nobody honors; and its no-panic guarantee is attributed to a `fuzz_codec_mpeg2` target that does not exist in the 35-target fuzz registry. All of it unreachable: the mpeg12 tree parses for consumers (probe facts, hw-feed) that were never built.

## 2. What actually works (verified)
— `read_bits` semantics hand-verified against the uimsbf/bslbf convention (MSB-first, 1..=32, typed `Invalid{"mpeg12.bits.width"}` for 0/>32, typed `Truncated{needed, have}`): every property pinned by an exact-`Err` or exact-value test.
— `remaining_bits` arithmetic re-derived by hand for partial-byte positions (bit_pos 5 of 8 → 3; bit_pos 13 of 16 → 3; boundary 8-of-8 → 0) — matches the implementation's `(len - pos/8)*8 - pos%8` with saturating guards; pinned by `remaining_counts_partial_bytes` (24 → 19 → 0).
— Baseline execution, all ok: `video::mpeg12::bitreader::tests::{reads_msb_first, truncation_is_typed, marker_bit_enforced, width_bounds_rejected, remaining_counts_partial_bytes, empty_input_only_truncates}` (test-workspace.log.txt:742-747); downstream `video::mpeg12::headers::tests::*` 10 tests (:758-774) and `video::mpeg12::facts::tests::fuzz_lite_scan_is_total` (:754) drive this reader over real and hostile shapes; `mpeg12_corpus` 3/3 ok (:1185-1187 — ffmpeg-shaped headers, TS/PS fixture transports, ffprobe-fact lanes).
— No panic paths: every method is bounds-checked or delegating; `peek_bits` swallows its error into 0 by documented design; empty input is pinned (`empty_input_only_truncates`).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/mpeg12/bitreader.rs:83-89
Code:
```
    /// Peek up to `n` bits (1..=32) without consuming (missing bits
    /// read as 0 — peeks only steer dispatch, never assert facts).
    #[must_use]
    pub fn peek_bits(&self, n: u32) -> u32 {
        let mut clone = self.clone();
        clone.read_bits(n).unwrap_or(0)
    }
```
What is wrong: `peek_bits` has zero production callers. `rg "peek_bits"` over the whole repository: the definition, its doc, and one call in this file's own test (:165, `assert_eq!(r.peek_bits(8), 0)`). headers.rs — the only consumer of this reader — never peeks (all reads are consuming `read_bits`/`read_bit`/`read_marker`); the corpus test has no match. The doc prescribes a usage contract ("peeks only steer dispatch, never assert facts") that no caller obeys, because no caller exists.
Why it matters: dead public API whose doc implies an in-module dispatch pattern (start-code lookahead, extension-id steering) that the module never implemented; a future caller following the doc's "missing bits read as 0" rule can silently mis-steer on truncated data — the exact hazard the doc's caveat exists to fence, untested because unused.
What correct looks like: either headers' extension/slice dispatch uses it (and a test pins the steer-vs-assert rule), or the method moves to the test module it serves.
Confidence: HIGH — Verification: rg census (module + corpus test) + read.

### F-002 [S4]
Where: crates/codecs/src/video/mpeg12/bitreader.rs:8-9
Code:
```
//! unbounded work on any input (the `fuzz_codec_mpeg2` target's
//! future guarantee, doc 04 §8.5).
```
What is wrong: the named target does not exist: the fuzz registry's 35 targets (fuzz/fuzz_targets/, all four registry sets in sync per baseline run #18) contain no mpeg2 target; the worklog records it as "deferred to admission" and docs/formats/video-mpeg12.md states the target "arrives only with a decoder". Citing doc 04 §8.5 as the backer of this guarantee points at a registry row that is not there. (The module root's harder variant — mod.rs:53-54 calling the same target "the registered" — is already counted as F-002 in mod.rs.audit.md; this file's softer "future guarantee" phrasing rides the same phantom and is recorded here, not re-counted there.)
Why it matters: the no-panic guarantee is currently backed only by unit tests and facts' in-tree fuzz-lite; attributing it to a named fuzz target borrows credibility from a target that was never written.
What correct looks like: "pinned in-tree by the bitreader unit tests and facts' fuzz-lite sweeps; a dedicated fuzz target arrives with the decoder (docs/formats/video-mpeg12.md)".
Confidence: HIGH — Verification: ls fuzz_targets (35, no mpeg2) + rg worklog/docs + read.

### F-003 [S4]
Where: crates/codecs/src/video/mpeg12/bitreader.rs:1-4 vs h264/parser/nal.rs:635-646
Code:
```
//! 13818-2 §6.2: every `uimsbf`/`simsbf`/`bslbf` element is
//! read most-significant bit first — the opposite convention from
//! H.264's Exp-Golomb reader, hence a sibling rather than a share).
```
What is wrong: the stated reason for not sharing holds only for the Exp-Golomb half. The fixed-width read — this file's `read_bits` (42-62) — is semantically and structurally the same MSB-first `acc = (acc << 1) | bit` loop as h264's `BitReader::bits` (nal.rs:635-646, same 1..=32 width gate, same Invalid-class code, one error-string prefix apart) and the hevc twin: a third copy of the same loop, not a different convention.
Why it matters: the "sibling rather than a share" rationale reads as a spec-forced divergence; the actually-divergent part (Exp-Golomb) could not be shared, the duplicated part (u(n)) could. A bounds fix to one loop (e.g. the u32 cap, the width gate) will land in two of three copies.
What correct looks like: share the fixed-width core (a tiny `read_msb(n)` helper both codecs' readers call), or state the truth: "duplicated deliberately for module independence; the fixed-width halves are identical, the golomb halves are not".
Confidence: HIGH — Verification: read (both loops, side by side).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `BitReader::new` | 24-26 | REAL (executed) | — | trivial cursor init |
| `remaining_bits` | 30-34 | REAL (executed) | — | partial-byte arithmetic hand-verified; saturating chains make negative/overflow unreachable; u32 cap defensive for >512MiB (irrelevant for headers); test-pinned |
| `read_bits` | 42-62 | REAL (executed) | — | width gate (0/>32 → Invalid), truncation gate with exact needed/have, MSB-first loop; bit-by-bit (fine: headers are tens of bytes, no entropy path); exact-Err tests |
| `read_bit` | 68-70 | REAL (executed) | — | delegate |
| `read_marker` | 76-81 | REAL (executed) | — | spec marker enforcement, per-site `code` context; exact-Err test; consumed at every §6.2 marker site in headers.rs (:179, :308, :383) |
| `peek_bits` | 86-89 | REAL (test-only) | S3 | zero production callers; phantom usage contract (F-001) |
| tests ×6 | 97-166 | REAL | — | mutation-sensitive, exact `Err` equality |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| reads_msb_first (:98-104) | YES | LSB-first mutation flips `0b1011`; partial-consumption sequence pins cursor advance |
| truncation_is_typed (:107-119) | YES | exact `Err(Truncated{needed, have})` equality on both the empty tail and the 9-of-8 case — wrong `have` math fails |
| marker_bit_enforced (:122-132) | YES | exact `Err(Marker{code})` + ok-case |
| width_bounds_rejected (:135-149) | YES | 0 and 33 must be `Invalid` |
| remaining_counts_partial_bytes (:152-159) | YES | 24 → 19 → 0 pins the partial-byte formula |
| empty_input_only_truncates (:162-166) | YES | empty slice must truncate, peek must 0 |
| External: headers tests (:758-774), facts fuzz-lite (:754), mpeg12_corpus 3/3 (:1185-1187) | YES (for the shapes those lanes carry) | real ffmpeg-shaped sequence/GOP/picture/slice syntax drives this reader; corpus lanes compare against ffprobe-derived facts |
| Gap: nothing tests `peek_bits` steering semantics on truncated data | — | because no caller exists (F-001) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every `uimsbf`/`simsbf`/`bslbf` element is read most-significant bit first" | bitreader.rs:1-4 | REAL | read loop verified; `reads_msb_first` |
| "every read is either the requested bits or a typed `Mpeg12Error::Truncated`" | bitreader.rs:6-7 | REAL-with-omission | Truncated ✓, but `read_bits` also returns `Invalid{"mpeg12.bits.width"}` and `read_marker` returns `Marker` — the method-level `# Errors` docs state all three; the module sentence undercounts its own error vocabulary |
| "no panic, no unbounded work on any input" | bitreader.rs:7-8 | REAL (inspection + tests) | all paths bounds-checked; empty/garbage/fuzz-lite executed |
| "the `fuzz_codec_mpeg2` target's future guarantee, doc 04 §8.5" | bitreader.rs:8-9 | PAPER-ONLY (the target) | not in the 35-target registry; worklog "deferred to admission" (F-002) |
| "the opposite convention from H.264's Exp-Golomb reader, hence a sibling rather than a share" | bitreader.rs:3-4 | SHALLOW (as a rationale) | true for ue/se, false for the fixed-width half that got triplicated (F-003) |

## 7. Dead / duplicated / unreachable code
- Whole file UNREACHABLE from shipped entry points (module production-dead — header census; mod.rs.audit.md concurs).
- `peek_bits`: dead public method (F-001).
- Fixed-width MSB loop: third in-tree copy (h264 nal.rs `bits`, hevc nal.rs twin, this file) — F-003.

## 8. Dependencies & flags
None beyond `super::error::Mpeg12Error`. Module unconditional (`pub mod bitreader;` mod.rs:56; in-house ungated policy — see my video/mod.rs audit F-001 for the dead-weight consequence: this tree compiles into every native shipped CLI build with no consumer).

## 9. Open questions for the next agent
- If the Phase-6+ self-build or a Phase-7 hw adapter ever lands: `peek_bits`' steer-vs-assert contract (F-001) needs a caller and a test, or deletion.
- Whoever owns the fuzz registry: the mpeg12 story (no decoder → no target) is coherent, but two doc sites (mod.rs:53-54 counted; bitreader.rs:8-9 here) still gesture at the target — one worklog-facing correction would close both.
