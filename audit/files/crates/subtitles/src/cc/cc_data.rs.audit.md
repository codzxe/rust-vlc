# AUDIT: crates/subtitles/src/cc/cc_data.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-21 |
| Lines / functions | 578 / 27 (12 production items + 8 tests + 5 test helpers + Display/Error impls) |
| File verdict | REAL-UNPROVEN (in-crate test-proven; UNREACHABLE from shipped entry points; one spec-conformance deviation) |
| Reachable from shipped entry point? | NO. In-crate callers: cc/mod.rs `CcDecoder::decode_ga94`/`decode_scte20` (whose only callers are tests/cc_acceptance.rs and cc/fuzz.rs), cc/fuzz.rs:71-78 (all three parsers). `CcDecoder` has zero users outside the crate (rg over crates/ apps/ → none). The claimed codecs-side feed does not exist: crates/codecs/src/video/h264/parser/sei.rs `SeiMessage::Unparsed` records only `payload_type`+`size` and SKIPS the payload bytes — `parse_ga94_sei` can never receive real SEI bytes from the codecs crate. `parse_mpeg2_ga94` has NO caller at all except cc/fuzz.rs:75 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | `rg -n "parse_ga94_sei\|parse_mpeg2_ga94\|parse_scte20\|CcDataTriplet" src/cc tests/ fuzz/` (caller map); `rg -n "decode_ga94\|decode_scte20\|CcDecoder" crates apps fuzz` (zero external users); `sed -n 280,320p src/cc/mod.rs` (wrapper wiring); Read on crates/codecs/src/video/h264/parser/sei.rs enum (Unparsed = type+size only); `sed -n 340,470p tests/cc_acceptance.rs` (fuzz-feed tests); baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269 (267 unit green) :5552 (cc_acceptance 11/0); `rg -c "#\[test\]" src/cc/*.rs` (cc_data 8 — matches docs/formats/sub-cc.md claim) |

## 1. One-paragraph verdict
A genuinely implemented, panic-free cc_data() carriage layer: three parsers (GA94 SEI,
MPEG-2 GA94, SCTE-20 bit-packed), a typed error enum with real Display text, and 8
honest tests (round-trips assert exact triplet fields, parity re-synthesis is verified
against the eia608 predicate, rejection paths assert specific variants). All indexing
is length-gated (`needed = 1 + n*3 + 1` with n ≤ 31 — bounded), the bit reader uses
`.get()` — no panic path exists. The serious defect is spec conformance: the flags
byte is implemented as `[reserved|process_cc_data_flag|zero_bit|cc_count]` with a
hard `BadZeroBit` refusal on bit 5 and no em_data skip, while the published ATSC
A/53 Part 4 / CEA-708 header is `[process_em_data_flag|process_cc_data_flag|
additional_data_flag|cc_count]` with a conditional em_data byte and
additional_user_data — legal streams carrying either are misparsed (em_data byte
read as a triplet head) or refused outright. Second: `parse_mpeg2_ga94` is fuzz-only
dead code and the module doc's "captures type-4 messages intact" overstates a seam
whose producing half discards the bytes. Everything here is doubly unreachable: no
shipped binary links the crate, and even in-crate nothing but tests/fuzz drives it.

## 2. What actually works (verified)
- Round-trip + rejection tests ran green in baseline:
  audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5269-5271
  ("Running unittests src/lib.rs … subtitles … 267 passed; 0 failed") and
  :5552+ (tests/cc_acceptance.rs "11 passed; 0 failed"). The 8 in-file tests are
  inside the 267; docs/formats/sub-cc.md:74's "cc_data 8" is exact (`rg -c`).
- Parity re-synthesis verified by hand: `with_odd_parity(0x14)` → 0x14 has 2 ones
  (even) → `0x94` (`data | 0x80`), matching the test's `data[0] & 0x7F == 0x14` +
  `parity_ok` assertions (eia608.rs:77-91).
- Bounds safety traced: `parse_cc_data_body` slices `body[1+i*3..1+i*3+3]` only after
  `body.len() >= 1 + n*3 + 1`; `BitReader::take` returns `None` at end-of-data via
  `self.data.get(self.bit / 8)?`; every `None` becomes a typed `Truncated`.

