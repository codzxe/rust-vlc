# AUDIT: crates/demux/src/ogg/page.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Lines / functions | 341 / 14 (4 page-layer fns, 1 struct + 3 methods, 4 CRC fns, 6 tests + 1 builder) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → containers/mod.rs "ogg" arm → `OggDemuxer` → ogg/mod.rs:605,681,1135,1223 call `page::parse_header`; :642,696,1144 call `page::crc32`; :851 calls `packet_runs`. Baseline: ogg::page::tests 5/5 ok + corpus_ogg.rs 2/2 ok (ffmpeg truth differential), nofailfast log lines 1809-1830, 2003-2009) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
The Ogg page layer is small, correct, and independently verifiable — and I verified
it: the CRC-32 implementation (poly 0x04C11DB7, MSB-first, init 0, no reflect, no
xorout) reproduces the exact check value the test pins (`0x89A1897F` over
"123456789") when recomputed from the RFC parameters in an independent Python
bitwise reference; the parse enforces every structural bound (27-byte floor,
version 0, nonzero segments, table within window, header+table+body within stream)
before any slice; and the demuxer-level tests exercise CRC-mismatch sync recovery
and continued-packet lacing against real semantics. Two defects: `crc_valid` — the
function whose own doc calls it "the page verify seam" — has zero production
callers because the demuxer re-implements the identical zero-field fold inline;
and the typed-error test asserts only `is_err()`, not which typed refusal.

## 2. What actually works (verified)
- CRC math independently reproduced: python3 bitwise reference (poly 0x04C11DB7,
  init 0, MSB-first, no reflect, no xorout) → `crc32(b"123456789") == 0x89a1897f`
  and `crc32([0]) == 0x0` — exactly the asserts at page.rs:292-293. The
  table-driven implementation (crc_table:181-202, crc32:205-214) is the standard
  MSB-first form; `table[0] == 0` explains the single-zero-byte case.
