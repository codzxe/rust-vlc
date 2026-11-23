# AUDIT: crates/subtitles/src/srt/parser.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-23 |
| Lines / functions | 567 / 24 (14 prod incl. 2 entry points, 10 tests) |
| File verdict | UNREACHABLE (code is REAL — executed directly via compiled probe — but no shipped entry point links it) |
| Reachable from shipped entry point? | NO — `subtitles` crate is a dependency of no app/engine crate (only fuzz/Cargo.toml:50); shipped CLI drops subtitle frames (apps/cli/src/pipeline_host.rs:1237-1238) and deselects subrip tracks at open (apps/cli/src/containers/mkv.rs:108-110) |
| Findings | S0: 1 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | cargo test -p subtitles --lib timing_tolerances (exit 0, 1 passed); rustc probe vs prebuilt rlib: audit/runs/probe_srt.rs + probe_srt2.rs (exit 0, outputs below); sed quote verification (exit 0) |

## 1. One-paragraph verdict
This is the best-code-I've-audited-in-this-domain and the parser itself is genuine: I compiled a
probe against the real rlib and fed it the committed malformed fixtures and my own hostile inputs —
BOM variants decode, mutations degrade into a capped damage ledger, cues survive, no panics, and
the timestamp math is checked/saturating with no overflow path I could find. But it is a parser
without a product: nothing shipped links it, the CLI explicitly no-ops `Frame::Subtitle` and
deselects subrip tracks at open, and the command layer answers "the subtitle path is a Phase-5
card" — while the feature matrix declares subtitle-srt "implemented" on windows, linux and macos.
Two real parser defects survive the hostile pass: a module-doc claim about `,5` semantics is the
opposite of the code, and a numeric-only cue text line before a run-together cue silently drops
that cue while recording a mislabeled damage event.

## 2. What actually works (verified)
— `cargo test -p subtitles --lib timing_tolerances` → `test srt::parser::tests::timing_tolerances ... ok` (exit 0, 0.02s, prebuilt).
— probe (audit/runs/probe_srt, compiled vs libsubtitles rlib, exit 0):
   out1.srt → 2 cues clean (Styled bold/italic runs); xff3_120.srt → 3 cues + Windows1252Fallback;
   flip2_024.srt → 2 cues + BadTimestamp@2; flip2_112.srt → 2 cues + GarbageBlock@9;
   trunc2_064.srt → 1 cue + BadTimestamp@6; `,5` → 1005000µs; start-side junk → cue dropped;
   reversed-at-resync → first cue kept, second dropped + ReversedInterval; double UTF-8 BOM → clean parse.
— probe2 (probe_srt2, exit 0): overlapping cues preserved clean (2 cues, spans (1s,5s)/(2s,3s));
   duplicate ids/times preserved; 75 minutes → 4_500_000_000µs.
— Baseline: 10/10 parser unit tests + 6 canonical-differential + 4 fuzz-survival tests pass
   (audit/runs/baseline/cargo-test-workspace.txt:5949-5958, 6097-6120).