## 3. Findings
### F-001 [S2]
Where: crates/subtitles/src/cc/cc_data.rs:33-47 (grammar) and :178-183 (`flags_and_count`)
Code:
```
//! cc_data() {
//!   reserved               1  // '0' — tolerated if set
//!   process_cc_data_flag   1  // '0' => the payload is not captions
//!   zero_bit               1  // shall be '0' — typed refusal if set
//!   cc_count               5
```
and
```
    if byte & 0x20 != 0 {
        return Err(CcDataError::BadZeroBit);
    }
    // process_cc_data_flag == 0: the payload is not caption data —
    // an empty result, not an error (the A/53 allowance).
    Ok((byte & 0x40 != 0, byte & 0x1F))
```
What is wrong: The implemented flags byte is `[reserved(bit7), process_cc_data_flag
(bit6), zero_bit (bit5), cc_count (bits 0-4)]` with no conditional fields. The
published ATSC A/53 Part 4 / CEA-708 GA94 cc_data header is
`[process_em_data_flag(1)][process_cc_data_flag(1)][additional_data_flag(1)]
[cc_count(5)]`, followed by an 8-bit `em_data` byte when `process_em_data_flag=1`
and `additional_user_data()` when `additional_data_flag=1`. Consequences for legal
streams: (a) `process_em_data_flag=1` payloads ("tolerated if set") misalign — the
em_data byte is consumed as the first triplet's `[valid|type]` byte and the parse
produces garbage or a confusing `BadMarker`; (b) `additional_data_flag=1` payloads
are hard-refused as `BadZeroBit`. Bit 6 and the count field agree in both layouts,
so encoder-typical `0x40|count` bytes parse identically — every fixture in the tree
has bits 7 and 5 clear, so the tests cannot see the deviation. What correct looks
like: read bit7 as `process_em_data_flag` and skip one em_data byte when set; treat
bit5 as `additional_data_flag` (tolerate, skip past the marker to the payload end);
rename the error accordingly. Why it matters: this is the file's entire job — the
cc_data() grammar; a spec-conformant ATSC stream with emergency-message or
additional user data decodes wrongly or not at all. Confidence: MEDIUM — the
code-side facts are HIGH (read), the spec text is recalled from A/53 Part 4
Table 6.3 / CEA-708 §9.1.1 and could NOT be verified offline (no network, no spec
copy in tree; ffmpeg source not in tree). Verification: read + UNVERIFIED on the
spec layout.

