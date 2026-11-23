# AUDIT: crates/demux/src/mkv/ebml.rs
| Field | Value |
|---|---|
| Auditor | W05-A10 |
| Date | 2026-11-23 |
| Lines / functions | 670 / 17 prod (+4 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/containers/mod.rs:57 `"matroska" => mkv::open(stream, opts)` -> mkv/mod.rs:164 `EbmlReader::new(stream)` -> every parse path (header.rs, cluster.rs, cues.rs, tags.rs, chapters.rs, editions.rs, attachments.rs) via `ebml::parse_header_at` / `parse_vint_*` / `read_*_at` (17+ call sites rg-verified) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/demux-036dd705535b2fb3 mkv::ebml::tests` -> 4 passed, 0 failed, exit 0; python3 exact replication of `parse_vint_size` (edge battery); rg for `end_of_header`, `MAX_PARSE_DEPTH`, `master::walk`, `bad_vint(` |

## 1. One-paragraph verdict
The EBML primitive layer is genuinely implemented and correct on every hostile edge I could construct: all-ones unknown-size detection is right at every width including the 8-octet `0x01 FF×7` streaming form (vacuous first-byte value bits handled by `value_bits == mask-1` with mask=0x01 — verified by exact python replication, since no test covers it); 8-octet known sizes decode right (`0x01 00…01 5B B9` -> 0x15BB9); `int_from_be` sign extension is correct two's complement (hand-traced `[0x80,0x00]` -> -32768); sizes cap at 2^56-1 so no u64 overflow downstream; `prefix_len` clamps every read against `until.min(length)` so allocation is bounded by the physical stream. The debt: a dead public field (`end_of_header`, always identical to `data_pos`, read nowhere in the repo), a module doc that lies about the error taxonomy for unknown-size non-masters (claims `bad_vint`, code mints `bad_structure` at every enforcement site) and cites a `master::walk` module that does not exist, and zero direct tests for `EbmlReader` and `parse_header_at` inside this file.

## 2. What actually works (verified)
— verified by `./target/debug/deps/demux-036dd705535b2fb3 mkv::ebml::tests` -> `test result: ok. 4 passed; 0 failed` (all four unit tests, incl. the 8-octet known size and 1/2/3-octet unknown forms).
— verified by python3 exact replication of parse_vint_size: `[0x01,FF×7]` -> (Ok, 72057594037927935, unknown=True, len=8); `[0x01,FF×FF×FF×FF×FF×FF,FE]` -> unknown=False; `[0x01,00,00,00,00,01,5B,B9]` -> 89017/len 8. The all-ones formula at mask=0x01 is correct.
— verified by hand-trace: `int_from_be([0x80,0x00])` = ((-1<<8)|0x80)<<8 = -32768 (correct sign extension); `parse_vint_id(0x08…)` -> 5-octet pattern -> Invalid (len>4 guard fires before mask exhausts).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/mkv/ebml.rs:409-413 (and 481-487)
Code:
```
    /// Absolute position of the payload (past the id+size vints).
    pub data_pos: u64,
    /// Absolute position just past the header vints.
    pub end_of_header: u64,
}
```
What is wrong: `end_of_header` is assigned `after_id + size.len as u64` — the identical expression as `data_pos` (line 485-486) — and is read nowhere: repo-wide rg finds only the definition (412) and the assignment (486). Dead public field whose name implies a distinction from `data_pos` that does not exist; every reader of `ElementHeader` must wonder which to use (they are always equal). What correct looks like: delete it.
Confidence: HIGH — Verification: ran (rg over crates/: exactly 2 hits, both in this file)

F-002 [S4]
Where: crates/demux/src/mkv/ebml.rs:616-652 (the size-vint test)
Code:
```
        // Eight-byte sizes (the segment's `01 xx xx …` form).
        let big = [0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x5B, 0xB9];
        assert_eq!(
            parse_vint_size(&big),
            Ok(VintSize {
                value: 0x01_5B_B9,
                unknown: false,
                len: 8
            })
        );
```
What is wrong: The test covers 8-octet KNOWN sizes and 1/2/3-octet unknown sizes, but never the 8-octet UNKNOWN form `0x01 FF FF FF FF FF FF FF` — the canonical encoding real streaming muxers write for unknown-size Segments, and the exact edge this module's docs headline ("the streaming-EBML case", line 19). Repo-wide: no 8-octet unknown vint exists in any src or test file; the only integration pin of unknown-size is a 1-octet `0xFF` marker on a Tags master (tests/mkv_extras_wiring.rs:677-683). The code is correct (python replication above); a regression in the `mask-1` arithmetic for len=8 (where value_bits is vacuously 0) would pass every test in the repo.
Confidence: HIGH — Verification: ran (replication + rg)
F-003 [S4]
Where: crates/demux/src/mkv/ebml.rs:439-488 (parse_header_at) and 262-395 (EbmlReader)
Code:
```
        Err(VintError::NeedMore) => {
            // The parent bound cut the window before the id completed:
            // the parent's end decides (clean).
            if window < pos + 12 {
                return Ok(None);
            }
            // The stream itself ended mid-id: truncation.
            return Err(error::truncated(pos + 1, pos + bytes.len() as u64));
        }
```
What is wrong: Not a code bug — a coverage hole. `EbmlReader` (7 methods incl. the EOF-stop `read_bounded` loop and the clamping `prefix_len`) and `parse_header_at` (the triage above: parent-cut -> Ok(None) vs stream-cut -> typed truncation) have zero unit tests in this file; the `#[cfg(test)] mod tests` covers only the pure vint/value functions. The triage is the subtlest logic in the file. Indirect coverage exists piecemeal (e.g. tests/mkv_fixtures.rs:435 `truncated_cluster_is_the_typed_truncation` pins the truncation lane); the `Ok(None)` parent-cut lane's coverage elsewhere is UNVERIFIED (did not audit the integration suites' full matrix).
Confidence: HIGH (absence of direct tests) / UNVERIFIED (indirect coverage of the Ok(None) lane)
F-004 [S3]
Where: crates/demux/src/mkv/ebml.rs:9-19 (module doc) — three claims contradicted by code
Code:
```
//!   data-driven descent the parsers make — [`master::walk`]-style
//!   child loops — carries an explicit depth counter refused past
//!   [`MAX_PARSE_DEPTH`]).
...
//!   with an unknown size is a typed [`crate::mkv::error::bad_vint`].
```
What is wrong: (a) `[master::walk]` links to a module that does not exist anywhere in the crate (rg: the string occurs only here) — a phantom citation. (b) "the one data-driven descent" — there are three, with three different guards: chapters.rs:427 refuses past `ebml::MAX_PARSE_DEPTH` (16), editions.rs:283 past its own `MAX_ATOM_NESTING` (8), tags.rs:492 past its own `MAX_TAG_NESTING`; and header.rs:851-854 claims "the cluster walker uses it at runtime" — cluster.rs contains zero references to MAX_PARSE_DEPTH (rg-verified), so that claim is FALSE too. (c) "a non-master with an unknown size is a typed `bad_vint`" — every enforcement site mints `bad_structure`, not `bad_vint` (cluster.rs:123, quoted below; same in editions.rs/attachments.rs); `bad_vint(` is used only for vint-grammar and uint-decode failures. Why it matters: a consumer matching typed error codes (`demux.mkv.bad-vint` vs `demux.mkv.bad-structure`) is told the wrong string by the file's own contract; the depth-guard doc understates the actual (fine) state by misciting it.
Supporting quote, cluster.rs:119-124:
```
        let Some(child_end) = child.declared_end() else {
            // Unknown-size children inside a cluster are not legal —
            // the typed refusal (the streaming form is a Segment- or
            // Cluster-level property, not a Timecode one).
            return Err(error::bad_structure(child.data_pos));
        };
```
Confidence: HIGH — Verification: ran (rg for master::walk / MAX_PARSE_DEPTH / bad_vint( across the crate)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_vint_id | 179-207 | REAL | — | 4-octet cap correct; zero-head rejected; 5-octet patterns Invalid (traced) |
| parse_vint_size | 210-244 | REAL | — | all-ones detection correct at every width incl. 8 (replicated); 2^56 ceiling |
| VintError (enum) | 248-255 | REAL | — | — |
| EbmlReader::new | 274-292 | REAL | — | rejects unknown-length/unseekable/empty, typed |
| EbmlReader::length | 295-297 | REAL | — | — |
| EbmlReader::seek_to | 300-302 | REAL | — | — |
| EbmlReader::prefix_len | 306-309 | REAL | — | clamps to until.min(length); saturating — no overflow |
| EbmlReader::read_bounded | 313-324 | REAL | — | EOF-stop loop, READ_STEP_MAX chunks |
| EbmlReader::read_prefix_at | 336-348 | REAL | — | alloc bounded by prefix_len (physical length) |
| EbmlReader::read_prefix_into_at | 361-370 | REAL | — | zero-alloc twin; used by the packet path (mod.rs:812) |
| EbmlReader::read_exact_at | 379-394 | REAL | — | typed truncation both lanes |
| ElementHeader::declared_end | 418-424 | REAL | — | checked_add |
| parse_header_at | 439-488 | REAL | S4 | correct triage (traced); no direct tests (F-003) |
| uint_from_be | 492-501 | REAL | — | >8 -> None pinned by test |
| int_from_be | 505-518 | REAL | — | sign extension correct (traced); only -1/+1 pinned by test |
| float_from_be | 523-534 | REAL | — | 0/4/8 lanes pinned; other widths None pinned |
| utf8_from | 539-541 | REAL | — | invalid UTF-8 -> None pinned |
| tests (4) | 547-669 | REAL | S4 | honest for what they cover; 8-octet unknown missing (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| id_vints_roundtrip_all_four_lengths | YES | exact id+len for all 4 widths |
| id_vints_reject_invalid_grammar | YES | zero head, two 5-octet patterns, NeedMore pinned |
| size_vints_roundtrip_and_flag_unknown | YES (1/2/3-octet + 8-octet known) / NO (8-octet unknown) | F-002 |
| typed_values_follow_the_grammar | YES | uint/int/float/utf8 lanes incl. rejection cases |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "non-master with an unknown size is a typed bad_vint" | ebml.rs:17 | FALSE | cluster.rs:123 &c. mint bad_structure; rg `bad_vint(`: only vint-grammar/uint sites |
| "the one data-driven descent ... refused past MAX_PARSE_DEPTH" | ebml.rs:11-14 | FALSE (understated/mis-attributed) | chapters.rs:427 (yes), editions.rs:283 (own constant), tags.rs:492 (own constant), cluster.rs: zero refs |
| "the cluster walker uses it at runtime" | header.rs:851-854 (about MAX_PARSE_DEPTH) | FALSE | rg: cluster.rs has no MAX_PARSE_DEPTH/depth |
| `[master::walk]`-style child loops | ebml.rs:12 | UNREACHABLE/phantom | no `master` module exists; string occurs only here |
| "Declared sizes are hints: every size is checked against the enclosing bound" | ebml.rs:20-24 | REAL | prefix_len clamps every read (read); callers spot-checked (header.rs:307-341 clamp before read) |

## 7. Dead / duplicated / unreachable code
- `end_of_header` (F-001): dead public field, repo-wide.
- `[master::walk]` doc reference (F-004): phantom module.

## 8. Dependencies & flags
- No external deps (C5 std-only honored); uses media_types + plugin_api (SeekOrigin) + crate::mkv::error. No feature gates, no unsafe, no unwrap/expect in non-test code (house rules honored). `#[PROPOSAL: ...]` markers on MAX_PARSE_DEPTH=16 (documented as proposal).

## 9. Open questions for the next agent
- Does any integration suite pin `parse_header_at`'s Ok(None) parent-cut lane (a child header exactly straddling the parent's declared end)? UNVERIFIED here.
- header.rs / error.rs owners: header.rs:851-854's "cluster walker uses it at runtime" is false; error.rs:87-89's bad_vint doc lists "unknown-size marker on a non-master" as a bad_vint trigger — same falsehood family as F-004(c).
- Should `parse_vint_id` reject the all-ones 1-octet head `0xFF` as an ID (RFC 8794 reserves it)? Current parser accepts it as id 0xFF; impact nil (skipped as unknown). LOW confidence on the RFC wording — spec text not available in sandbox.
