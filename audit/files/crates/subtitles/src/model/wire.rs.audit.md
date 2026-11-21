# AUDIT: crates/subtitles/src/model/wire.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-21 |
| Lines / functions | 517 / 21 (no in-file tests; exercised by tests/roundtrip_matrix.rs, tests/timing_edges.rs, tests/fuzz_survival.rs) |
| File verdict | UNREACHABLE (real, well-engineered code; production callers = zero — test/debug interchange only; crate unshipped) |
| Reachable from shipped entry point? | NO. rg `serialize_document\|parse_document\|write_document` across repo: definitions here + `model/mod.rs` re-export + `crates/subtitles/tests/{timing_edges,roundtrip_matrix,fuzz_survival}.rs` only. No demux/playback/CLI consumer; `subtitles` crate is a fuzz-only dependency. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `rg -n "wire::\|serialize_document\|parse_document\|write_document" crates/subtitles` (test-only callers) · read model/time.rs (CueInterval::new), model/style.rs (TextStyle fields), model/cue.rs (CueId/Cue const ctors) · read tests/roundtrip_matrix.rs (shape coverage) · baseline: `orchrun-test-workspace-nofailfast.log.txt` lines 5598-5666 (fuzz_survival 4/4, roundtrip_matrix 5/5, timing_edges 8/8 ok) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
A genuinely careful single-pass line-oriented serializer/parser with typed errors, a Copy header state machine I could not break (C/P/S/T ordering, unterminated styled payloads, escape edge cases, usize::MAX segment counts — all typed, no panics, lazy allocation as claimed), and a green acceptance matrix in the baseline. But its headline contract is now false: `TextStyle` grew the G-2/issue-#17 fields `strikeout`, `font_size_milli`, `font_name` (model/style.rs:107-117) and the wire format never serializes them — a styled cue carrying `\s`/`\fs`/`\fn` styling round-trips to a cue with those fields silently defaulted, so "cue → serialize → parse → equal" (module doc lines 1-2) is data loss, not equality. The 176-shape acceptance matrix never constructs those fields, so the suite is green precisely because it skips the fields that break. Smaller: the "each byte visited exactly once" claim fails on one error path, the serializer allocates throwaway Strings per cue, and a stray `T` line reports the wrong error.

## 2. What actually works (verified)
— Parser robustness + acceptance: `tests/fuzz_survival.rs` 4/4 (`mutated_documents_never_panic`…), `tests/roundtrip_matrix.rs` 5/5 (1584-cue matrix, byte stability), `tests/timing_edges.rs` 8/8 — all ok in `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:5598-5666`. State machine traced by hand: C-duplicate, P/S-without-C, T-without-S, S-then-P, T-overflow, dangling C at EOF, unterminated S at EOF, `S 0`, empty `P` — every arm returns a typed `WireError`; no unwrap/expect/index/panic path exists in the file (verified by reading every function).
— `build_cue`'s "structurally dead" Err arm (lines 510-515) is genuinely dead: `CueInterval::new` (model/time.rs:48-53) fails only on `end < start`, which `parse_header` (lines 372-378) already rejected.
— Lazy allocation claim holds: `S <huge>` allocates nothing (StyledPending.segments grows only per T line; EOF errors via `reject_unterminated`).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/model/wire.rs:1-2, 131-137, 460-464 (vs model/style.rs:107-117)
Code:
```rust
//! The stable wire form (card 5.L2 acceptance): cue → serialize →
//! parse → equal.
```
```rust
                out.push(if seg.style.bold { '1' } else { '0' });
                out.push(if seg.style.italic { '1' } else { '0' });
                out.push(if seg.style.underline { '1' } else { '0' });
```
```rust
    let style = TextStyle::new(
        flag_bytes[0] == b'1',
        flag_bytes[1] == b'1',
        flag_bytes[2] == b'1',
        color,
    );
```
What is wrong: The wire T-record carries only bold/italic/underline + color. `TextStyle` (model/style.rs:99-118) has three more fields — `strikeout`, `font_size_milli`, `font_name` — added by issue #17 G-2 (\s, \fs, \fn). `write_cue` drops them; `parse_segment` reconstructs via `TextStyle::new(...)` which defaults all three (`strikeout: false, font_size_milli: None, font_name: None`, style.rs:124-134). Round-trip equality — the module's stated acceptance — therefore FAILS for every cue whose segments use G-2 styling; the failure mode is silent data loss (no error, no counter).
Why it matters: This is the model's "canonical interchange form for tests and debug dumps" (line 52-54): any debug dump of an ASS-parsed cue (ass/tags.rs DOES produce font_name/font_size_milli styled runs, e.g. tags.rs:590-599) loses its font information through the wire form, and any test asserting round-trip equality over real parser output would fail — which is exactly why the acceptance matrix avoids those fields (tests/roundtrip_matrix.rs lines 61-98: every styled shape is built with `TextStyle::new(b,i,u,color)` only). The green suite masks the hole instead of exposing it.
What correct looks like: Extend the T-record (`T <b><i><u><s> <color> <fs-milli> <escaped-family> <text>` or a versioned `LUMEN-CUES 2`), or make serialize reject (typed) cues carrying fields the format cannot represent — silent dropping is the one option that is never right.
Confidence: HIGH — Verification: read (all four code sites) + shape list of the acceptance test read; no run possible in sandbox (rule 8) but the field mismatch is static.