### F-002 [S3]
Where: crates/subtitles/src/cc/cc_data.rs:9-16 (module doc) and :251-255 (`parse_mpeg2_ga94`)
Code:
```
//! * **GA94 SEI form** ([`parse_ga94_sei`]) — the H.264 SEI type-4
//!   `user_data_registered_itu_t_t35` payload as ATSC A/53 Part 4 /
//!   CTA-708 define it: country code `0xB5`, provider code `0x0031`,
//!   the `GA94` user identifier, `user_data_type_code 0x03`, then the
//!   `cc_data()` body. The codecs crate's SEI parser captures type-4
//!   messages intact and skipped (`SeiMessage::Unparsed`, the
//!   read-only seam named by ADR-012 "extraction surface (a)") —
//!   this is the consuming half.
```
and
```
/// Parse the GA94 closed-caption payload of an MPEG-2 picture
/// `user_data` field — the bytes after the `0x000001B2` start code,
/// starting at the `GA94` identifier. The SEI form
/// ([`parse_ga94_sei`]) is the T.35 superset; this entry exists
/// because the MPEG-2 carriage has no T.35 country/provider header.
```
What is wrong: Two overstatements. (1) "captures type-4 messages intact and
skipped" — the codecs crate's `SeiMessage::Unparsed` (h264/parser/sei.rs) records
`payload_type` + `size` and SKIPS the payload; the bytes are never retained, so
"this is the consuming half" describes a pipe whose producing half drops the
water. (2) `parse_mpeg2_ga94` — the "decode half" of the MPEG-2 carriage — has
exactly one caller in the tree: cc/fuzz.rs:75. No production path constructs its
input. docs/formats/sub-cc.md:65-67 is honest about this ("the feed from a live
H.264 decode is not wired"); this file's doc is not. Why it matters: the doc
asserts a working seam that does not exist; a reader concludes captions can flow
from an H.264/MPEG-2 decode. What correct looks like: state the seam status the
way sub-cc.md does. Confidence: HIGH — caller map and the codecs enum both read.

### F-003 [S4]
Where: crates/subtitles/src/cc/cc_data.rs:367-369
Code:
```
    // marker nibbles: 4 bits '1111' per caption byte pair — padding,
    // consumed when present, never required.
    let _ = r.take(count * 2 * 4);
```
What is wrong: "4 bits … per caption byte pair" computes to `count * 4`, but the
code consumes `count * 2 * 4` = 8 bits (one 0xFF byte) per pair. One of comment
and code matches SCTE 20; the tests do not pin it (test helper appends a single
0xFF and labels it "marker nibbles (approximate)" at :442). The value is discarded,
so behavior is unaffected — pure documentation drift. What correct looks like:
state the actual marker size the code assumes (one byte per pair) and cite the
SCTE 20 clause. Confidence: HIGH (comment/code mismatch read); which one matches
the spec: UNVERIFIED offline.

### F-004 [S4]
Where: crates/subtitles/src/cc/cc_data.rs:326-341
Code:
```
    let flags = r.take(2).ok_or(CcDataError::Truncated {
        what: "SCTE-20 flags",
        needed: 2,
        got: 0,
    })?;
```
What is wrong: Every SCTE-20 `Truncated` error hard-codes `got: 0` regardless of
how many bits were actually available, and `needed` counts bits while the
byte-aligned errors (`parse_cc_data_body`:195-200) count bytes — the diagnostics
units are inconsistent and the "got" figure is fiction. What correct looks like:
track consumed/available bits in `BitReader` and report them. Why it matters:
error text is this crate's damage surface; a wrong "got" misleads any caller
triage. Confidence: HIGH — read (test at :573-576 asserts only the variant, not
the fields).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CcDataTriplet` (struct) | 57-67 | REAL | — | plain data, Hash+Copy |
| `CcDataTriplet::cc` | 71-78 | REAL | — | test/constructor convenience; used by cc_acceptance |
| `CcDataError` (7 variants) | 82-121 | REAL | — | `#[non_exhaustive]`, carries offending values |
| `Display for CcDataError` | 123-155 | REAL | — | real messages; `b as char` on 0..=255 is Latin-1-ish, debug-only |
| `Error for CcDataError` | 157 | REAL | — | empty impl, correct |
| consts GA94/ATSC_*/CC_TYPE_CODE | 160-169 | REAL | — | values correct (0xB5/0x0031/0x03) |
| `flags_and_count` | 172-184 | REAL | S2 (F-001) | bit5 hard-refusal; bit7 ignored without em_data skip |
| `parse_cc_data_body` | 188-217 | REAL | — | bounded (n ≤ 31, needed ≤ 95); marker checked; trailing bytes tolerated per doc |
| `parse_ga94_sei` | 225-249 | REAL | — | 8-byte header gate; big-endian provider; callers: cc/mod.rs:290 + fuzz/tests |
| `parse_mpeg2_ga94` | 259-276 | REAL (dead-in-production) | S3 (F-002) | sole caller cc/fuzz.rs:75 |
| `BitReader::take` | 286-294 | REAL | — | MSB-first; `.get()`-gated; 248-bit marker take shifts bits out of u32 silently — value discarded, no UB |
| `parse_scte20` | 311-371 | REAL | S4 (F-003/F-004) | parity re-synthesis via `eia608::with_odd_parity` (verified); marker consumption comment mismatch; `got: 0` diagnostics |
| tests: `flags`/`triplet_bytes`/`ga94_body` | 380-398 | REAL helpers | — | build grammar-conformant vectors |
| tests: `BitWriter`/`scte20_body` | 401-444 | REAL helpers | — | MSB-first writer; marker byte "approximate" |
| `ga94_roundtrip` | 446-460 | REAL test | — | exact triplet equality |
| `ga94_rejects_wrong_country_provider_identifier_type` | 462-488 | REAL test | — | 4 distinct variants |
| `ga94_rejects_truncation_marker_and_zero_bit` | 490-507 | REAL test | — | truncation + BadMarker + BadZeroBit |
| `process_flag_zero_is_empty_not_error` | 510-514 | REAL test | — | pins the A/53 allowance |
| `trailing_bytes_after_marker_are_tolerated` | 517-521 | REAL test | — | pins dirty-world tolerance |
| `mpeg2_ga94_entry_matches_sei_body` | 524-534 | REAL test | — | pins the fuzz-only entry's grammar |
| `scte20_roundtrip_with_resynthesized_parity` | 537-559 | REAL test | — | asserts 7-bit payloads + parity_ok on all bytes |
| `scte20_truncated_is_typed` | 562-577 | REAL test | — | empty/short/mid-count truncation |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ga94_roundtrip | YES | full triplet equality incl. type 2 (708) |
| ga94_rejects_wrong_country_provider_identifier_type | YES | asserts each specific variant |
| ga94_rejects_truncation_marker_and_zero_bit | YES | truncation depth, marker byte, zero bit |
| process_flag_zero_is_empty_not_error | YES | pins empty-not-Err |
| trailing_bytes_after_marker_are_tolerated | YES | pins len==1 |
| mpeg2_ga94_entry_matches_sei_body | YES | byte equality on data pair |
| scte20_roundtrip_with_resynthesized_parity | YES | 7-bit payload + parity predicate both asserted |
| scte20_truncated_is_typed | YES | three distinct truncation points |
Coverage gap (not a test lie): no fixture sets bit 7 or bit 5 of the flags byte —
F-001's deviation is invisible to the suite. Baseline: green
(orchrun-test-workspace-nofailfast.log.txt:5269, :5552).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Carriage grammars: GA94 SEI type-4 …, MPEG-2 picture-`user_data` GA94, SCTE-20 bit-packed" | docs/formats/sub-cc.md:29-30 | REAL (in-crate) | all three parsers exist and are tested |
| "cc_data 8" unit tests | docs/formats/sub-cc.md:74 | REAL | `rg -c "#\[test\]"` → 8 (exact) |
| status `implemented` (win/linux/macos) | sub-cc.md:9-11; matrix row `subtitle-cc-608-708` | UNREACHABLE-in-product | CcDecoder has zero callers outside the crate; nothing shipped links `subtitles` |
| "the feed from a live H.264 decode is not wired" | sub-cc.md:65-67 | REAL (honest gap) | verified: SeiMessage::Unparsed keeps type+size only |
| "this is the consuming half [of the codecs SEI seam]" | cc_data.rs:13-16 | FALSE-in-practice (F-002) | producing half drops the bytes |
| "No end-to-end proof from a real H.264 SEI stream yet" | sub-cc.md:82-83 | REAL (honest) | corroborates F-002 |

## 7. Dead / duplicated / unreachable code
- `parse_mpeg2_ga94`: fuzz-only (cc/fuzz.rs:75) — the only parser entry with no
  non-fuzz caller.
- `CcDataTriplet::cc`: used by tests only (in-crate API convenience) — acceptable
  for a library crate, noted for completeness.
- No duplication found within the file.

## 8. Dependencies & flags
Uses `super::eia608` (with_odd_parity/parity_ok — verified present and correct at
eia608.rs:77-91). No external deps, no unsafe, no feature gates, no cfg beyond
`#[cfg(test)]`. Workspace lints inherited (no unwrap/expect in production code —
clippy deny, baseline #5 CLEAN).

## 9. Open questions for the next agent
- Whoever holds spec access (orchestrator): confirm F-001 against ATSC A/53 Part 4
  Table 6.3 / CEA-708-E §9.1.1 — specifically `process_em_data_flag`/`em_data`
  and `additional_data_flag`/`additional_user_data()` — and ffmpeg's
  `mpeg12dec.c` GA94 path (does it skip an em_data byte?).
- cc/mod.rs owner: does any CcUpdate consumer surface `CcDataError` text to a
  user, or is it dropped (F-004's `got: 0` would then be harmless)?
- cc/cea708.rs owner: does the 708 packet re-assembly depend on cc_type 2/3
  ordering that F-001(a) would corrupt first?
