# AUDIT: crates/playlists/src/mpd.rs#part1
| Field | Value |
|---|---|
| Auditor | W10-A05 |
| Date | 2026-11-21 |
| Lines / functions | 642 (1–642 of 2855) / 21 items in table |
| Chunk boundary | part1 = 1–642. Equal 5-way split is 571; extended through the end of the `MpdWarning` enum (642) so the `// ----- parse` section marker at 644 opens part2 (A03). Lines 572–642 flagged as part1-owned in the log; A03 should start at 643. |
| File verdict | REAL (library; UNREACHABLE from shipped entry points) |
| Reachable from shipped entry point? | NO for everything in this range. The model types are constructed only by `parse_mpd`/`parse_mpd_bytes` (652/685, part2). Their only callers: `streaming/src/dash/resolve.rs:744`, `abr.rs:330`, `sched.rs:477` — all inside `#[cfg(test)]` (verified: `#[cfg(test)]` at resolve.rs:741, abr.rs:327, sched.rs:474) — plus `dash/fuzz.rs:59`, and `rg` over apps/ + tests/ finds zero uses of `streaming::dash` outside the streaming crate. `apps/cli` (the shipped binary) uses `streaming::{fetch,ranged,schemes,mockhttp}` only (rg over apps/cli/src). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "parse_mpd" crates/streaming/src apps/cli/src` (test-gated hits only); `rg -n "streaming::dash|dash::" apps/ tests/` (zero); `rg -n "response_limit" crates/streaming/src/fetch.rs` (`DEFAULT_RESPONSE_LIMIT: u64 = 8*1024*1024`, fetch.rs:28 — matches MAX_MPD_BYTES claim); `rg -n "MAX_LINE_BYTES" crates/playlists/src/m3u.rs` (m3u.rs:100 = 64*1024 — matches line-cap claim); `sed -n` verbatim checks at 138-145, 541-548, 231-234, 671-678; baseline: orchrun-test-workspace-nofailfast.log.txt:4278-4295 (16 mpd tests ok). |

## 1. One-paragraph verdict
Part1 is the typed MPD manifest model — 16 structs, a 7-variant error enum with stable dotted codes and hand-rolled `Display`, and a 10-variant warning enum — and it is honest, well-documented declarative code whose bounds claims (8 MiB total, 64 KiB line, depth 64) check out against the constants they cite in sibling crates. The parse functions that would vindicate the model live in part2 (A03) and the tests in part4 (A01), but baseline shows all 16 mpd tests passing. The stack it serves is dead in the shipped product: nothing outside `#[cfg(test)]`/fuzz calls `parse_mpd`, and the CLI never touches `streaming::dash`. One documented contract is false: `MpdParsed.warnings` claims "in document order" but the merge in `parse_mpd` appends all scanner warnings after all driver warnings, two buckets, not one order.

## 2. What actually works (verified)
— `MAX_MPD_BYTES = 8*1024*1024` (mpd.rs:132) matches `streaming/src/fetch.rs:28` `DEFAULT_RESPONSE_LIMIT: u64 = 8 * 1024 * 1024` (rg). The doc claim "matches the streaming port's default response_limit" is true.
— Line-cap claim: `m3u.rs:100 pub const MAX_LINE_BYTES: usize = 64 * 1024` (rg) — the "crate's 64 KiB cap" claim is true.
— All 16 `mpd::tests` pass in baseline: orchrun-test-workspace-nofailfast.log.txt:4278-4295, including `bounded_input_battery`, `truncated_refuses`, `depth_exceeded_refuses`, `error_code_table` (which pins the codes declared in this range).

## 3. Findings

