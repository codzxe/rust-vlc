# AUDIT: crates/playlists/src/cue.rs#part1 (lines 1–477)
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-21 |
| Lines / functions | 477 audited (chunk part1 of 954; part2 = 478-954 is another agent's — `parse_cue`, `parse_cue_bytes`, `TrackBuilder`, `finalize_track`, `split_first_token`, `unquote`, `parse_file_spec`, `parse_time`; boundary is clean: part1 ends on `parse_cue`'s doc lines 476-477, part2 opens at the fn body 478) / 7 executable fns (`frames_to_ns`, three `from_token`s, `Display for TimeContext`, `CueSheet::track_windows`/`file_of`) + 10 type definitions (CueFile, CueFileKind, CueIndex, CueTrackType, CueFlag, CueTrack, TrackWindow, CueSheet, TimeContext, CueWarning) whose doc-invariants were verified against part2's construction sites |
| File verdict | UNREACHABLE (real, tested library code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. Census: `parse_cue` re-exported at crates/playlists/src/lib.rs:101-103, zero callers outside the playlists crate (rg over apps/ crates/ tools/ — the only other `parse_cue*` hits are Matroska cue points in demux/src/mkv/cues.rs, unrelated). Tests are the sole consumer (`crates/playlists/tests/cue_suite.rs`). No feature-matrix row claims CUE (rg -i cue in feature-matrix.toml → only Matroska/WebVTT/DVB usages). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `grep -n` structure map (18-item pin); `sed -n` verbatim verification of every quote; read of part2 (478-954) to adjudicate every doc-invariant in this range; `rg "parse_cue\|CueSheet"` across repo (reachability); `rg -i cue` feature-matrix.toml (no product claim); `grep MAX_LINE_BYTES` lib.rs:83 (doc-link target exists — root re-export real); `od` byte-check of `#[must_use]` line 108 (display-pipeline artifact ruled out); baseline: `orchrun-test-workspace-nofailfast.log.txt:4311-4337` (cue_suite 20/20 ok) |

## Chunk boundary note
Part1 = 1–477: module docs (the format model + gapless-window semantics + dirty-world contract, 1-97), imports, `FRAMES_PER_SECOND`, `frames_to_ns`, and the complete typed data model through `CueWarning` (372-474). Part2 (478-954) owns the parser. The 477/478 boundary falls exactly between `parse_cue`'s doc comment and its body — no split functions.

## 1. One-paragraph verdict
This chunk is the CUE-sheet type model and its contract documentation, and it is unusually verifiable: every doc-invariant stated here (INDEX owning the FILE context open at its line, a track's file being its INDEX 01's file, the pregap-exclusion window arithmetic, NonMonotonicIndex zeroing, MissingIndex01 skipping, the 1..=99/0..=74 numeric ranges, BOM/CRLF/case tolerance, MAX_LINE_BYTES fatality) checks out against the parser code in part2, which I read line-by-line to adjudicate it. The one piece of executable arithmetic, `frames_to_ns`, is correct with a verified round-to-nearest proof (remainder analysis below). Two documentation defects only: the doc-example's explanatory sentence garbles which track the pregap is excluded from, and the "session scope (before the first TRACK)" definition diverges from the code, which accepts session commands anywhere no track is open — including after the last track, where a stray TITLE silently overwrites the album title. The model is unreachable from any shipped binary — no caller outside the crate's own test suite exists, and no feature-matrix row claims it.

## 2. What actually works (verified)
— `frames_to_ns` (109-114): verified the round-to-nearest proof by remainder analysis. `1e9 % 75 == 25`, so `frames·1e9 mod 75 ∈ {0, 25, 50}` → fractional parts are exactly 0, 1/3, 2/3 ns (never 1/2); `+37` (floor(75/2)) then rounds 1/3 down and 2/3 up — max error 1/3 ns, satisfying the doc's "< 1 ns". Cross-checked against the doc-example numbers: frames 13,675 → 182,333,333,333 ns and 175 → 2,333,333,333 ns, both matching the asserted values at :88 and :91.
— Every type doc-invariant verified against part2's construction sites: `CueTrack.file` = INDEX 01's file (finalize_track :889 `file: start.file`); `TrackWindow.duration_ns` = next same-FILE INDEX 01 (parse_cue :812-822), `None` for a FILE's last track; `gapless_join` = previous same-FILE track not overrunning (:807-808); `windows[i].track == i` (:825); `CueWarning` variants carry no line content — redaction claim (:345-346, :370) holds across all 14 variants (read).
— Doc-link hygiene: `[MAX_LINE_BYTES](crate::MAX_LINE_BYTES)` (:66) resolves — lib.rs:83 re-exports it from m3u.
— Execution evidence for the model: cue_suite 20/20 ok in baseline (:4311-4337) — including `ac1_single_file_gapless_windows`, `non_monotonic_index_empties_window`, `bad_pregap_arithmetic_is_typed`, `line_cap_is_fatal_and_redacted`, and two fuzz-lite runs (`fuzz_lite_mutations_of_valid_sheet_5k`, `fuzz_lite_random_bytes_10k_no_panic`) — the no-panic claim of :68 is pinned by executed fuzz sweeps.

## 3. Findings

### F-001 [S4]
Where: crates/playlists/src/cue.rs:89-90
Code:
```
//! // Track 2's pregap (INDEX 00..01 = 175 frames) is excluded from
//! // track 1's *start* and exposed on track 2 instead.
```
What is wrong: The pregap is excluded from **track 2's duration**, not "track 1's start". Track 1's start (00:00:00) has no relationship to track 2's pregap; the module's own semantics section (:29-31) says the pregap is excluded from the track's own duration and the window arithmetic in part2 (:812-815) computes `duration_ns` from INDEX 01 to next INDEX 01 — i.e., track 2's pregap span lies inside track 1's *window*, and is excluded from track 2's *duration*.
Why it matters: This is the doc example a newcomer runs first (it is a compiled doctest, :72-97); its prose teaches the wrong subject of the exclusion even though the assertions below it are correct.
What correct looks like: "Track 2's pregap (INDEX 00..01 = 175 frames) lies inside track 1's window but is excluded from track 2's own duration and exposed on track 2 instead."
Confidence: HIGH — Verification: read + arithmetic (pregap 175 frames = 2,333,333,333 ns matches the assert at :91; duration math at part2 :812-815).

### F-002 [S4]
Where: crates/playlists/src/cue.rs:8-9 (with part2 :731-751)
Code:
```
//! * Session scope (before the first `TRACK`): `CATALOG`,
//!   `CDTEXTFILE`, `TITLE`, `PERFORMER`, `SONGWRITER`, `REM`.
```
What is wrong: The parenthetical defines session scope as "before the first TRACK", but the code routes `TITLE`/`PERFORMER`/`SONGWRITER` to the session whenever **no track is open** — including after the last track closes (part2 :740-751: `match &mut open { None => … sheet.title = Some(value) }`). A session-scope command appearing after all tracks silently overwrites the album-level field the earlier lines established.
Why it matters: The doc's scope model and the code's scope model differ on a real shape (trailing session commands after the track block — malformed but tolerated dirty-world input); a consumer relying on "session = prefix" semantics gets a different `sheet.title` than the doc promises. The behavior itself is defensible (last-wins dirty-world tolerance); the definition is what's wrong.
What correct looks like: Either doc "Session scope (anywhere no `TRACK` is open — last one wins)" or reject/warn post-track session commands.
Confidence: HIGH — Verification: read of both sites; no cue_suite test covers a post-track TITLE (20-test list verified at baseline :4313-4336).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| frames_to_ns | 109-114 | REAL | — | Round-to-nearest verified by remainder analysis (§2); saturating_mul defensive (max real value 449,999 frames from 99:59:74 — no saturation reachable). `#[must_use]` present (od-verified). |
| CueFileKind::from_token | 148-158 | REAL | — | 6 named kinds; unknown → Other(verbatim). Matches doc's CDRWIN+community split. Case-insensitive. |
| CueTrackType::from_token | 199-210 | REAL | — | AUDIO + 6 data modes; other verbatim. Matches doc :15-16. |
| CueFlag::from_token | 227-235 | REAL | — | DCP/4CH/PRE/SCMS; None → UnknownFlag warning path (part2 :716-720). |
| CueSheet::track_windows / file_of | 334-336 / 340-342 | REAL | — | Trivial accessors; `windows[i].track == i` invariant holds (part2 :825). |
| Display for TimeContext | 358-364 | REAL | — | Uppercase command names for warning text; no content embedded. |
| CueFile / CueFileKind / CueIndex / CueTrackType / CueFlag / CueTrack / TrackWindow / CueSheet / TimeContext / CueWarning (types) | 118-474 | REAL | S4 (F-001, F-002) | Data models; every doc-invariant cross-verified against part2 construction sites (§2). CueWarning: 14 variants, line-scoped vs arithmetic-scoped split documented :367-369 and honored (line-stage first, `late` appended after — part2 :836). |

## 5. Test honesty
No inline tests in this file; the model is tested by `crates/playlists/tests/cue_suite.rs` (part2-adjacent, 20 tests). Baseline execution: all 20 ok (:4311-4337). From names + asserted semantics: `ac1_single_file_gapless_windows` and `non_monotonic_index_empties_window` pin this chunk's TrackWindow invariants; `line_cap_is_fatal_and_redacted` pins the D-1 fatality + redaction; the two fuzz-lite tests pin the no-panic claim (:68). Mutation reasoning: inverting `gapless_join` or the pregap exclusion in part2 would fail ac1/ac3; breaking `frames_to_ns` rounding would fail the numeric asserts (2,333,333,333 / 182,333,333,333 are exact-nearest values). Line-level honesty of cue_suite belongs to part2's auditor.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "nanosecond fields are round-to-nearest conversions… error < 1 ns" | cue.rs:50-51 | REAL | Remainder analysis §2; max error 1/3 ns. |
| "Every INDEX belongs to the FILE context open at its own line… a track's own file is the file of its INDEX 01" | cue.rs:16-18 | REAL | part2 :655-660 (INDEX uses cur_file), :889 (track.file = start.file). |
| "Backwards INDEX 01… the earlier window is emptied to zero (duration_ns = Some(0)), gapless_join false" | cue.rs:63-65 | REAL | part2 :812-819 (Some(0)), :807-808 (join false for the offending track). |
| "A track that never gets a valid INDEX 01 — damage-note and the track is skipped" | cue.rs:57-59 | REAL | part2 :871-876. |
| "No panics on any byte input — pinned by the fuzz-lite suite" | cue.rs:68 | REAL | `fuzz_lite_random_bytes_10k_no_panic` + `fuzz_lite_mutations_of_valid_sheet_5k` ok (baseline :4321, :4336). |
| "Session scope (before the first TRACK)" | cue.rs:8-9 | PARTIALLY FALSE | F-002: code accepts session scope anywhere no track is open. |
| Doc example prose "excluded from track 1's *start*" | cue.rs:89-90 | FALSE (prose only) | F-001: assertions correct, sentence garbled. |
| (no product claim) | feature-matrix.toml | N/A | No CUE row exists — no inflation to adjudicate. |

## 7. Dead / duplicated / unreachable code
- Entire module unreachable from shipped binaries (sole consumers: cue_suite tests). Re-exported publicly (lib.rs:101-103) — API surface for a future consumer, currently uncalled.
- No duplication: the CUE time base is self-contained; it does not fork the M3U line-splitting (part2 reuses only MAX_LINE_BYTES).

## 8. Dependencies & flags
`crate::error::ParseError`, `crate::m3u::MAX_LINE_BYTES` (64 KiB, shared D-1 cap — legitimate reuse), `std::fmt`, `thiserror` for CueWarning Display (the crate's one derive use here). No feature gates, no unsafe, no I/O — pure text-in/model-out per the crate's design.

## 9. Open questions for the next agent
- W10-A?? (cue.rs part2): the parser half — line-stage handling read clean on my pass (every command's warning paths match the CueWarning vocabulary defined here); adjudicate `parse_file_spec`'s unterminated-quote shape (name consumes rest of line, no type — :921-927) for spec honesty, and whether cue_suite covers the post-track session-command shape F-002 documents incorrectly.
- If a consumer ever wires CUE into the CLI: the album-overwrite behavior (F-002's code half) becomes user-visible; revisit as S3 then.
