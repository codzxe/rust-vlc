# AUDIT: crates/playlists/src/cue.rs#part2 (lines 478-954)
| Field | Value |
|---|---|
| Auditor | W10-A01 |
| Date | 2026-11-21 |
| Chunk boundaries | part2 = 478 (`parse_cue`) through 954 (end of `parse_time`). Part1 (W10-A08) = 1-477 (types, `CueWarning` ends 474). Equal-split anchor 478-954 — matches the natural seam exactly (whole parse machinery is part2). |
| Lines / functions | 477 / 8 (7 prod functions + TrackBuilder struct) |
| File verdict | UNREACHABLE (real, baseline-tested parser; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. `parse_cue` exported (playlists/src/lib.rs:103); callers: `crates/playlists/src/fuzz.rs:51` (in-crate fuzz harness) and `crates/playlists/tests/cue_suite.rs` only. No app crate depends on `playlists` (rg over apps/*/Cargo.toml → nothing; apps/cli, apps/web, apps/desktop-tauri contain zero cue/playlist-open references). The CLI's `open` lanes are File (container probe) and http(s) — a `.cue` file would die in the container probe; no cue lane exists. `demux/src/mkv/cues.rs` is Matroska `parse_cues` — a different parser that merely shares the substring. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "^(pub |)fn…" cue.rs` (outline/split); `rg -rn "parse_cue"` across repo (callers: fuzz.rs + cue_suite.rs; mkv hits are `parse_cues` — different symbol); `rg "playlists"` in app Cargo.tomls (absent); `rg "m3u|playlist|cue"` in apps/cli demux_host + commands (absent); baseline log check: `Running tests/cue_suite.rs` at orchrun-test-workspace-nofailfast.log.txt:4311; `sed -n` on all quoted ranges (verbatim); read of the pinned FILE-split test (cue_suite.rs:484-511). |

## 1. One-paragraph verdict
The CUE parser is careful, spec-literate work — correct FILE-does-not-close-track semantics, first-INDEX-01-wins finalization, in-file vs synthetic pregap arithmetic, honest open-ended durations for last tracks, round-to-nearest frame→ns conversion (part1), and a degrade-to-typed-warnings philosophy with a single fatal (line cap). But two defects violate its own doctrine: an unquoted multi-word `FILE` body without a type token silently corrupts the filename (the last word is stolen as a bogus type, no warning anywhere), and the parser has no total-size or count bound — only the per-line 64 KiB cap — while its sibling `mpd.rs` caps total bytes at 8 MiB for exactly this reason. And none of it is reachable from any shipped binary: no app opens cue sheets; the parser serves tests and the fuzz harness only.

## 2. What actually works (verified)
— verified by baseline run `orchrun-test-workspace-nofailfast.log.txt:4311` (`Running tests/cue_suite.rs`, suite passed within the 1681-pass workspace run): the parser executes in-tree over the 14-test suite incl. hand-computed window arithmetic (ac1-ac3) and the 4600-track stress (per matrix evidence, plfmt-cue row).
— verified by read: `parse_time` enforces spec bounds (`mm ≤ 99`, `ss ≤ 59`, `ff ≤ 74`) and rejects 4-part times (945-953); `unquote` strips exactly one outer pair and keeps interiors verbatim (907-913, byte-safe at ASCII quote boundaries); the window arithmetic guards every subtraction (`ix0.frames <= start` at 792, `next.start_frames >= start` at 814) — no underflow paths found.

## 3. Findings

### F-001 [S3]
Where: crates/playlists/src/cue.rs:928-939
Code:
```
let tokens: Vec<&str> = body.split_whitespace().collect();
match tokens.len() {
    0 => None,
    1 => Some((tokens[0].to_string(), None)),
    _ => {
        let kind = tokens[tokens.len() - 1].to_string();
        let name = tokens[..tokens.len() - 1].join(" ");
        Some((name, Some(kind)))
    }
}
```
What is wrong: for an unquoted `FILE` body the parser splits from the right, assuming the last token is the type. When a hand-edited sheet omits the type token but the path has a space (`FILE My Music.wav`), the result is name=`"My"`, kind=`Some("Music.wav")` — the filename is silently corrupted, and `CueFileKind::from_token` maps the bogus token to `Other(...)` without any warning (147-159, part1). The in-tree test pins only the spec-compliant shape (`FILE my music file.wav WAVE`, cue_suite.rs:498-500) and asserts `warnings.is_empty()` — the corrupting shape is untested and unwarned.
Why it matters: a silent mis-parse — the exact thing this file's own comment promises never to do ("a typed finding, never a guess", line 789); a downstream cue consumer would open the wrong file with no diagnostic. Muted today by unreachability; this is the 5.O2 landing surface.
What correct looks like: when the last token is not a known type token AND the body has ≥2 tokens, emit a `CueWarning` (e.g. FileWithoutType) and keep the whole body as the name — the same degrade the quoted-missing-type path already takes (553-559).
Confidence: HIGH — Verification: read + hand-trace; test coverage inspected (484-511).

### F-002 [S3]
Where: crates/playlists/src/cue.rs:497-502 (the only size bound) vs crates/playlists/src/mpd.rs:132
Code:
```
if raw.len() > MAX_LINE_BYTES {
    return Err(ParseError::LineTooLong {
        line: line_no,
        length: raw.len(),
    });
}
```
What is wrong: the only fatal bound is the per-line 64 KiB cap. There is no total-input cap (mpd.rs: `MAX_MPD_BYTES = 8 MiB`) and no track/index/warning count cap. Every line allocates: unknown commands, garbage tokens, and REM pairs each push heap-allocated warnings/pairs — roughly 10-20× amplification of input size in allocations (each `CueWarning`/`String` pair costs tens of bytes per ~5-10 byte line).
Why it matters: a hostile 100 MB cue sheet (all lines under 64 KiB) drives >1 GB of allocations with no typed refusal — the unbounded-allocation-from-file-controlled-input class the sibling parsers in the same crate explicitly bound. Drift from the crate's own posture, not a shipped-path risk (unreachable).
What correct looks like: a `MAX_CUE_BYTES` fatal (match mpd.rs) or count caps (tracks/indexes/warnings) with a typed `TooLarge` error, matching the module's D-1 style.
Confidence: HIGH — Verification: read + rg for caps across the crate (only MAX_LINE_BYTES at m3u.rs:100 and MAX_XML_DEPTH at xspf.rs:66 found; cue has neither pattern).

### F-003 [S4]
Where: crates/playlists/src/cue.rs:761-762
Code:
```
match &mut open {
    Some(t) => t.isrc = Some(body.to_string()),
```
What is wrong: `ISRC` stores the raw body while `TITLE`/`PERFORMER`/`SONGWRITER` pass through `unquote` (739) and `CATALOG`/`CDTEXTFILE` too (535/545). A quoted `ISRC "XXYYYNNNNNNN"` keeps the quote characters in the model — inconsistent value normalization for the one field that is most often quoted by tagging tools.
Why it matters: consumers must remember which fields self-clean and which do not; model inconsistency.
What correct looks like: `unquote(body).to_string()` like its siblings.
Confidence: HIGH — Verification: read of all value-bearing arms.

### F-004 [S4]
Where: crates/playlists/src/cue.rs:713-716 (and 871, 812-819)
Code:
```
for tok in tokens {
    match CueFlag::from_token(tok) {
        Some(flag) => t.flags.push(flag),
```
What is wrong: three duplicate-tolerant paths emit no warning: `FLAGS DCP DCP` pushes the flag twice (713-716); a second `INDEX 01` in one track is silently ignored under first-wins (871: `.find(|ix| ix.number == 1)`); two tracks in one FILE with identical `INDEX 01` times produce a zero-duration window with no warning (812-819 warns only on strictly-lower starts).
Why it matters: damaged-but-parseable sheets lose the crate's signature typed diagnostics in exactly the shapes a hand editor produces; zero-duration windows are indistinguishable from deliberate zero-length tracks.
What correct looks like: duplicate-flag, duplicate-INDEX-01, and equal-start warnings (all cheap, all typed).
Confidence: HIGH — Verification: read; cue_suite has no duplicate-shape test.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_cue | 478-840 | REAL (defects) | S3/S4 | two-pass line machine + window arithmetic; all subtraction guards present; F-002 no total cap; BOM/CRLF/lowercase tolerated (pinned by cue_suite:338) |
| parse_cue_bytes | 844-847 | REAL | — | typed InvalidUtf8; fuzz-pinned claim matches fuzz.rs:51 |
| TrackBuilder | 852-865 | REAL | — | in-flight state; FILE does not close a track — spec-correct, documented |
| finalize_track | 869-892 | REAL | S4 | first INDEX 01 wins silently (F-004); MissingIndex01 typed otherwise |
| split_first_token | 896-901 | REAL | — | trivial correct |
| unquote | 907-913 | REAL | — | one outer pair, verbatim interior, byte-safe slicing |
| parse_file_spec | 920-940 | REAL (defect) | S3 | F-001 right-split corruption; quoted path correct incl. unterminated-quote degrade |
| parse_time | 945-954 | REAL | — | spec bounds, 3-part enforcement, `(mm·60+ss)·75+ff` correct; max ≈ 450k frames — no overflow downstream (frames_to_ns saturating, part1:109-112) |

## 5. Test honesty (tests/cue_suite.rs — for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| ac1_single_file_gapless_windows (35) | YES | hand-computed window truth incl. gapless joins |
| ac2_multi_file_windows_open_ended (126) | YES | open-ended last-track durations pinned |
| ac3_pregap_postgap_arithmetic (184) | YES | in-file vs synthetic pregap truth |
| bad_times/more_bad_time_shapes/pregap_postgap_bad_times (232/276/302) | YES | typed-refusal matrices per context |
| bom_crlf_lowercase_and_indentation (338) | YES | dialect tolerance |
| track_without_file_and_outside_track_commands (353) | YES | CommandOutsideTrack/TrackWithoutFile warnings |
| malformed_numbers_and_missing_arguments (379) | YES | track/index bounds (1-99, 0-99) |
| unknown_command_and_flags (421) | YES | UnknownCommand/UnknownFlag |
| non_monotonic_index_empties_window (446) | YES | NonMonotonicIndex → zero-duration pinned |
| bad_pregap_arithmetic_is_typed (468) | YES | cross-file/overrunning INDEX 00 typed |
| quoted_unquoted_and_verbatim_values (485) | YES for the shapes it covers | pins the good FILE shape + escapes-verbatim; does NOT cover F-001's type-less multi-word shape |
| empty_and_minimal_inputs / garbage_bytes (513/535) | YES | empty input, InvalidUtf8 |
Mutation check (reasoned): remove the `ff > 74` guard → more_bad_time_shapes fails; swap pregap arms in the second pass → ac3 fails. Suite is load-bearing; the one blind spot is F-001's shape.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Parses a CUE sheet. Fatal only for the D-1 line-cap violation; every other defect degrades to a typed warning" | cue.rs:476-477 | FALSE in two corners | F-001 (silent corruption, no warning), F-004 (duplicate shapes unwarned) |
| plfmt-cue row: "3 handcrafted AC sheets vs hand-computed frame->ns truth, zero-gap joins pinned exactly, 13 typed warnings, 4600-track stress" | feature-matrix.toml:1231 evidence | REAL (as library tests) / UNREACHABLE from shipped binary | cue_suite.rs test list matches; baseline :4311; no app caller (reachability above); the row's status should say the parser is unwired |
| module doc: "A line over MAX_LINE_BYTES is fatal" | cue.rs:66 | REAL | 497-502 verified |

## 7. Dead / duplicated / unreachable code
Whole file unreachable from shipped entry points (see header). No dead code within the file; every function is exercised by cue_suite.rs.

## 8. Dependencies & flags
std-only; reuses `MAX_LINE_BYTES` from crate::m3u (100). No feature gates. Compiled into the shipped binary only as a transitive dep of streaming→playlists; no call chain reaches it.

## 9. Open questions for the next agent
- Part1 owner (W10-A08): does any consumer of `CueFileKind::Other` exist that would notice F-001's bogus type? My rg says the model has no consumer at all outside tests.
- Wave-level: three of four playlist families (cue, mpd; m3u/xspf partially via remote_playlist — itself unwired) are compiled-but-unwired. Whoever owns the parity/matrix rollup should state this once at wave level rather than per-file.