- All 5 page tests + the 17 ogg::tests lanes + corpus_ogg.rs (2/2, "match the
  ffmpeg truth") pass in baseline — audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1809-1830
  and 2003-2009.
- Bounded-memory discipline verified: `parse_header` checks `head.len() < 27 +
  nsegs` (page.rs:142-145) before the table copy, and `offset + table_end` /
  `offset + total` against `stream_len` (151-158) before the body is ever read;
  body_len ≤ 255×255 by construction. No allocation from unvalidated counts.
- The CRC-field-zeroing fold in `crc_valid` (22 bytes + 4 zero inputs + nsegs +
  table + body) is byte-exact RFC 3533 §5; the demuxer's inline twin
  (ogg/mod.rs:636-642, read) computes the same layout over pooled body bytes.

## 3. Findings

### F-001 [S3] `crc_valid` — the documented "verify seam" — is dead in production; the demuxer duplicates its logic inline
Where: crates/demux/src/ogg/page.rs:216-218
Code:
```rust
/// Verify a whole in-memory page (header + table + body, CRC field
/// as stored): `true` iff the recomputed CRC matches.
pub fn crc_valid(page: &[u8]) -> bool {
```
(and its doc'd role at page.rs:240-241: `/// Continue a CRC-32 computation from a
running register (the page verify seam: ...)`)
What is wrong: rg across crates/, apps/, tests/ shows `crc_valid`'s only callers are
its own tests (page.rs:277, 280, 283). The demuxer never calls it: ogg/mod.rs:636-642
hand-builds the zeroed-CRC page buffer and compares `page::crc32(&whole) !=
header.crc` — the same 22+4-zero+1+table+body fold `crc_valid` implements, written
out a second time (and again at mod.rs:696 and :1144).
Why it matters: Two copies of the CRC-field-zeroing layout is a drift hazard — a fix
to one (say, an off-by-one in the zeroed span) leaves the other wrong, and the dead
`pub fn` survives the dead-code lint so nothing flags it. The "verify seam" the docs
advertise is not the seam the product uses.
What correct looks like: Either the demuxer builds its `whole` buffer and calls a
`crc_valid_parts(stored_22, nsegs_table, body)` seam, or `crc_valid` is deleted and
its doc stops claiming to be the seam.
Confidence: HIGH — Verification: rg "crc_valid" crates/ apps/ tests/ → definition +
3 test lines only; read ogg/mod.rs:630-650 (inline fold).
Note: the inline version being used in production means CRC verification itself IS
live (bad_crc_page_is_dropped_with_sync_recovery, baseline :1814) — only the
function is dead, not the behavior.

### F-002 [S4] Typed-error test asserts only `is_err()` — which refusal is unpinned
Where: crates/demux/src/ogg/page.rs:328-339
Code:
```rust
    fn nonzero_version_and_zero_segments_are_typed() {
        let mut p = page_bytes();
        p[4] = 1;
        assert!(parse_header(&p, 0, 64).is_err());
```
and (page.rs:338-339)
```rust
        q.push(0); // nsegs = 0
        assert!(parse_header(&q, 0, 64).is_err());
```
What is wrong: `parse_header` mints at least five distinct typed refusals
(bad_capture, bad_version, bad_structure, truncated_page ×2). This test asserts
only that SOME error comes back for the version and nsegs lanes — if both
collapsed to `bad_capture` (or to any other code), the test stays green.
Why it matters: The error taxonomy is this crate's product surface (the error-table
convention pinned elsewhere, e.g. asf tests); a lane that stops minting its
specific code would go unnoticed here.
What correct looks like: `assert_eq!(err.context.code, "demux.ogg.<specific>")`
for each lane, as the asf/wav/avi test files do.
Confidence: HIGH — Verification: sed -n '328,340p' (quote verified); contrast with
asf/tests.rs:400-443 style.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PageHeader::total_len` | 67-69 | REAL | — | 27 + nsegs + body_len; usize→u64 cast safe (≤65 542). |
| `PageHeader::end` | 72-74 | REAL | — | offset + total; offset bounded by real file positions. |
| `PageHeader::packet_runs` | 81-89 | REAL | — | Maximal-run semantics correct (terminal = lace < 255); pinned by lacing test with a 255-continuation. Allocates one Vec per page — acceptable at page granularity. |
| `parse_header` | 100-173 | REAL | — | All bounds checked pre-slice; four `try_into` Err arms are documented-unreachable defensiveness (head.len() ≥ 27 makes [6..14]/[14..18]/[18..22]/[22..26] always exact) — deliberate, commented (8.A1 sweep), not flagged. |
| `crc_table` | 181-202 | REAL | — | OnceLock, MSB-first, `i << 24` top-entry construction — verified against independent python reference. |
| `crc32` | 205-214 | REAL | — | Table-driven MSB-first; check value verified (§2). |
| `crc_valid` | 218-238 | DEAD (prod) / REAL (test) | S3 | F-001. |
| `crc32_cont` | 242-249 | REAL | — | Running-register fold; only caller is crc_valid (and thus also test-only in effect — noted under F-001's drift hazard). |
| tests: `page_bytes` | 257-272 | REAL | — | Honest builder, CRC patched last. |
| tests: `crc_round_trips_and_catches_flips` | 274-284 | REAL | — | Positive + payload-flip + capture-break lanes. |
| tests: `crc_matches_the_ogg_variant_check_value` | 286-294 | REAL | — | Check value independently reproduced by this audit (§2). |
| tests: `header_fields_parse_little_endian` | 296-306 | REAL | — | All fields + total_len pinned. |
| tests: `lacing_runs_mark_packet_boundaries` | 308-326 | REAL | — | The continued/terminal run table pinned exactly. |
| tests: `nonzero_version_and_zero_segments_are_typed` | 328-340 | REAL | S4 | F-002: is_err() only. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| crc_round_trips_and_catches_flips | YES | Self-built page verifies; a flipped payload bit and a broken capture both fail — pins the fold AND the sensitivity. |
| crc_matches_the_ogg_variant_check_value | YES | Hard check-value assert — a wrong poly/init/reflection changes it (independently confirmed by this audit's python reference). |
| header_fields_parse_little_endian | YES | Every field's value asserted. |
| lacing_runs_mark_packet_boundaries | YES | Exact run triple asserted incl. the 255-continuation. |
| nonzero_version_and_zero_segments_are_typed | PARTIAL | F-002: error presence pinned, error identity not. |
| (integration) ogg::tests::bad_crc_page_is_dropped_with_sync_recovery | YES | The production CRC lane (inline fold): mismatch → damage + recovery, next page demuxes (baseline :1814). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "CRC-32 — verify and generate, native (C5: no new deps); the polynomial and shift are specified by RFC 3533 §5" | page.rs:2-4 | REAL | Independent python re-derivation matches (§2); std-only imports verified (page.rs:26-28, 182). |
| "nsegs 1..=255" / "zero segments is not valid" | page.rs:17, 146-149 | REAL | nsegs is a u8 (≤255 by type); 0 refused typed. |
| "body (sum of the lacing values, ≤ 255×255)" | page.rs:19 | REAL | Sum of ≤255 entries each ≤255 = 65 025 max. |
| crc_valid doc "the page verify seam" (via crc32_cont) | page.rs:240-241 | FALSE | The production verify path is the inline fold in ogg/mod.rs:636-642, not this seam (F-001). |
| "cross-checked against a real corpus page by the integration suite" | page.rs:290-291 | REAL | corpus_ogg.rs "corpus_oggs_match_the_ffmpeg_truth" ok in baseline (:2006-2007). |

## 7. Dead / duplicated / unreachable code
- `crc_valid` + (effectively) `crc32_cont`: production-dead, duplicated by the
  demuxer's inline fold (F-001).
- The four documented-unreachable `try_into` Err arms in parse_header — deliberate,
  commented; not counted as a finding.

## 8. Dependencies & flags
- No feature gates; ogg module unconditional; std-only (OnceLock for the CRC
  table — the one std::sync use). No unsafe. Shipped binary reaches this file
  through the "ogg" container arm.

## 9. Open questions for the next agent
- ogg/mod.rs (another agent's file): the inline CRC fold at :636-642 — if F-001 is
  fixed by extracting a parts-based seam, :696 and :1144 carry two more copies of
  the same pattern; all three should share it.
- Is `PageHeader.crc` (the stored value) ever surfaced in damage telemetry? (Not
  this file's concern; noted for the mod.rs auditor.)
