# AUDIT: crates/demux/src/ogg/page.rs
| Field | Value |
|---|---|
| Auditor | W04-A09 |
| Date | 2026-02-14 |
| Lines / functions | 341 / 11 (5 prod fns + PageHeader impl 3 + 5 tests + page_bytes helper) |
| File verdict | REAL (one function UNREACHABLE: `crc_valid`) |
| Reachable from shipped entry point? | YES: lumen-cli `apps/cli/src/containers/ogg.rs:60,81` -> `OggDemuxer` -> `page::parse_header` (:605 mod.rs), `page::crc32` (:642). `crc_valid` (:218) is NOT on the path — zero callers outside its own unit tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `./target/debug/deps/demux-5e0c7194d741938b --test-threads=1 'ogg::page'` -> 5 passed, exit 0; python3 bitwise CRC reference -> crc32(b"123456789")=0x89a1897f (matches test constant), full-page-with-LE-CRC residue ≠ 0 (confirms zero-field discipline matters); rg `crc_valid` workspace-wide -> only page.rs; sed quote verifications |

## 1. One-paragraph verdict
The Ogg page layer is real and correct: the 27-byte header parse is
bounds-checked at both the window and stream levels with typed truncations,
the segment-table math (nsegs→table_end→body sum→total) is exact with no
overflow possible (≤255×255+282), the CRC-32 is the genuine RFC 3533 Ogg
variant — I recomputed the "123456789" check value 0x89a1897f with an
independent bitwise reference and it matches — and the demuxer's walk
(mod.rs:642) verifies every page's CRC with the field zeroed. All 5 unit
tests pass live. The one substantive defect: `crc_valid` — explicitly
documented as "the page verify seam" with a zero-alloc folding trick — is
dead code; the demuxer reimplements the same computation by building an
allocating patched copy per page (mod.rs:628-631), so the advertised seam is
UNREACHABLE and the hot path pays a Vec alloc + copy the seam was built to
avoid. Secondary: a false doc sentence on `packet_runs`, dead unreachable
try_into arms, and the truncation branches are unit-untested in this file.

## 2. What actually works (verified)
— verified by live run `ogg::page` -> "test result: ok. 5 passed", exit 0.
CRC variant independently verified: python3 MSB-first poly 0x04C11DB7 init 0
no-reflect no-xorout over b"123456789" -> 0x89a1897f == test constant
(page.rs:292). Demuxer CRC verification confirmed in mod.rs:626-642 (field
zeroed via `&[0u8;4]` splice, compared against `header.crc`). Truncation
path exercised demuxer-side by ogg/tests.rs:263
(`truncated_page_is_a_typed_sticky_error`, part of the passing demux lib
suite).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/ogg/page.rs:216-238
Code:
```
/// Verify a whole in-memory page (header + table + body, CRC field
/// as stored): `true` iff the recomputed CRC matches.
pub fn crc_valid(page: &[u8]) -> bool {
    ...
    // The CRC field itself is zeroed for the computation (RFC 3533
    // §5) — the four zero bytes are INPUT, not a gap: fold them, then
    // the rest, never allocating a patched copy.
    let mut crc = crc32(&stored[0..22]);
    crc = crc32_cont(crc, &[0u8; 4]);
```
What is wrong: the function is dead in production — rg over the entire
workspace finds zero callers outside page.rs's own tests. The demuxer
(mod.rs:628-642) duplicates this exact logic but builds `whole` as a fresh
`Vec` per page (`Vec::with_capacity(head_len + body_len)` + three
extend_from_slice + body copy — up to ~65 KB copied per page) instead of
using this seam, whose comment advertises "never allocating a patched copy"
and whose own doc calls it "the page verify seam". So the zero-alloc verify
path exists, is tested, and is bypassed. Why it matters: a per-page
allocation + full-page memcpy on the Ogg walk's hot path (every page!), the
exact cost the seam eliminates; plus dead public API. What correct looks
like: demuxer computes `crc32_cont` over its existing buffers (head[0..22],
zeros, head[26..], body view) and deletes the copy; or delete crc_valid.
Confidence: HIGH — Verification: read + rg (callers) + live tests.

F-002 [S4]
Where: crates/demux/src/ogg/page.rs:78-80
Code:
```
    /// A run is terminal when its lacing value < 255 (the packet
    /// ends here); runs are the maximal groups of body bytes a
    /// single packet owns on this page.
```
What is wrong: the sentence is false. For a packet spanning lacing entries
[255, 100] the code returns TWO runs (0,255,false) and (255,100,true); the
maximal group the packet owns is one 355-byte span. Runs are per-LACING-ENTRY
slices, not maximal per-packet groups. What correct looks like: "one run per
lacing entry; consecutive non-terminal runs belong to the same packet".
Confidence: HIGH — Verification: read + the file's own test at :322-325
returns 3 runs for 3 laces.

