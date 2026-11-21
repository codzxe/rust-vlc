# AUDIT: crates/codecs/src/video/mjpeg/huffman.rs
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2026-11-21 |
| Lines / functions | 463 / 17 items in table (enum Stop, struct, new, rebuild, decode + 9 tests + naive-oracle impl + 3 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open <avi>` (the first vertical) → apps/cli/src/pipeline_host.rs mjpeg arm → `codecs::video::mjpeg` decoder → `decoder.rs:94 use super::huffman::{HuffmanDecoder, Stop}` → `refresh_table` (decoder.rs:822-835: `Some(table) => table.rebuild(bits, vals)`, `None => HuffmanDecoder::new`) → `decode` per symbol in the scan decoder (decoder.rs:986-987 dc/ac table params). EXECUTED in baseline: AVI MJPEG+PCM fixture `v_frames=60 v_decoded=60` (INDEX.md MP4 SMOKE contrast + rc-smoke 6/6 engine checks), all 9 huffman unit tests ok (test-workspace.log.txt:700-708), mjpeg_field_differential 3/3 ok (:1122-1126). |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:2 |
| Commands run | full-file read; `rg -n "HuffmanDecoder\|rebuild("` over mjpeg/ (callers decoder.rs:94,385-386,830-832,986-987); sed of bitreader.rs:117-177 (peek_bits/peek8_padded/available/consume_all contracts — proved the `unwrap_or(0)` fallback dead); sed of parser.rs:463-523 (DHT parse: `total > 256` gate at :488-490 + duplicated oversubscription check — upstream bound for this file's `values`); hand-traced the canonical-code construction and all 9 tests' expected vectors (bit-by-bit); baseline log sed :700-708. |

## 1. One-paragraph verdict
This is the shipped MJPEG lane's Huffman core, and it is the best-engineered file I audited this wave: a total (panic-free) canonical-table builder with checked arithmetic and a spec-faithful oversubscription gate, an 8-bit look-ahead LUT whose padding can never fabricate a decode (the length-vs-available check), the classic MINCODE/MAXCODE walk for 9..=16-bit codes, and a test suite that keeps the pre-optimization bit-at-a-time decode as a live oracle and crosses 600 random tables × random streams (including 0xff stuffing stress and truncated tails) demanding symbol-exact equivalence — the exact harness that kills LUT-refactor bugs. Truncation and invalid-code semantics are typed (`Stop`) and pinned by dedicated tests; the whole thing is exercised end-to-end by the baseline AVI 60/60 decode. The only findings are hygiene: a dead `unwrap_or(0)` fallback on a `peek_bits` call that cannot return None in context (a silent fabricated code 0 if a refactor ever breaks the invariant — deserves a debug_assert, not a swallow), and the reuse of the decode-time `Stop::InvalidCode` variant for build-time structural rejection (documented, but conflates two failure classes in the type). No S0–S3: no panic path, no allocation in decode, no misdecode found in any traced case.

## 2. What actually works (verified)
- The full shipped decode path — baseline `decode-bench` on `tests/fixtures/generated/valid/avi_mjpeg_pcm.avi`: `v_frames=60 v_decoded=60` (INDEX.md MP4 SMOKE contrast section); rc-smoke engine-only 6/6 PASS (INDEX.md row 20).
- All 9 unit tests ok in baseline — test-workspace.log.txt:700-708 — including the 600-round LUT-vs-bit-walk equivalence sweep and the mid-code-truncation padding test.
- Canonical build (rebuild :88-113): `checked_add` overflow guard (:101), `last >= (1 << length)` oversubscription gate (:102-104), BITS/HUFFVAL count agreement (:111-113) — hand-traced against Annex F.2.2.3/C.2; the LUT ranges (:119-132) are in-bounds by construction (codes < 2^length ⇒ start+width ≤ 256), and `values` indexing is guarded by the :111 count check that runs BEFORE the LUT loop.
- LUT semantics: `peek8_padded` + `available >= length` gate (:155-163) — the zero-padding can never produce a symbol; pinned by `truncation_mid_code_is_exhausted_not_fabricated` (:423-443), which asserts both the Exhausted verdict AND that the partial bits are consumed.
- Slow path (:177-203): the `available < 8 ⇒ Exhausted` gate (:169-172) is provably equivalent to the bit-walk (a ≤8-bit match would have hit the LUT; anything longer needs ≥9 bits) — verified by hand-trace of the toy table and the 16-bit-code test.
- Bit contracts cross-checked: `peek_bits` returns None only when `nbits < n` after refill (bitreader.rs:134-139), and `nbits` cannot change between the `available` capture and the peek (no consumption in the loop; refill is idempotent once >24 bits) — the slow path's guard ordering is sound.

## 3. Findings

### F-001 [S4]
Where: crates/codecs/src/video/mjpeg/huffman.rs:183
Code:
```
            let code = reader.peek_bits(length_u32).unwrap_or(0) as i32;
```
What is wrong: `peek_bits` returns `None` only when fewer than `n` bits remain (bitreader.rs:131-139), and the caller has just verified `available >= length` at :179-182 with no intervening consumption — the `None` branch is unreachable today, but the fallback fabricates code `0` (a valid-looking canonical code) instead of surfacing the broken invariant.
Why it matters: this is the SHIPPED MJPEG path. Any future refactor that lets `available` go stale (e.g. moving a `consume` into the loop, or changing refill semantics) turns this dead branch into a silent wrong-decode of whatever symbol sits at the head of the length-`l` code range — precisely the class of bug the equivalence test exists to catch, but only if the random tables happen to stress the refactored shape. A swallowed impossible-error should be loud.
What correct looks like: `let Some(code) = reader.peek_bits(length_u32) else { return Err(Stop::Exhausted) };` — the honest answer if the invariant ever breaks is exhaustion, not code 0; or a `debug_assert!(reader.available() >= length_u32)` before the peek documenting the invariant.
Confidence: HIGH — Verification: read (bitreader.rs:134-139 contract sed-verified; loop traced — no consumption between guard and peek).

### F-002 [S4]
Where: crates/codecs/src/video/mjpeg/huffman.rs:58-64
Code:
```
    /// Build from DHT fields.
    ///
    /// # Errors
    /// [`Stop::InvalidCode`] semantics at build time: the table is
    /// structurally unusable (over-subscribed code space, or BITS/HUFFVAL
    /// disagreeing) — surfaced as `Err` so the caller can reject the
```
What is wrong: build-time structural rejection reuses the decode-time `Stop::InvalidCode` variant — the enum's own doc (:32-34) defines InvalidCode as "the bits read form a code absent from the table", which is not what a BITS/HUFFVAL disagreement is. Two distinct failure classes (bad stream bytes vs bad table) collapse into one variant; the doc comment admits the conflation rather than fixing it.
Why it matters: a caller (the scan decoder's concealment logic) cannot distinguish "damage inside the entropy data, conceal and continue" from "the table itself is garbage, reject the frame" by type — it must rely on which call site returned the error. Blast radius small (the mjpeg decoder does distinguish by call site), but it is the project's own typed-error discipline applied sloppily at a seam.
What correct looks like: a separate `BuildError`/`Stop::BadTable` variant (or a distinct error type for `new`/`rebuild`), keeping `Stop` for the entropy-decode stops the doc actually describes.
Confidence: HIGH — Verification: read (both definitions and both production call sites — decoder.rs:830-832 propagates the same `Stop`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum Stop` | 28-35 | REAL | — | honest two-case stop vocabulary; F-002 conflation at build sites |
| `struct HuffmanDecoder` | 39-55 | REAL | — | MINCODE/MAXCODE/VALPTR arrays + LUT; no allocation after build |
| `HuffmanDecoder::new` | 65-75 | REAL | — | delegates to rebuild; fresh-slot allocation only |
| `HuffmanDecoder::rebuild` | 88-139 | REAL | — | total build: checked_add :101, oversubscription :102, count agreement :111; LUT :118-132 in-bounds by construction; V-3 in-place HUFFVAL reuse :136-137 (capacity bounded — parser.rs:488-490 gates total ≤ 256 upstream); accepts `values.len()` > 256 itself (upstream-bounded, struct doc :81 claims the limit — harmless second-layer) |
| `HuffmanDecoder::decode` | 146-204 | REAL | S4 | fast path padding-honest (:158-163); `available < 8` gate equivalent-to-walk (traced); slow walk spec-faithful; F-001 `unwrap_or(0)`; zero allocation, O(1) per symbol |
| `tests::reader_of`/`toy_table` | 211-226 | REAL(test) | — | minimal 2-code table incl. legal incomplete space |
| `decodes_each_present_code` | 228-241 | REAL(test) | — | bit-by-bit hand-verified (0x40 → 0x05, 0x11, 5×0x05, then Exhausted) |
| `code_absent_from_table_is_an_error` | 243-251 | REAL(test) | — | 16-bit invalid walk → InvalidCode, consume-16 semantics pinned |
| `oversubscribed_table_is_rejected_at_build` | 253-261 | REAL(test) | — | negative build gate |
| `code_overflow_past_sixteen_bits_is_rejected` | 263-272 | REAL(test) | — | 255+3 overflow shape — the exact arithmetic the checked_add guards |
| `single_symbol_table_uses_a_one_bit_code` | 274-289 | REAL(test) | — | skewed-table shape + all-ones → InvalidCode (hand-traced) |
| `sixteen_bit_codes_decode` | 291-312 | REAL(test) | — | 1-code-per-length 1..=16 table, length-16 code through stuffing — slow-path end-to-end (hand-traced) |
| `decode_naive` (oracle) | 316-344 | REAL(test) | — | pre-LUT walk kept verbatim as equivalence oracle — exemplary |
| `lut_decode_matches_the_bit_walk_on_random_streams` | 346-420 | REAL(test) | — | 600 rounds, random tables (incl. rejected over-subscribed), 0xff stuffing stress, truncated tails; symbol + terminal-Stop equivalence — the mutation killer for any LUT refactor |
| `truncation_mid_code_is_exhausted_not_fabricated` | 422-443 | REAL(test) | — | pins the padding-honesty property AND consume_all |
| `short_codes_decode_near_the_tail` | 445-462 | REAL(test) | — | exact-tail-bit-length match through the padded window |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 9 | YES | every expected vector hand-verified during this audit; the equivalence sweep compares against an independently-written bit-at-a-time walk on randomized inputs — kills offset bugs, LUT-range bugs, consume-count bugs, padding fabrication; negative tests pin the build gates |
No `#[ignore]`, no mocks (BitReader is the production reader), no self-generated goldens. All 9 executed ok in baseline (test-workspace.log.txt:700-708).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "decode of a code absent from the table or of exhausted entropy data is a typed [`Stop`]" | huffman.rs:5-7 | REAL | Stop enum + 4 dedicated tests; executed in baseline |
| "the entry's length is validated against the reader's available count … a truncated stream yields the same Stop::Exhausted" (ED-FIX-1) | huffman.rs:9-23 | REAL | :158-163 code + truncation test :423-443 + equivalence sweep |
| "Build is total (no panics)" | huffman.rs:5 | REAL | all arithmetic checked/bounded — traced; no unwrap/expect/index-unguarded in 65-139 |
| "capacity retained, bounded by the 256-symbol DHT limit" | huffman.rs:81 | REAL | upstream gate parser.rs:488-490 (`total > 256` reject) sed-verified |
| wave-6 hostile question | task prompt | N/A directly | this file is MJPEG (the shipped non-H.264 lane); it IS the proof the first vertical decodes — baseline AVI 60/60 |

## 7. Dead / duplicated / unreachable code
- `decode_naive` (:319-343) is test-only by location (inside `mod tests`) and deliberately kept as the oracle — not dead.
- The oversubscription check is duplicated between parser.rs:497-511 (parse-time) and rebuild :95-110 (build-time) — deliberate defense-in-depth, both commented; not flagged as duplication.
- Nothing dead found; every production symbol has callers in decoder.rs.

## 8. Dependencies & flags
- `super::bitreader::BitReader` (crate-internal) only; std; no unsafe; no feature gate (mjpeg is unconditional per codecs/Cargo.toml:25-30 — the IN-HOUSE list). Lint-clean (baseline clippy #5/#6). pub(crate) visibility — correctly not exported.

## 9. Open questions for the next agent
- bitreader.rs owner: `peek_bits`'s `debug_assert!(n <= 32)` — the slow path only ever peeks ≤ 16; fine, but the contract's n ≤ 32 vs `decode`'s n ≤ 16 usage is worth a cross-check from your side.
- decoder.rs owner: the concealment policy when `refresh_table` fails mid-stream (build-time Stop::InvalidCode on a re-defined DHT) — does it reject the frame or conceal? The type conflation (F-002) forces the call-site distinction there.