### F-001 [S3]
Where: crates/playlists/src/mpd.rs:142-144 (contract), violated at 671-674 (part2, A03's range)
Code:
```
    /// Non-fatal findings (unknown elements/attrs, degraded
    /// representation facts, salvage notes) in document order.
    pub warnings: Vec<MpdWarning>,
```
(and, in `parse_mpd` at 671-674:)
```
    // Scanner-level findings (bad entities, tag markup damage)
    // join the driver's — two passes, one list (order: driver's
    // structural findings first, scanner's lexical ones after).
    driver.warnings.extend(scanner.warnings);
```
What is wrong: the public field's documented contract is "in document order", but the only producer appends every scanner warning (`BadEntity`, `BadTag`) after every driver warning (`UnknownElement`, …), regardless of line numbers. A host sorting or de-duplicating warnings by position gets lied to.
Why it matters: this is an API contract on a public field of the crate's primary return type; the in-code comment admits the real order while the field doc contradicts it.
What correct looks like: either merge the two streams by line number (both carry `line`) or fix the field doc to state the two-bucket order.
Confidence: HIGH — Verification: read of both sites; scanner warnings are only drained once, after the token loop (no interleaving).

### F-002 [S4]
Where: crates/playlists/src/mpd.rs:541
Code:
```
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum MpdWarning {
```
What is wrong: non-fatal diagnostics implement `std::error::Error` (via thiserror) while the fatal refusals (`MpdError`, 418-533) are hand-rolled with a parallel `code()` + `Display` pair. Two error machineries in one module; warnings-as-errors is a modeling smell that pushes hosts toward treating diagnostics as failures.
Why it matters: style drift inside one module; every future variant maintenance has two patterns to pick from.
What correct looks like: one mechanism for both (either both thiserror with the codes in a method, or both hand-rolled), and no `Error` impl on a non-error type.
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/playlists/src/mpd.rs:232-233
Code:
```
    /// `@startWithSAP` (enumerated 0/1/2), when present.
    pub start_with_sap: Option<u64>,
```
What is wrong: the doc says the attribute is an enumeration of 0/1/2 but the field is an unbounded `u64`; junk like `startWithSAP="7"` is representable in the model with no warning at the type level (parse-side policy for set-level facts is warn+drop, but the model cannot express "one of three").
Why it matters: a typed enum would make the constraint checkable; as written the doc carries semantics the type does not enforce.
What correct looks like: `Option<StartWithSap>` with a 3-value enum, or drop the "(enumerated 0/1/2)" from the doc.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAX_MPD_DEPTH` | 121-126 | REAL (lib-only) | — | 64; same as XSPF cap per doc. |
| `MAX_MPD_BYTES` | 128-132 | REAL (lib-only) | — | 8 MiB; verified to match fetch.rs:28. |
| `MpdParsed` | 136-145 | REAL (lib-only) | S3 (F-001) | Carries the false document-order contract. |
| `Mpd` | 147-178 | REAL (lib-only) | — | All attrs Optional except type; durations as f64 seconds (precision fine to µs for any sane duration); times recorded verbatim by documented policy. |
| `MpdType` | 180-190 | REAL (lib-only) | — | Static/Dynamic; spec default documented (parse side's job). |
| `Period` | 192-211 | REAL (lib-only) | — | Inheritance of segment_template documented as C1b's job. |
| `AdaptationSet` | 213-255 | REAL (lib-only) | S4 (F-003) | start_with_sap typing; content_type fallback contentType→mimeType documented. |
| `Representation` | 257-294 | REAL (lib-only) | — | bandwidth Optional + MissingBandwidth warning per tolerance policy — honest. |
| `SegmentTemplate` | 296-316 | REAL (lib-only) | — | Templates stay verbatim strings — matches the "honest unresolved" claim. |
| `SegmentTimeline` | 318-323 | REAL (lib-only) | — | Data only. |
| `SegmentTimelineEntry` | 325-337 | REAL (lib-only) | — | `r: Option<i64>` correctly models negative repeat (spec §5.3.9.6); `d: u64` required matches fatal-BadValue policy. |
| `SegmentList` | 339-354 | REAL (lib-only) | — | Data only. |
| `SegmentUrl` | 356-364 | REAL (lib-only) | — | media/mediaRange verbatim; range parsing deferred to C1b (documented). |
| `SegmentBase` | 366-380 | REAL (lib-only) | — | Data only. |
| `ContentProtection` | 382-394 | REAL (lib-only) | — | Recorded-not-interpreted policy matches the HLS `#EXT-X-KEY` split claim. |
| `Descriptor` | 396-407 | REAL (lib-only) | — | Data only. |
| `MpdError` | 411-465 | REAL (lib-only) | — | 7 variants, no content leakage (attr names are `&'static str` structural — MP §12 claim holds). |
| `MpdError::code` | 467-480 | REAL (lib-only) | — | Stable dotted codes; pinned by `error_code_table` test (baseline 4284). |
| `Display for MpdError` | 482-531 | REAL (lib-only) | — | Hand-rolled, includes code + caps; duplicates code strings with `code()` (deliberate house style, duplication noted). |
| `impl Error for MpdError` | 533 | REAL (lib-only) | — | Empty, no sources — correct for leaf errors. |
| `MpdWarning` | 537-642 | REAL (lib-only) | S4 (F-002) | 10 variants, all line-numbered, structural names only; `thiserror::Error` derive is F-002. |

## 5. Test honesty
No tests in this range (tests at 2089+, part4/A01). Baseline 4278-4295 shows the 16 tests passing, including `error_code_table` (pins part1's codes) and `duration_table` (pins the duration parsing that consumes part1's f64 fields). Would-fail analysis belongs to A01's audit; the model-range tests that exist do assert against these types.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Entity-expansion bombs are structurally impossible … pinned by test" | mpd.rs:63-68 module docs | REAL-UNPROVEN (here) | Scanner (decode-once, DOCTYPE skip) is part2/3 code; cross-ref to A03. The claim is tested per baseline test list (`fuzz_lite_mutations_never_panic`, 4295) but I did not trace the scanner. |
| "8 MiB … matching the fetch port's default response budget" | mpd.rs:57-58, 128-132 | REAL | fetch.rs:28 `DEFAULT_RESPONSE_LIMIT: u64 = 8 * 1024 * 1024` (rg). |
| "the crate's 64 KiB cap (m3u::MAX_LINE_BYTES)" | mpd.rs:59-60 | REAL | m3u.rs:100 (rg). |
| "warnings … in document order" | mpd.rs:142-144 | FALSE | parse_mpd:671-674 appends scanner bucket after driver bucket (F-001). |
| "No URIs/paths/line text (MP §12)" (error payloads) | mpd.rs:413-415 | REAL | All variants carry line numbers and structural `&'static str`/local names only (read). |
| "ContentProtection is recorded, never interpreted" | mpd.rs:19-22 | REAL | Model stores 3 optional strings; no DRM logic in range (read). |

## 7. Dead / duplicated / unreachable code
Everything in this range is unreachable from shipped entry points (see header) — the model exists to serve a parse core whose only consumers are `#[cfg(test)]` modules and the fuzz harness. No dead fields found within the model; every modeled attribute has a doc comment tying it to the spec.

## 8. Dependencies & flags
`use crate::m3u::MAX_LINE_BYTES` (intra-crate), `thiserror` via MpdWarning derive (declared in playlists/Cargo.toml — playlists crate audit exists). No feature gates, no unsafe, no panics in this range (declarative only).

## 9. Open questions for the next agent
- A03 (part2, from 643): `parse_mpd_bytes` validates UTF-8 BEFORE the size check — a 100 MB non-UTF-8 body reports `InvalidUtf8`, not `TooLarge`; confirm the intended precedence. Also owns the F-001 violating merge and the entity-decode-once scanner claim.
- A01 (part4/part5, tests at 2089+): confirm `dirty_world_degrades_to_warnings` asserts warning ORDER or only membership — F-001 says order is not document order.
- Matrix owner: DASH rows — this parse core is REAL but session-unwired from any shipped entry point; expected row verdict UNREACHABLE.
