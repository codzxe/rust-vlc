# AUDIT: crates/demux/src/ts/psi.rs
| Field | Value |
|---|---|
| Auditor | W05-A09 |
| Date | 2025-06-27 |
| Lines / functions | 606 / 17 (no in-file tests; unit tests in ts/tests.rs, corpus in tests/ts_corpus.rs) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli containers/mod.rs:72 `"mpegts" => ts::open` -> containers/ts.rs:110 `TsDemuxer::new` -> `start()` PSI scan -> `parse_section` (ts/mod.rs:917) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 3 S4: 4 |
| Commands run | `cargo test -p demux --lib ts::tests` (exit 0, 28 ts tests pass; 457 lib total); python3 independent CRC check (`123456789` -> 0x0376E6E7, matches standard); `sed -n` quote verification |

## 1. One-paragraph verdict
Real PSI/SI parser with correct section-length arithmetic (12-bit mask,
`whole = 3 + section_length`, exact-length contract), a correct CRC-32/MPEG-2
(independently verified against the standard check value; the real-corpus
differential pins `psi_crc_failures == 0` on ffmpeg-authored bytes), correct
PMT/SDT field offsets, and strictly-advancing bounded loops. The hostile-focus
gaps: NO version/section-number continuity exists anywhere (body[6]/body[7]
never read; a stale-version section silently clobbers fresh table data in
`apply_section`), the ISO 6937 cedilla composition sits on the wrong lead
byte, a lying SDT descriptor length drops the whole service while the comment
claims "degrades to unknown names", and the Wave-4 claim about
`unreachable!` at ts/mod.rs:369,372 is TRUE and violates the crate's own
documented house rule.

## 2. What actually works (verified)
— verified by `cargo test -p demux --lib ts::tests` -> 28 passed (PAT/PMT/SDT
byte fixtures, CRC damage, truncation, future-version skip, short-form skip,
sections spanning packets). — CRC-32/MPEG-2 verified independently:
python3 (MSB-first, poly 0x04C11DB7, init 0xFFFFFFFF) gives
`123456789` -> 0x0376E6E7 (the standard check value); psi.rs:91-97 is the
identical algorithm in table form. — Corpus differential (real ffmpeg TS
bytes, `psi_crc_failures == 0`): baseline cargo-test-workspace.txt:2409-2429
`corpus_ts_lanes_match_the_ffprobe_truth ... ok` (corpus absent from sandbox
today, so not re-run by me — baseline cited). — Loop safety: PAT stride 4,
PMT stride 5+eil, SDT stride 5+dl, each bounded before field reads; all
sub-lengths clamped to parent (`12 + pil > body.len()`, `at + 5 + eil >
body.len()`, `at + 5 + dl > body.len()`); allocation bounded by
MAX_SECTION_BYTES = 4098.

## 3. Findings
F-001 [S2]
Where: crates/demux/src/ts/psi.rs:310-313 (and consumers ts/mod.rs:723-803)
Code:
```
    let body = &buf[..whole - 4]; // CRC stripped
    // Long-form tail: version/current_next + section numbers.
    let current_next = body[5] & 0x01 != 0;
    if !current_next {
```
What is wrong: `body[5]`'s 5-bit `version_number` is never extracted,
`body[6]` (`section_number`) and `body[7]` (`last_section_number`) are never
read by anything. There is no version continuity anywhere in the zone. Why it
matters: ISO 13818-1 §2.4.4.4 receivers must treat a version change as table
replacement; without version comparison, a late/replayed STALE-version PMT
silently overwrites newer PMT data (ts/mod.rs:783 `prog.streams = streams`),
PAT programs removed by a newer version persist forever (apply only adds or
updates, never removes), and PIDs dropped by a newer PMT stay in `es_pids`
forever (ts/mod.rs:774-776 insert-only). Multi-section tables (legal for PAT
and SDT) get no section_number continuity check at all. Realistic for
captured broadcasts — the corpus itself has a `0000-param-midchange.ts` lane
(mid-stream parameter change). What correct looks like: parse
version/section_number into `Pat`/`Pmt`/`Sdt`, and have `apply_section`
refuse stale versions and prune removed entries.
Confidence: MEDIUM — Verification: read (psi.rs + apply_section ts/mod.rs:723-803;
rg `version|section_number` in ts/ hits nothing but the current_next bit).

F-002 [S3]
Where: crates/demux/src/ts/psi.rs:440-445
Code:
```
        if at + 2 + len > descriptors.len() {
            // A lying descriptor length: the service degrades to
            // unknown names (the loop above already validated the
            // parent length, so this is defensiveness, not policy).
            return None;
        }
```
What is wrong: `return None` makes `parse_sdt` drop the service ENTIRELY
(psi.rs:421-423 `if let Some(svc) ... push`), and the same `None` fires when
a service simply has no 0x48 descriptor (line 475). The comment claims "the
service degrades to unknown names" — the code deletes the service from the
surface. Why it matters: comment lies; services whose descriptor loop carries
any other descriptor before a lying one, or no 0x48 at all, vanish from
`programs()` silently. What correct looks like: return
`Some(SdtService { names: None, .. })` for the no-0x48/lying-length cases.
Confidence: HIGH — Verification: read.