## 3. Findings
F-001 [S0]
Where: docs/feature-matrix.toml:891-902 vs apps/cli/src/pipeline_host.rs:1237-1238, apps/cli/src/dispatch.rs:197-198
Code: (verbatim)
```
status = { windows = "implemented", linux = "implemented", macos = "implemented", web = "not_started" }
evidence = "crates/subtitles/src/srt/** (card 5.M1; record docs/formats/sub-srt.md, 5.Q2-V: 6/6 ffmpeg 7.1.5 differential incl. UTF-16 both endians, 105/0 at card, 230/0 current tree)"
```
vs (pipeline_host.rs)
```
            // Subtitle/data limbs are not wired in the first vertical.
            Frame::Subtitle(_) => Ok(()),
```
What is wrong: the matrix declares SRT "implemented" on three platforms. The shipped player on
every platform: (a) links no `subtitles` crate (only fuzz/Cargo.toml:50 does — rg over every
Cargo.toml in the workspace); (b) drops every subtitle frame as a no-op (pipeline_host.rs:1238);
(c) deselects subrip tracks at MKV open (containers/mkv.rs:108-110 "Subtitles deselect at open");
(d) answers `subtitle.*` commands with "the subtitle path is a Phase-5 card" (dispatch.rs:197-198).
The row's own semantics include platform wiring (web = "not_started" though the crate is std-only
and platform-independent) — so "implemented" is a product-level claim that shipped code
contradicts. Why it matters: a user reading the matrix believes lumen renders SRT; it renders
nothing. What correct looks like: status "partial"/"library-only" until an engine wiring card
lands. Confidence: HIGH — Verification: ran rg over Cargo.tomls + read CLI code; matrix-row
semantics ruling belongs to the doc auditor (flagged).
F-002 [S3]
Where: crates/subtitles/src/srt/parser.rs:20-21 vs 358-360 (and the test at 449-452)
Code: (verbatim, module doc)
```
//!   `.` is accepted for the `,` millisecond separator; the
//!   millisecond field may be 1–3 digits (decimal-fraction scaling:
//!   `,5` = 500 ms) or absent; whitespace around `-->` is optional;
```
vs (verbatim, code comment)
```
    // Field semantics, ffmpeg/VLC `sscanf("%3d")` style: the digits
    // ARE the milliseconds (`,5` = 5 ms), 1–3 digits accepted; more
    // than 3 truncate to the first 3 with damage.
```
What is wrong: the module doc promises decimal-fraction scaling (`,5` = 500 ms); the code (and
its own inline comment, and the unit test, and my probe: `00:00:01.5 --> 00:00:02,25` →
start=1_005_000µs) implements digits-are-milliseconds (`,5` = 5 ms). The doc is false; the code
matches ffmpeg/VLC. Why it matters: anyone implementing a writer or a differential oracle from
the module doc will be off by 100× on 1–2-digit ms fields. What correct looks like: fix the doc
sentence. Confidence: HIGH — Verification: executed (probe_srt) + test run.
F-003 [S3]
Where: crates/subtitles/src/srt/parser.rs:231-243
Code: (verbatim)
```
    let (text_end, resync) = if let Resync::At(at) = resync {
        if at > tpos + 1 && is_index_like(lines[at - 1].1) {
            (at - 1, Resync::At(at - 1))
        } else {
            (at, Resync::At(at))
        }
```
What is wrong: when a cue's ONLY text line is numeric (e.g. a countdown "3") and the next cue is
run into it (missing blank line — the exact malformation this parser advertises resync for), the
numeric text is stolen as the next cue's index; `text_end <= tpos + 1` then records EmptyCueText
and the FIRST cue is silently dropped. Additionally `finish_block` (line 156) records
StrayLinesBeforeTiming for that same adopted index line — a damage event mislabeling a line the
parser itself consumed as an index. Executed: `"1\n00:00:01,000 --> 00:00:02,000\n2\n00:00:03,000
--> 00:00:04,000\nsecond\n"` → 1 cue, damages `[EmptyCueText, StrayLinesBeforeTiming]`. Why it
matters: data loss on plausible input under the parser's flagship tolerance scenario. What
correct looks like: adopt the numeric line as index only when at least one non-index text line
remains; never record StrayLinesBeforeTiming for an adopted index. Confidence: HIGH —
Verification: executed (probe_srt2).
F-004 [S4]
Where: crates/subtitles/src/srt/parser.rs:301-307
Code: (verbatim)
```
    // Start timestamp: the last whitespace-delimited token of the left
    // side (tolerates glued `-->` and stray prefixes).
    let s_trim = start_side.trim();
    let s_tok = s_trim.split_ascii_whitespace().next_back().unwrap_or("");
    if s_tok.len() < s_trim.len() {
        sink.record(SrtDamageKind::TrailingJunk, line_no);
    }
```
What is wrong: taking the LAST token tolerates prefix junk but kills the cue on suffix junk
(`00:00:01,000 junk --> ...` → s_tok = "junk" → BadTimestamp, cue dropped; executed: cues=0,
damages `[TrailingJunk, BadTimestamp]`). The recorded TrailingJunk event (a "tolerated" kind)
fires before the fatal parse failure and is also the name used for prefix junk — the ledger entry
misleads about both position and outcome. Rare input; low blast radius. Confidence: HIGH —
Verification: executed (probe_srt).
F-005 [S4]
Where: crates/subtitles/src/srt/parser.rs:34-36 vs 244-248, 147
Code: (verbatim)
```
//! Everything is one pass over `str::split('\n')` slices (D-1/D-3:
//! zero intermediate line vectors beyond the current block, no
//! regex); the cue `Vec` grows only with completed cues.
```
vs
```
    let joined = lines[tpos + 1..text_end]
        .iter()
        .map(|(_, l)| *l)
        .collect::<Vec<&str>>()
        .join("\n");
```
What is wrong: each cue's text lines ARE collected into an intermediate `Vec<&str>` per cue, so
"zero intermediate line vectors beyond the current block" is overstated; and `finish_block`'s
`std::mem::take(block)` (line 147) discards the block Vec's capacity every block, so each block
reallocates from empty. Both are plan-time nits, not per-frame costs. Confidence: HIGH —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SrtDocument::is_clean | 63-66 | REAL | — | correct conjunction |
| parse | 75-88 | REAL | F-001* | byte entry: decode→normalize→walk; executed on 5 fixtures |
| parse_str | 92-104 | REAL | — | no BOM strip (documented: caller-decoded); executed |
| walk | 110-124 | REAL | — | blank-line block split; final-block flush correct |
| next_synthetic | 128-132 | REAL | — | u64::MAX downwards, saturating; dup-0 only after 2^64 cues |
| finish_block | 141-161 | REAL | F-003 | loop resync, offset strictly increases (at ≥ tpos+1) — no infinite loop (verified by reasoning) |
| parse_block | 173-255 | REAL | F-003 | index/timing/text split; index-adoption heuristic steals numeric text |
| resync_after | 261-268 | REAL | — | requires looks_like_timing — literal `-->` in text stays text (executed) |
| looks_like_timing | 272-274 | REAL | — | probe re-parses without recording — double parse per candidate (nit) |
| is_index_like | 277-280 | REAL | — | |
| parse_index | 283-288 | REAL | — | digits-only + u64 fit |
| Timing (struct) | 291-294 | REAL | — | |
| parse_timing | 299-332 | REAL | F-004 | arrow split, last-token start / first-token end, X1/Y1 positional junk |
| parse_time_token | 336-386 | REAL | F-002 | checked_mul/add all the way; ×1000 with i64 saturation + damage; no overflow path found |
| tests ×10 | 400-566 | REAL | — | see §5 |
(*) F-001 is the crate-wiring/matrix finding; the function itself is correct.

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| canonical_two_cue_document_parses_clean | YES | exact ids, µs times, Plain text |
| crlf_and_blank_whitespace_lines_separate_blocks | YES | 2 cues + no-trailing-newline flush |
| index_line_is_optional_and_multiline_text_is_verbatim | YES | synthetic ids MAX/MAX-1, indentation kept |
| timing_tolerances | YES | asserts µs values for `.`-sep, glued arrow, 1-digit ms, h>23, m>59 — I re-ran it (exit 0) |
| trailing_junk_and_position_payloads_are_tolerated | YES | exact damage kinds + line numbers |
| malformed_blocks_are_skipped_without_aborting | YES | 2 cues survive garbage + stray lines |
| reversed_and_empty_and_bad_indexes | YES | 0 cues, exact damage vectors |
| saturation_and_long_milliseconds | YES | i64::MAX clamp + LongMilliseconds |
| byte_entry_decodes_utf16_and_reports_encoding | YES | BOM-built UTF-16LE bytes decode; empty/blank inputs |
| stray_text_before_timing_is_kept_as_text | YES | resync adopts index; literal `-->` stays text — misses the numeric-text drop (F-003), the one hole |
Honest suite; the gap is the untested numeric-text edge, not tautology.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Parse and render SubRip subtitles … status implemented (win/linux/macos)" | docs/feature-matrix.toml:891-902 | FALSE at product level | CLI drops Frame::Subtitle (pipeline_host.rs:1238); crate unlinked (rg); tracks deselected (mkv.rs:108-110) — F-001 |
| "`,5` = 500 ms (decimal-fraction scaling)" | parser.rs:20-21 | FALSE | executed: `,5` → 5 ms (F-002) |
| "Construction never fails and never panics" | parser.rs:47-48 | REAL | probe + 2000-iteration mutation loop baseline + my probes; no panic |
| "hours may exceed 23 … values beyond the i64 microsecond domain saturate with damage" | parser.rs:22-24 | REAL | unit test + probe (86_399_900_000µs rollover fixture cue) |
| "zero intermediate line vectors beyond the current block" | parser.rs:34-36 | SHALLOW | per-cue Vec<&str> collect (F-005) |
| "The entry point for the demux/engine seam" | parser.rs:69-71 | FALSE | no demux/engine caller exists — seam unwired (F-001) |

## 7. Dead / duplicated / unreachable code
No dead functions inside the file; the file itself is unreachable from shipped entry points
(F-001). `parse_str` is exercised only by tests and my probe — no in-crate caller besides tests
(rg: only srt/mod.rs re-export + tests + fuzz harness use the module).

## 8. Dependencies & flags
model::cue/position/time, srt::damage (256-event cap), srt::encoding (BOM matrix), srt::tags —
all in-crate. Std-only. No features, no unsafe. Fuzz target fuzz/fuzz_targets/fuzz_sub_srt.rs
delegates to srt::fuzz::fuzz_sub_srt (nightly-libfuzzer wiring admitted unbuilt in its header).

## 9. Open questions for the next agent
- Doc auditor: rule on feature-matrix status semantics for subtitle-* rows (F-001) — 17
  subtitle-format rows likely share the same product-vs-library inflation.
- srt/render.rs and srt/fuzz.rs were context-only here; their placement math is unaudited by me.
