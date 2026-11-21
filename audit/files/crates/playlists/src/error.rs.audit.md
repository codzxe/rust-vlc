# AUDIT: crates/playlists/src/error.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-21 |
| Lines / functions | 86 / 8 (module doc + 2 error variants + 5 warning variants, see §4) |
| File verdict | UNREACHABLE (correct, redaction-clean typed vocabulary — but no shipped runtime path ever constructs one of these) |
| Reachable from shipped entry point? | **NO.** The types' only constructors are `m3u.rs` (parse paths) and `cue.rs:498,845` (its two fatal shapes) — both demonstrated unreachable in my m3u.rs#part1 audit (every `playlist.*` command is a typed refusal in both shipped hosts; the remote-playlist consumer is unwired). The types are *linked* into the shipped CLI via streaming→playlists, but no runtime path reaches a parse. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read (whole file); `rg -n "ParseError\|ParseWarning" crates/playlists/src/` (users: m3u.rs + cue.rs only); `rg -n "enum .*Error\|enum .*Warning" crates/playlists/src/{pls,xspf,cue,hls,mpd}.rs` (every other parser owns its vocabulary); `rg -n "ParseError\|PlsWarning" crates/playlists/src/pls.rs` (PLS does not use these types); `rg -rn "byte line cap\|LineTooLong" crates/playlists/tests/` (Display strings pinned by m3u_suite + xspf_suite, both green in baseline :4363-4385 + xspf section); `rg -n "pub use error" crates/playlists/src/lib.rs` (root re-export :77). |