F-003 [S3]
Where: crates/demux/src/ts/psi.rs:589-592
Code:
```
        0xCB => match base {
            b'c' => 'Ç',
            _ => return None,
        },
```
What is wrong: ISO 6937 / EN 300 468 annex A assigns cedilla to lead 0xCA
and double acute to 0xCB. The code puts cedilla on 0xCB and leaves 0xCA
unmapped, so a standard-encoded French `Ç` (0xCA 'c') falls back to the
Latin-1 reading "Êc", and a standard 0xCB pair (ő/ű, double acute) can never
compose. Why it matters: wrong rendering of realistic Western DVB text; the
doc (psi.rs:38-39) presents this table as "the common Western set". What
correct looks like: `0xCA => b'c' => 'Ç'` (+ 0xCB double acute if carried).
Confidence: MEDIUM (ISO 6937 lead-byte table from standards knowledge; the
spec text is not in the sandbox — UNVERIFIED against the document itself).
Verification: read + ffmpeg's public dvbchar table from memory.

F-004 [S3] — Wave-4 claim verification (TRUE)
Where: crates/demux/src/ts/mod.rs:366-372 vs crates/demux/src/lib.rs:37-40
Code:
```
                return CodecDescriptor::new(&format!("ts-stream-{other:02x}"))
                    .unwrap_or_else(|_| unreachable!("formatted names are valid"));
            }
        };
        CodecDescriptor::new(name).unwrap_or_else(|_| unreachable!("static names are valid"))
```
against the crate's own house rule:
```
//! Std-only, zero external dependencies (C5); no `unsafe`; no
//! `unwrap`/`expect`/`panic!`/`todo!`/`unreachable!`/`dbg!` in
//! non-test code; bounded memory (MP §12): every read is
```
What is wrong: the Wave-4 claim "no unreachable!/panic in non-test code is
violated at ts/mod.rs:369,372" is TRUE — both lines are `unreachable!` in
production (non-`#[cfg(test)]`) code, directly contradicting the documented
house rule. Not currently triggerable: `CodecDescriptor::new`
(media-types/src/ids.rs:582-591) rejects only empty or control-char strings,
and neither "ts-stream-{:02x}" nor the static names can fail. But it is a
latent panic in the demux path if that validation ever tightens, and the doc
rule is false as written. What correct looks like: propagate the error
(no `unreachable!` in non-test code, per the crate's own rule).
Confidence: HIGH — Verification: read; lines byte-verified with
`sed -n '366,373p'` (see log).

F-005 [S4]
Where: crates/demux/src/ts/psi.rs:285-288
Code:
```
    if section_length < 5 {
        // The long-form remainder (5 fixed bytes + 4 CRC) cannot fit.
        return Err(bad_structure(0));
    }
```
What is wrong: the minimum applies the LONG-form rule to short-form sections
too — a short-form section (syntax=0) with `section_length` 0..4 is
structurally legal (private tables), and would be `Skipped` anyway; instead
it surfaces as bad_structure ("unparseable" damage note).
Confidence: MEDIUM — Verification: read.

F-006 [S4]
Where: crates/demux/src/ts/psi.rs:334-338
Code:
```
    let transport_stream_id = u16::from_be_bytes([body[3], body[4]]);
    let mut entries = Vec::new();
    let mut at = 8;
    if body.len() < 8 || !body.len().is_multiple_of(4) {
```
What is wrong: indexes `body[3]`/`body[4]` BEFORE the length check; safe only
because `parse_section` guarantees `whole >= 12` (body.len() >= 8), which
also makes the `body.len() < 8` half of the check dead code. Any future
caller of this private fn with a shorter body panics.
Confidence: HIGH — Verification: read (caller guarantee at psi.rs:303-306).

F-007 [S4]
Where: crates/demux/src/ts/tests.rs:29-32 (tests FOR this file)
Code:
```
    // The known-check-value lane (poly 0x04C11DB7, init all-ones,
    // no reflection): 32 zero bytes → 0x4E08BFB4? — the corpus's own
    // section CRCs are the authority here; pin one exactly.
    assert_eq!(crc32_mpeg(&[]), 0xFFFF_FFFF);
```
What is wrong: the only pinned value is the init constant — a tautology that
passes with any table, however broken; the other assertions are circular
(test_support computes section CRCs with the same `crc32_mpeg`); and the
comment's guessed check value "0x4E08BFB4?" is wrong (true value for 32 zero
bytes is 0x4A55AF67 per my python computation). The implementation itself is
correct — evidenced by the real-corpus differential (`psi_crc_failures == 0`
on ffmpeg bytes, baseline:2409-2429) and my independent check-value run
("123456789" -> 0x0376E6E7). What correct looks like: pin
`crc32_mpeg(b"123456789") == 0x0376E6E7`.
Confidence: HIGH — Verification: ran (python3) + read.

