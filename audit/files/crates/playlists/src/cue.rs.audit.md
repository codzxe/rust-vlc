# AUDIT: crates/playlists/src/cue.rs
| Field | Value |
|---|---|
| Auditor | W10-A04 |
| Date | 2026-02-14 |
| Lines / functions | 955 / 13 fns + 10 data types |
| File verdict | UNREACHABLE (real, well-tested parser; no shipped entry point calls it) |
| Reachable from shipped entry point? | NO — only playlists' own tests + fuzz target call `parse_cue`/`parse_cue_bytes` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `cargo test -p playlists --test cue_suite` → 20 passed/0 failed (0.20s); `cargo test -p playlists --doc cue` → 1 passed; rg caller hunts |

## 1. One-paragraph verdict
This is a genuine, careful CUE parser: correct Red Book time grammar with typed range refusals (`MM>99/SS>59/FF>74`, L950), a mathematically correct round-to-nearest frames→ns conversion (I verified `(f·1e9+37)/75` handles the 0/⅓/⅔ ns fractional cases — remainder of 1e9 mod 75 is 25, so ties never occur), honest gapless window arithmetic that never invents file lengths (`duration_ns: None` for a FILE's last track), per-INDEX file binding, and dirty-world degradation to typed warnings instead of panics — backed by 20 tests including a 10k-case random-bytes and 5k-case mutation fuzz-lite, all of which I ran green. The problem is that nothing ships it: `rg parse_cue` over the repo shows callers only in `crates/playlists/tests/cue_suite.rs` and `crates/playlists/src/fuzz.rs` (fuzz harness); the remote-playlist lane sniffs M3U/XSPF only, `probe()` (which can detect `.cue`) has zero external callers, and apps/cli has no playlists dependency at all. A user cannot get a CUE sheet parsed by any shipped binary.

## 2. What actually works (verified)
— `cargo test -p playlists --test cue_suite` → `test result: ok. 20 passed; 0 failed; 0 ignored` (includes `fuzz_lite_random_bytes_10k_no_panic`, `fuzz_lite_mutations_of_valid_sheet_5k`, `large_valid_sheet_under_200kb` with 4600 tracks, exact-ns window assertions in `ac1_single_file_gapless_windows`). Doctest of the module example → 1 passed. Hand-verified: 13,675 fr → 182,333,333,333 ns and 175 fr → 2,333,333,333 ns match the test's asserted hand-truth.

## 3. Findings
F-001 [S2]
Where: crates/playlists/src/cue.rs:478 (pub fn parse_cue) vs crates/streaming/src/remote_playlist.rs:224-236
Code:
```
pub fn parse_cue(input: &str) -> Result<CueSheet, ParseError> {
```
and the only production-adjacent playlist consumer:
```
fn sniff_format(body: &[u8]) -> RemotePlaylistFormat {
```
What is wrong: `parse_cue`/`parse_cue_bytes` are called from `tests/cue_suite.rs` and `src/fuzz.rs` only. `sniff_format` distinguishes M3U/XSPF; CUE is not a remote-playlist format. `probe.rs:142-143` recognizes the `.cue` extension but nothing outside the playlists crate calls `probe()`. apps/cli does not depend on the playlists crate (Cargo.toml hunt: only streaming + fuzz do). Why it matters: the whole card-5.O2 feature (gapless album windows) is dead code in the shipped product; the CUE matrix presence is zero (no `cue` feature row in docs/feature-matrix.toml — the only "cues" hits are Matroska/WebVTT cues). What correct looks like: either a CLI `playlist`/`cue` lane that opens sheets, or an honest UNREACHABLE marker in docs. Confidence: HIGH — Verification: ran rg over crates/ and apps/.
(Severity S2 not S0: no doc/matrix row claims shipped CUE support, so no false claim — just unwired code.)

F-002 [S4]
Where: crates/playlists/src/cue.rs:512-527 (REM) with 571-574 (TRACK close-on-open)
Code:
```
                let pair = (key.to_string(), unquote(value).to_string());
                if let Some(t) = &mut open {
                    t.rem.push(pair);
                } else {
                    sheet.rem.push(pair);
                }
```
What is wrong: after a malformed `TRACK` line (which closes the previous track and leaves `open = None`), subsequent `REM`/`TITLE`/`PERFORMER`/`SONGWRITER` lines that are textually inside track scope are attributed to the session (`sheet.rem`, session title/performer overwritten). Scope attribution is line-order-dependent on the *validity* of an earlier line, not on the sheet's textual structure. Blast radius: metadata mis-attribution on damaged sheets only. What correct looks like: a "track scope pending" state that swallows track-scoped commands until a valid TRACK opens. Confidence: MEDIUM — behavior read from code; no test pins it either way.
Verification: read.

F-003 [S4]
Where: crates/playlists/src/cue.rs:921-927
Code:
```
    if let Some(after) = body.strip_prefix('"') {
        let (name, tail) = match after.find('"') {
            Some(i) => (&after[..i], &after[i + 1..]),
            None => (after, ""),
        };
```
What is wrong: `FILE "" WAVE` yields an empty file name with no warning; an unclosed quote consumes the whole rest of the line as the name with no warning (documented at L917, but a `BadPregap`-style typed note would match the module's own damage-note discipline). Why it matters: a downstream opener receives `files[0].name == ""` and must refuse elsewhere. Confidence: HIGH — Verification: read (no test covers the empty-name shape; `quoted_unquoted_and_verbatim_values` covers well-formed shapes).

F-004 [S4]
Where: crates/playlists/src/cue.rs:762
Code:
```
                match &mut open {
                    Some(t) => t.isrc = Some(body.to_string()),
```
What is wrong: ISRC is the only metadata command not passed through `unquote` — a quoted ISRC keeps its quotes while TITLE/PERFORMER/SONGWRITER lose them. Inconsistent, harmless for spec-conformant sheets (ISRC is unquoted in the wild). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| frames_to_ns | 109-114 | REAL | — | round-to-nearest correct (hand-verified: 1e9%75=25 ⇒ fractions 0/⅓/⅔, +37 never miscarries); saturating mul |
| CueFileKind::from_token | 148-159 | REAL | — | unknown kept verbatim |
| CueTrackType::from_token | 199-211 | REAL | — | full CDRWIN data-mode set |
| CueFlag::from_token | 227-236 | REAL | — | 4 tokens, None otherwise |
| CueSheet::track_windows | 334-336 | REAL | — | slice accessor |
| CueSheet::file_of | 340-342 | REAL | — | bounds-checked |
| TimeContext::Display | 358-365 | REAL | — | redaction-safe context strings |
| parse_cue | 478-840 | REAL | — | two-pass: line machine + window arithmetic; BOM strip; CRLF; MAX_LINE_BYTES=64 KiB fatal (m3u.rs:100); no unwrap/index panics found; byte-split only on ASCII '\n' (char-boundary safe) |
| parse_cue_bytes | 844-847 | REAL | — | UTF-8 typed refusal |
| finalize_track | 869-892 | REAL | — | first INDEX 01 wins; MissingIndex01 damage note + skip |
| split_first_token | 896-901 | REAL | — | Unicode-whitespace split |
| unquote | 907-913 | REAL | — | whole-value quote pair only; boundary-safe slicing |
| parse_file_spec | 920-940 | REAL | S4 | F-003 empty/unclosed-quote names |
| parse_time | 945-954 | REAL | — | exact grammar; max 449,999 frames, no overflow |
| window arithmetic (inline, 781-836) | 781-836 | REAL | — | pregap INDEX-00↔01 with BadPregap on cross-file/backwards; NonMonotonicIndex empties window to Some(0); gapless_join same-file non-overrun; O(n) |

## 5. Test honesty (tests/cue_suite.rs — 20 tests, all ran green)
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ac1_single_file_gapless_windows | YES | exact ns values vs hand-truth table in comments |
| ac2_multi_file_windows_open_ended | YES | per-file open-endedness |
| ac3_pregap_postgap_arithmetic | YES | in-file vs synthetic distinction |
| bad_times / more_bad_time_shapes / pregap_postgap_bad_times_and_contexts | YES | every malformed shape typed |
| bom_crlf_lowercase_and_indentation | YES | lexical tolerance |
| track_without_file_and_outside_track_commands | YES | scope warnings |
| malformed_numbers_and_missing_arguments | YES | arg validation |
| unknown_command_and_flags | YES | unknown token handling |
| non_monotonic_index_empties_window | YES | Some(0) + warning |
| bad_pregap_arithmetic_is_typed | YES | cross-file INDEX 00 |
| quoted_unquoted_and_verbatim_values | YES | quoting rules |
| empty_and_minimal_inputs | YES | empty sheet OK |
| garbage_bytes_are_typed_invalid_utf8 | YES | typed error, no panic |
| line_cap_is_fatal_and_redacted | YES | 64 KiB fatal |
| lone_cr_is_one_weird_line | YES | documents CR-only non-support |
| large_valid_sheet_under_200kb | YES | 4600 tracks, joins, last open-ended |
| fuzz_lite_random_bytes_10k_no_panic | YES | structural invariants on every Ok |
| fuzz_lite_mutations_of_valid_sheet_5k | YES | mutation survival + invariants |
(plus doctest L72-97 in-file: real API walkthrough)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "No panics on any byte input — pinned by the fuzz-lite suite" | cue.rs:68 | REAL | fuzz-lite tests ran green (my run) |
| "error < 1 ns" round-to-nearest | cue.rs:51 | REAL | hand-verified remainder math |
| "The last track of each FILE has duration_ns: None" | cue.rs:40-42 | REAL | window arithmetic L812-822; ac2 test |
| "An unquoted name is accepted by taking the last whitespace-delimited token as the type" | cue.rs:13-14 | REAL | parse_file_spec L929-938 |

## 7. Dead / duplicated / unreachable code
Whole module unreachable from shipped entry points (F-001). No internal dead code found. No duplication (M3U's line discipline is shared via `MAX_LINE_BYTES` import).

## 8. Dependencies & flags
No feature gates. Crate-internal deps: error, m3u (MAX_LINE_BYTES only). No unsafe. External: thiserror for CueWarning Display.

## 9. Open questions for the next agent
- Will the Phase-4 playlist model (apps/cli/src/dispatch.rs:192 refuses `playlist.*` commands as "a Phase-4 card") ever land? Until then this parser is a library without a consumer.