F-003 [S4]
Where: crates/demux/src/ogg/page.rs:109-140 (four occurrences)
Code:
```
            .map_err(|_| error::bad_capture(offset))?,
```
What is wrong: four `try_into` Err arms are documented as unreachable
("HEADER_BYTES (27) is checked above; the Err arm is unreachable slicing
residue") yet still construct an error value — dead defensive code with a
misleading error kind (bad_capture for a slice-length impossibility). Not a
panic risk (that's the point), but noise: a `debug_assert!(head.len() >=
HEADER_BYTES)` once would document the invariant better than four dead arms.
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/demux/src/ogg/page.rs:150-159
Code:
```
    let table_end = (HEADER_BYTES + nsegs) as u64;
    if offset + table_end > stream_len {
        return Err(error::truncated_page(offset + table_end, stream_len));
    }
    let segments = head[HEADER_BYTES..HEADER_BYTES + nsegs].to_vec();
    let body_len: usize = segments.iter().map(|&s| s as usize).sum();
    let total = table_end + body_len as u64;
    if offset + total > stream_len {
        return Err(error::truncated_page(offset + total, stream_len));
    }
```
What is wrong: neither truncation branch has a unit test in this file — the
suite tests version/zero-segments/field parse/runs/CRC, but the
stream-length refusals are covered only indirectly through the demuxer
(ogg/tests.rs:263). A regression that flipped `>` to `>=` or dropped the
body check would not fail any page.rs test. The math itself is correct
(table_end ≤ 282, body_len ≤ 65,025 — no overflow; offset bounded by
stream_len). What correct looks like: two table-driven cases
(table-past-end, body-past-end) asserting the typed
`demux.ogg.truncated-page` code.
Confidence: HIGH — Verification: read + rg of this file's tests.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PageHeader::total_len / end | 66-74 | REAL | — | 27+nsegs+body; callers verified (mod.rs walk) |
| PageHeader::packet_runs | 76-89 | REAL | S4 (F-002) | math correct; doc sentence false |
| parse_header | 100-173 | REAL | S4 (F-003, F-004) | window + stream bounds both checked, typed errors; truncation branches untested in-file |
| crc_table | 181-202 | REAL | — | MSB-first, poly 0x04C11DB7; OnceLock; verified vs bitwise reference |
| crc32 | 205-214 | REAL | — | correct; check value independently reproduced |
| crc_valid | 218-238 | UNREACHABLE | S3 (F-001) | zero callers outside own tests; demuxer duplicates with alloc |
| crc32_cont | 242-249 | REAL (via crc_valid only) | — | correct continuation; transitively unreachable in prod |
| tests (5) + page_bytes | 252-341 | REAL | — | all pass live; CRC flip test catches corruption |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| crc_round_trips_and_catches_flips | YES | patched-CRC verify + two single-bit corruptions must fail |
| crc_matches_the_ogg_variant_check_value | YES | hard check value; independently reproduced by me (python3) |
| header_fields_parse_little_endian | YES | all fields + total_len pinned |
| lacing_runs_mark_packet_boundaries | YES | exact (start,len,terminal) triples incl. the 255 continuation |
| nonzero_version_and_zero_segments_are_typed | PARTIAL | asserts is_err() only — the error KIND is not pinned (a swapped error family would pass) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the Ogg CRC-32 — verify and generate, native (C5: no new deps; the polynomial and shift are specified by RFC 3533 §5)" | page.rs:3-4 | REAL | python3 bitwise reference == 0x89a1897f; table+fold implementation verified |
| "never allocating a patched copy" (crc_valid) | page.rs:232 | REAL for the function, IRRELEVANT in prod | function never called by the demuxer; mod.rs allocates a patched copy instead (F-001) |
| "runs are the maximal groups of body bytes a single packet owns" | page.rs:79-80 | FALSE | code returns per-lacing entries; own test shows [255,100]→2 runs |

## 7. Dead / duplicated / unreachable code
- `crc_valid` + `crc32_cont` — production-dead (F-001); the demuxer's
  mod.rs:626-642 and :683-696 duplicate the zero-field splice twice more
  (two copies of the same patched-copy construction in mod.rs itself).

## 8. Dependencies & flags
media_types + super::error only. No unsafe. No features. Std OnceLock.

## 9. Open questions for the next agent
- ogg/mod.rs (the walk, not assigned to me): the per-page `whole` Vec
  (F-001) contradicts the zero-alloc discipline proven elsewhere in this
  crate (zero_alloc_read_chain audits the AVI seams); an ogg zero-alloc test
  would fail on this today — worth a dedicated check.
- recover_sync's `budget_end.max(at + 1)` (mod.rs:667) — potential
  window-size subtlety when budget_end < at; audit when mod.rs is assigned.