F-008 [S4]
Where: crates/demux/src/ts/mod.rs:527-532 (consumer of this file's constant)
Code:
```
            let declared = 3 + ((usize::from(self.buf[1] & 0x0F) << 8) | usize::from(self.buf[2]));
            if declared > MAX_SECTION_BYTES {
                // Over the 12-bit ceiling: a corrupt length, drop it.
```
What is wrong: dead branch — the masked 12-bit field cannot exceed 4095, so
`declared <= 3 + 4095 == MAX_SECTION_BYTES` always; the "corrupt length"
path can never fire. Harmless but claims a guard that cannot guard.
Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| build_crc_table | 68-86 | REAL | - | const fn; correct MSB-first poly table |
| crc32_mpeg | 91-97 | REAL | - | independently verified vs check value |
| section_crc_ok | 102-104 | REAL | - | whole-section hash == 0, correct |
| mint | 108-110 | REAL | - | typed ctor helper |
| not_psi | 113-123 | REAL | - | sniffed[..16] — len-min safe |
| truncated | 128-135 | REAL | - | typed ctor |
| bad_structure | 140-149 | REAL | - | typed ctor |
| crc_mismatch | 154-162 | REAL | - | source-chain offset leaf |
| SectionOffset Display/Error | 169-174 | REAL | - | trivial |
| parse_section | 278-329 | REAL | S4 | F-005; otherwise correct arithmetic |
| parse_pat | 331-353 | REAL | S4 | F-006 ordering; stride/parse correct |
| parse_pmt | 355-396 | REAL | - | offsets verified (PCR 8..10, pil 10..12, stride 5+eil) |
| parse_sdt | 398-431 | REAL | - | 5-byte loop header per EN 300 468; clamped |
| parse_service_descriptor | 435-476 | REAL | S3 | F-002 service dropped, comment lies |
| dvb_text | 481-522 | REAL | - | selectors handled; bounded |
| decode_utf16be_lossy | 524-532 | REAL | - | odd tail byte dropped (documented approx) |
| compose_iso6937 | 538-606 | REAL | S3 | F-003 cedilla on wrong lead byte |

## 5. Test honesty (tests live in ts/tests.rs — see F-007)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crc32_matches_the_corpus_shape | NO (table broken) | init-value tautology + circular CRC; corpus test covers it for real |
| pat/pmt/sdt_parses_* | YES | hand-built bytes, field-level asserts |
| cat_and_future_and_short_sections_skip | YES | distinguishes the three skip paths |
| crc_mismatch_is_typed | YES | bit-flipped CRC must produce the typed code |
| truncated_and_bad_structure_are_typed | YES | both lanes pinned |
| dvb_text_prefixes_and_composition | YES (partially) | pins 0x05/0x15/0x10 lanes; does NOT pin 0xCB cedilla (F-003 untested) |
| sections_spanning_packets_and_multi_section_payloads | YES (spanning) | name overpromises: no multi-SECTION table is tested |
| (integration) ts_corpus.rs lanes | YES | real ffmpeg bytes vs measured ffprobe truth |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every declared sub-length is clamped to its parent before it is consumed" | psi.rs:19-23 | REAL | parse_pmt:365,380; parse_sdt:417; descriptor:440 |
| "the corpus SDT bytes pin 5 (a 6-byte read would eat the tag byte)" | psi.rs:25-31 | REAL | sdt_parses_service_names_the_5_byte_loop_header passes |
| "Non-representable compositions fall back to the raw byte" | psi.rs:44-46 | REAL | dvb_text fallback at 518; but F-003: the cedilla pair is mis-slotted, not "non-representable" |
| "no unreachable! in non-test code" (house rule) | demux/src/lib.rs:37-40 | FALSE | ts/mod.rs:369,372 (F-004, Wave-4 claim TRUE) |
| CRC table "poly 0x04C11DB7, init 0xFFFFFFFF, no reflection, no final xor" | psi.rs:64-65 | REAL | python3 check value 0x0376E6E7 |

## 7. Dead / duplicated / unreachable code
- psi.rs itself: no unreachable!/todo!/panic! (rg-verified).
- Dead: parse_pat's `body.len() < 8` arm (F-006), ts/mod.rs:528 ceiling check
  (F-008). The two `unreachable!` at ts/mod.rs:369,372 are live code paths
  whose panic arms are unreachable today (F-004).

## 8. Dependencies & flags
`media_types` only (typed errors); no feature gates; module always compiled
into `demux`, reachable via apps/cli containers (mod.rs:72). No #[allow].

## 9. Open questions for the next agent
- ts/mod.rs `apply_section` (723-803): confirm my read that PAT removals and
  PMT stream removals never propagate (F-001's consumer side) — that file is
  not mine; I read only the apply paths.
- Does any corpus lane carry a >1-section PAT/SDT? If not, F-001's
  multi-section half is untested and untestable with current fixtures.