### F-002 [S4]
Where: crates/subtitles/src/model/wire.rs:39-42 vs 288-294, 324-326
Code:
```rust
//! * **Single-pass parser** — [`parse_document`] walks the input once
//!   via `str::split('\n')` (zero-copy `&str` slices, no line vector,
//!   no regex, no intermediate collections); each source byte is
//!   visited exactly once.
```
```rust
    if header.is_some() {
        return Err(WireError::MissingPayload {
            line_no: line_count(src),
        });
    }
```
What is wrong: On the dangling-C-header-at-EOF error path, `line_count(src)` re-walks the entire input (`src.split('\n').count()`), a second pass. "Each source byte is visited exactly once" is false on that path.
Why it matters: Engineering-note claims are the module's sales pitch; this one has a counterexample. (Happy path IS single-pass — verified.)
What correct looks like: Track the last line number in the existing enumerate loop and use it; or scope the claim to "the accept path".
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/subtitles/src/model/wire.rs:106-114
Code:
```rust
    let line_tok = match p.line {
        None => "-".to_string(),
        Some(LinePlacement::Percent(v)) => format!("p{}", v.get()),
        Some(LinePlacement::LineNumber(n)) => format!("n{n}"),
    };
```
What is wrong: The serializer — under the module's "speed + memory care" header — heap-allocates two throwaway `String`s per cue (`line_tok`, `pos_tok`, plus `"-".to_string()` twice) before a `writeln!` that could consume `&str`/integers pushed directly into `out`.
Why it matters: 2-4 avoidable allocations per cue on the only serialization path (baseline perf smoke: 15,840 cues; the waste scales with cue count). Cosmetic at current volumes, but the module explicitly claims memory care.
What correct looks like: `out.push('p'); write int into out` (or `write!` with the value inline), no intermediate strings.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/subtitles/src/model/wire.rs:256-263
Code:
```rust
            b'T' => {
                let seg = parse_segment(line, line_no)?;
                let Some(st) = styled.as_mut() else {
                    return Err(WireError::MissingField {
                        line_no,
                        field: "S line before T",
                    });
                };
```
What is wrong: `parse_segment` runs BEFORE the styled-pending check. A stray malformed `T` line outside any `S` block reports the segment-level error (BadValue/BadColor/BadEscape) instead of the structural one ("S line before T").
Why it matters: Error-precedence inversion misleads whoever debugs a truncated document: the line's real crime is being orphaned, not its flag spelling.
What correct looks like: Check `styled` first, parse the segment second.
Confidence: HIGH — Verification: read + hand-trace.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `MAGIC` const | 67 | REAL | — | "LUMEN-CUES 1", lint-allow comment present |
| `RawHeader` (struct) | 72-81 | REAL | — | Copy, all fields validated pre-heap — claim holds |
| `serialize_document` | 88-92 | REAL | — | 48 B/cue capacity hint |
| `write_document` | 95-101 | REAL | — | Buffer-reuse variant; used by roundtrip test |
| `write_cue` | 103-149 | REAL | S2/S4 | Drops G-2 style fields (F-001); throwaway Strings (F-003) |
| `escape_into` | 152-162 | REAL | — | \, \n, \r, \t — sufficient to keep records one line |
| `unescape_into` | 166-183 | REAL | — | Trailing backslash → BadEscape; multi-byte after `\` → BadEscape; slicing on ASCII boundaries — safe |
| `parse_document` | 191-296 | REAL | S4 | State machine traced clean; error-path second pass (F-002); blank-line tolerance fine |
| `StyledPending` (struct) | 299-305 | REAL | — | `expected` kept for error detail |
| `reject_unterminated` | 309-318 | REAL | — | Reports S line + expected/seen — correct |
| `strip_cr` | 320-322 | REAL | — | One CR; \r\r\n leaves one CR (tolerant, harmless) |
| `line_count` | 324-326 | REAL | S4 | Second pass (F-002) |
| `payload_body` | 329-343 | REAL | — | Bare tag → empty body; round-trips "P " and "P" |
| `parse_header` | 349-391 | REAL | — | EndBeforeStart before alignment checks; ExtraFields enforced; negative µs legal by design (time.rs docs) |
| `parse_line_spec` | 393-415 | REAL | — | `-`/`p<0..100>`/`n<i64>`; i64::MAX line number round-trips (matrix shape) |
| `parse_percent_opt` | 418-423 | REAL | — | transpose idiom, correct |
| `parse_percent` | 425-438 | REAL | — | 0..=100 domain; double-check via Percent::new kept typed (not masked) |
| `parse_segment` | 440-469 | REAL | S2 | TextStyle::new defaults G-2 fields (F-001); flags/color/text parse solid; no-space color-only line handled |
| `split_first_token` | 471-480 | REAL | — | Correct |
| `field_str` / `parse_field` | 482-500 | REAL | — | Generic; BadInt name accurate for all uses (ints only) |
| `build_cue` | 504-517 | REAL | — | Dead Err arm confirmed dead (CueInterval::new fails only on end<start, pre-rejected); EMPTY fallback never taken |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| roundtrip_matrix `matrix_document_roundtrip` / `per_cue_roundtrip` | YES for the fields it covers (would catch a dropped underline flag, escape corruption, percent/line loss — concrete `assert_eq!(parsed, cues)`) — but NO for strikeout/font_size_milli/font_name: no shape constructs them, so F-001 is invisible to the acceptance suite |
| `serialization_is_deterministic` | YES | Two buffer paths byte-compared |
| `empty_document_roundtrips` | YES (weakly) | Pins magic bytes |
| timing_edges 8 tests | YES | Concrete error-variant asserts at the wire (EndBeforeStart, PercentOutOfRange, CRLF, zero-duration bytes) |
| fuzz_survival `mutated_documents_never_panic` + re-serialization check | Panic-check: YES. Re-serialization equality: NO for F-001 — parse already dropped the fields, so parse→serialize→parse is trivially stable over the lossy subset (tautology w.r.t. the dropped fields) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "cue → serialize → parse → equal" (card 5.L2 acceptance) | wire.rs:1-2, lib.rs:67-68 | FALSE for G-2-styled cues | F-001: three TextStyle fields unserialized; acceptance matrix skips them |
| "T flags are three 0/1 characters (bold, italic, underline)" | wire.rs:28-29 | REAL (format doc) but documents the gap | Same code; format frozen at the pre-G-2 subset |
| "Single-pass… each source byte visited exactly once" | wire.rs:39-42 | SHALLOW | F-002: line_count re-walk on the dangling-C error path |
| "Lazy allocation… grows with completed cues only" | wire.rs:43-46 | REAL | `S <usize::MAX>` allocates nothing until T lines; verified by read |
| "Stable — re-serializing a parsed document reproduces the accepted bytes" | wire.rs:47-50 | REAL (subset) | Baseline roundtrip_matrix byte-stability assert green; holds for everything the format CAN represent |
| wire = "canonical interchange form for tests and debug dumps" [PROPOSAL] | wire.rs:52-54 | REAL | Zero production callers (rg) |

## 7. Dead / duplicated / unreachable code
- The whole module is production-dead: test/debug interchange with no shipped consumer (crate reachable only via fuzz targets).
- `WireError` variants all reachable (traced each construction site).
- No duplication found; `parse_field`/`field_str` are appropriately factored.

## 8. Dependencies & flags
- std-only (`std::fmt::Write`); no feature gates; no unsafe; media-types not used here. Lint-allow comment on MAGIC follows the repo's lint-literals policy (baseline #13 shows that gate is real).

## 9. Open questions for the next agent
- model/mod.rs + style.rs owners: is `TextStyle`'s G-2 growth (commit per matrix row subtitle-ass-ssa notes, ba3dd0f/1bd5519) accompanied by any plan item for `LUMEN-CUES 2`? The matrix row "ass-ssa" evidence cites "267/0 lib tests incl. per-tag suites" — none of those can round-trip through this wire form today.
- If the wire form is ever used for golden files, F-001 will silently corrupt them — version bump before that happens.
