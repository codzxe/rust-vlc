# AUDIT: crates/subtitles/tests/ass_ffmpeg_ac.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-23 |
| Lines / functions | 126 / 4 tests |
| File verdict | REAL (fixtures verified genuine ffmpeg output by byte-exact regeneration; one FALSE provenance claim about HOW the unicode fixture's tags got there) |
| Reachable from shipped entry point? | N/A (integration test; baseline audit/runs/baseline/cargo-test-workspace.txt:6012-6020 — 4 passed, 0 failed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | ffmpeg regenerations ×4 variants (exit 0; 3 byte-exact matches incl. the disproof run); diff; rg worklog; sed quotes |

## 1. One-paragraph verdict
A tight acceptance suite over three committed ffmpeg-generated ASS fixtures, and the fixtures are REAL: I regenerated `ac_basic.ass` and `ac_sixcues.ass` byte-exact from reconstructed SRTs with the sandbox's ffmpeg 7.1.5, and `ac_unicode.ass` byte-exact as well — but only after disproving the doc's recipe. The doc claims the source SRT carried "literal `{\b1}` tags that survive SRT→ASS conversion"; I executed that exact recipe and ffmpeg STRIPS them (output: `Bold html & ampersand`). The committed fixture's `{\b1}html{\b0}` is what ffmpeg synthesizes from `<b>html</b>` HTML tags in the SRT. The fixture is genuine ffmpeg output and the test's G-2 assertions (styled `html` run, `applied_override_tags == 2`, `dropped_tags["b"] == None`) are correct and pass — but the recorded provenance mechanism is false, so anyone replaying generation per the doc produces a different, tag-free fixture and the AC silently stops covering the override-tag surface.

## 2. What actually works (verified)
- Regeneration (ffmpeg 7.1.5-0+deb13u1, `-c:s ass`), all exit 0:
```
ac_basic: BYTE-EXACT MATCH          (SRT: multi-line cue)
ac_sixcues: BYTE-EXACT MATCH        (SRT: A–F 2 s ladder)
ac_unicode (with <b> + bare &): BYTE-EXACT MATCH
```
- Disproof run (SRT carrying literal `{\b1}html{\b0}` per the doc's recipe):
```
< Dialogue: 0,0:00:03.10,0:00:04.90,Default,,0,0,0,,Bold html & ampersand
> Dialogue: 0,0:00:03.10,0:00:04.90,Default,,0,0,0,,Bold {\b1}html{\b0} & ampersand
```
- Baseline: 4/4 tests ok (cargo-test-workspace.txt:6012-6020).
- Expectation arithmetic re-derived: `0:01:02.75` → 62_750_000 µs ✓; `0:10:03.20` → 603_200_000 µs ✓; `\N` → 1 hard newline ✓; alignment 2 → bottom-center ✓ (cross-checked against parser.rs `map_alignment`).

## 3. Findings
F-001 [S3]
Where: crates/subtitles/tests/ass_ffmpeg_ac.rs:8-11
Code:
```
//! the generation commands are recorded in the worklog and can be
//! replayed: the three SRT sources each had different content
//! (basic multi-line, unicode + literal `{\b1}` tags that survive
//! SRT→ASS conversion, and a six-cue even-spaced ladder).
```
What is wrong: the "literal `{\b1}` tags survive SRT→ASS conversion" claim is FALSE — executed disproof above: ffmpeg's subrip decoder strips literal `{\b1}`/`{\b0}` override blocks; the byte-exact source is `Bold <b>html</b> & ampersand` (ffmpeg synthesizes the ASS overrides from HTML font tags). The same false description is propagated in docs/worklog/archive-2026-10.md ("unicode+literal `{\b1}` passthrough"). Why it matters: this codebase sells honest evidence ledgers; a replay following the recorded recipe produces a tag-free fixture and this AC silently loses its override-tag coverage (the strongest assertion in the suite — the 3-segment styled run). What correct looks like: doc says "unicode + `<b>html</b>` HTML tags, which ffmpeg converts to literal `{\b1}…{\b0}` overrides". Confidence: HIGH — Verification: ran (two ffmpeg runs, byte-level diff). Blast radius: documentation/replay only — the committed fixture and the assertions are correct.

F-002 [S4]
Where: crates/subtitles/tests/ass_ffmpeg_ac.rs:8 ("generation commands are recorded in the worklog")
What is wrong: no literal generation command lines exist in the worklog archive (rg over docs/worklog/ finds only prose: "3 ASS generated with real [ffmpeg] from 3 hand-written SRTs" — no `ffmpeg -i … -c:s ass` lines, no SRT contents). "Recorded and replayable" overstates: I had to reverse-engineer the SRT sources (2 attempts) to achieve byte-exact replay. Confidence: HIGH — Verification: rg (no command lines found).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ffmpeg_header_and_style_parse_clean` | 24-44 | REAL | — | header/style/margins/alignment + zero damage and zero dropped tags |
| `ffmpeg_basic_three_cues_times_and_text` | 46-71 | REAL | — | exact µs ×3, dense ids, style inheritance, hard_newlines == 1 |
| `ffmpeg_unicode_and_tag_stripping` | 73-107 | REAL | — | the G-2 end-to-end pin: 3 styled segments, bold only on `html`, applied==2, dropped b==None |
| `ffmpeg_six_cue_ladder` | 109-126 | REAL | — | parametric ladder, plain-only via typed error on styled |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ffmpeg_header_and_style_parse_clean | YES | exact field equality incl. font_size_milli 16_000 and Color::WHITE |
| ffmpeg_basic_three_cues_times_and_text | YES | exact times/texts/ids + `hard_newlines` counter |
| ffmpeg_unicode_and_tag_stripping | YES | exact segment texts + bold flags + both tag counters (an engine that dropped `\b` fails `dropped_tags.get("b") == None`; one that ignored tags fails the Styled match) |
| ffmpeg_six_cue_ladder | YES | parametric times; styled payload → typed error |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "three ASS documents generated by real ffmpeg … 7.1.5-0+deb13u1" | L1-2 | REAL | all 3 fixtures byte-exact regenerated with sandbox ffmpeg 7.1.5-0+deb13u1 |
| "literal `{\b1}` tags … survive SRT→ASS conversion" | L10 | FALSE | executed disproof (F-001); tags are synthesized from `<b>` |
| "generation commands are recorded in the worklog and can be replayed" | L8 | SHALLOW | no command lines in worklog (F-002); replay achievable only by reverse-engineering |
| "centisecond tokens → µs, ×10 000 exact" | L3-4 | REAL | re-derived: 0:01:02.75 → 62_750_000 |

## 7. Dead / duplicated / unreachable code
None. (Crate-level unreachability from shipped apps applies — see parser.rs audit.)

## 8. Dependencies & flags
`subtitles` only; `include_str!` committed fixtures (790/864/893 bytes). No dev-deps needed here (no testkit). No features.

## 9. Open questions for the next agent
- docs/formats/sub-ass-ssa.md repeats the false `{\b1}` passthrough description (same archive source) — its owner should correct both together with F-001.
- The worklog's original generation note recorded the OLD expectation ("dropped_tags[`b`]=2") — the G-2 landing updated the test but not the worklog; harmless history, worth a correction marker.