## 1. One-paragraph verdict
A small, disciplined error vocabulary: two fatal shapes (line-cap, non-UTF-8) and five M3U dirty-world warnings, every `Display` string carrying kind + line number only — the MP §12 redaction rule is honored verbatim (I read all seven format strings; the only data besides line numbers is a byte *count*), and the strings are pinned byte-exact by the m3u suite in the baseline run. The one defect is scoping: this is the **M3U card's** vocabulary, yet it sits at the crate root re-exported as `playlists::{ParseError, ParseWarning}` as if it were the crate-wide error type, while PLS/XSPF/CUE/MPD/HLS each define their own — and the module doc's "only the D-1 line-cap violation and non-UTF-8 input are fatal" is M3U truth presented as crate truth (XSPF's depth guard is fatal too, pinned by `depth_guard_is_fatal_at_65`). Like everything else in this crate: correct, tested, and unreachable from any shipped binary.

## 2. What actually works (verified)
— Redaction rule verified by reading all `#[error(...)]` strings (lines 20-23, 33-35, 46, 53, 62, 70, 81): no line text, locations, URLs, or paths — only static prose, `line`, and (LineTooLong) a byte length.
— Display strings pinned by tests: `m3u_suite::ten_megabyte_line_is_a_typed_error` asserts the full string `"playlists: line 1 is 10485760 bytes, over the 65536-… line cap (D-1 bounded input)"` (match verified via rg in tests/m3u_suite.rs; suite 18/18 green — orchrun log :4363-4385); `xspf_suite` pins its twin.
— thiserror derive with the interpolated `MAX_LINE_BYTES` constant — compiles (baseline build CLEAN, run #1) and formats correctly per the pinned test.

## 3. Findings

### F-001 [S4]
Where: crates/playlists/src/error.rs:1-13 with crates/playlists/src/lib.rs:77
Code:
```
//! Typed parse errors and warnings (card 4.D1).
//!
//! Redaction rule (MP §12, matching `media-types`' error discipline): the
//! `Display` of every variant names the *kind* and the *line number* only.
```
```
/// Fatal parse failures. Dirty input usually degrades to a
/// [`ParseWarning`] instead (D-2); only the D-1 line-cap violation and
/// non-UTF-8 input are fatal, because both mean we cannot honestly
/// tokenize the file at all.
```
What is wrong: Scope deception by placement. `ParseError`/`ParseWarning` are the **M3U parser's** vocabulary — all five warning variants describe M3U tags (`#EXTINF`, `#EXT…`, `#EXT-X-*`), and the only users are `m3u.rs` plus `cue.rs` borrowing the two fatal shapes — yet they are re-exported at the crate root (`playlists::ParseError`) as the crate's error surface. PLS, XSPF, CUE, MPD, and HLS each define their own error/warning enums (`PlsWarning`, `XspfError`/`XspfWarning`, `CueWarning`, `MpdError`/`MpdWarning`, `HlsPlaylistError`), and XSPF *duplicates* the line-cap variant rather than reusing this one. The doc's "only the D-1 line-cap violation and non-UTF-8 input are fatal" is true for M3U and false as a crate statement (XSPF's depth guard is fatal — pinned by `depth_guard_is_fatal_at_65` in xspf_suite).
Why it matters: A consumer matching on `playlists::ParseError` at the crate boundary will silently miss every non-M3U failure shape; the crate-root name promises a totality the type does not have. The duplicated Xspf line-cap variant is the same drift in code form.
What correct looks like: Move the vocabulary to `m3u` (or rename to `M3uError`/`M3uWarning`) and keep the root export for the M3U entry points only; or mint a crate-level umbrella `PlaylistsError` that wraps the per-format enums. The doc's fatal-only claim should name its scope.
Confidence: HIGH — Verification: read + rg user map (header; every other parser's own enum located).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-6 | REAL | S4 (F-001) | Redaction rule honored (verified); scope claims are M3U-only but read crate-wide. |
| `ParseError::LineTooLong` | 16-29 | REAL | — | `{line, length}`; Display interpolates `MAX_LINE_BYTES`; pinned byte-exact by m3u_suite. Constructed at m3u.rs:158, cue.rs:498. |
| `ParseError::InvalidUtf8` | 31-36 | REAL | — | No payload; constructed at m3u.rs:314, cue.rs:845. |
| `ParseWarning::BadDuration` | 44-50 | REAL | — | Constructed m3u.rs:189. |
| `ParseWarning::ExtinfMissingColon` | 52-57 | REAL | — | Constructed m3u.rs:180. |
| `ParseWarning::DirectiveMissingColon` | 59-66 | REAL | — | Constructed m3u.rs:257, 274; doc covers the lone-`#EXTM` shape. |
| `ParseWarning::DanglingEntry` | 68-74 | REAL | — | Constructed m3u.rs:302; payload = first pending line. |
| `ParseWarning::BadExtXTag` | 76-85 | REAL | — | Constructed m3u.rs:204/215/226/235/244; doc lists the exact damage classes. |

## 5. Test honesty
No tests in this file. The vocabulary is pinned indirectly: `m3u_suite` (18/18 green, orchrun :4363-4385) asserts variant shapes (`ten_megabyte_line_is_a_typed_error` matches `ParseError::LineTooLong { line: 1, length }` AND its full Display string; `garbage_bytes_are_typed_invalid_utf8` pins `InvalidUtf8`; the dirty-fixture tests pin the warning set). Would they fail if the strings drifted? YES — the Display assertions are byte-exact.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Line text, locations, URLs and filesystem paths never enter an error or warning message" | error.rs:4-6 | REAL | All seven format strings read (§2). |
| "only the D-1 line-cap violation and non-UTF-8 input are fatal" | error.rs:11-13 | REAL for M3U, FALSE as crate scope | XSPF depth-guard fatality pinned by xspf_suite (`depth_guard_is_fatal_at_65`, green in baseline). |
| (implied) `playlists::ParseError` is the crate's parse-error surface | lib.rs:77 re-export | Misleading (F-001) | Five sibling parsers use their own enums (rg map, header). |

## 7. Dead / duplicated / unreachable code
- No dead variants: every variant has a constructor (map in §4).
- Duplication: XSPF's own `LineTooLong`-shaped variant repeats this file's message form instead of reusing the type (part of F-001).
- Layering note: `error.rs` imports `crate::m3u::MAX_LINE_BYTES` while `m3u.rs` imports `error::{ParseError, ParseWarning}` — a legal but circular in-crate module dependency; the cap constant would live more naturally beside the error that reports it.

## 8. Dependencies & flags
- `thiserror` (pre-audited config-family external, per allowlist), `crate::m3u::MAX_LINE_BYTES`. No feature gates, no unsafe, no panics. Types are `Clone + PartialEq + Eq` — comparison-friendly for tests (used by the suites' exact-match assertions).

## 9. Open questions for the next agent
- W10-A10 (m3u.rs#part2) / cue/xspf owners: if F-001's rename happens, `MAX_LINE_BYTES` should move with it — cue.rs reuses both the constant (via its own cap check at :498) and the variants.
- Whoever audits `lib.rs` (W10-A01): the root re-export list (:77-89) is where the scope deception is consummated — consider whether the crate needs a per-format export shape instead.
