# AUDIT: crates/plugin-api/src/subtitle.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-24 |
| Lines / functions | 231 / 8 (2 traits, 1 enum, 1 fn, 4 test items incl. toy parser) |
| File verdict | REAL (contract compiles, tested against a toy impl) but UNADOPTED — no production implementor or consumer |
| Reachable from shipped entry point? | NO — `SubtitleParser`/`SubtitleSink`/`not_this_format` have zero references outside plugin-api itself (rg over crates/ + apps/): the real `subtitles` crate (srt/vtt/ass/dvb/pgs, ~2300+ LOC) does NOT implement this trait and does not even depend on plugin-api (subtitles/Cargo.toml:12: "plugin-api, which joins when the SubtitleParser seam lands (5.M1)"); playback-engine/demux/lumen-cli never reference the trait. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | cargo test -p plugin-api --offline --lib subtitle → 3 passed, exit 0; rg implementor/consumer searches (exit 0); sed quote verification (exit 0) |

## 1. One-paragraph verdict
A clean, small trait contract (chunked feed → cue sink, three-state ParseStatus with a counted-recovery vocabulary, one typed format-mismatch error helper) with honest tests that exercise chunk-split buffering, drain, and damage-as-data semantics through a toy parser. The code is real and passes (my run + baseline). The problem is adoption: this is a contract nothing implements or calls. The module doc says "Impl crate: `subtitles` (L2)" — but the subtitles crate ships its own parser APIs (srt/vtt/ass/dvb/pgs) without this trait and without a plugin-api dependency; its Cargo.toml names the seam as future card 5.M1. So the doc implies an impl relationship that does not exist today, and every byte of this module is dead weight in the compiled release lane (plugin-api is linked by playback-engine/media-probe, but nothing calls into `subtitle`).

## 2. What actually works (verified)
— `cargo test -p plugin-api --offline --lib subtitle` (my run): `parses_in_chunks_and_drains`, `damage_is_data_not_silence`, `format_mismatch_error_shape` — 3 passed; 0 failed, exit 0. Baseline: cargo-test-workspace.txt:4847-4849 (same 3 ok).
— rg: `impl SubtitleParser` exists ONLY at plugin-api/src/subtitle.rs:106 (the test's LineParser); no `plugin_api` reference anywhere in crates/subtitles/src/.

## 3. Findings
F-001 [S3]
Where: crates/plugin-api/src/subtitle.rs:1-2
Code:
```
//! The subtitle-parser contract (doc 06 §1.2 kind 10): subtitle
//! format → timed [`SubtitleCue`]s. Impl crate: `subtitles` (L2).
```
What is wrong: the doc presents `subtitles` as the implementing crate of this contract. It is not: crates/subtitles has no dependency on plugin-api (Cargo.toml:12 — "plugin-api, which joins when the SubtitleParser seam lands (5.M1)") and implements none of `SubtitleParser`/`SubtitleSink`. No other crate implements or consumes the trait either. The contract is real code with zero adopters — a designed-but-unwired seam documented as if the impl relationship existed. Why it matters: a reader (or a matrix row) could conclude Lumen parses subtitles through this contract; in reality the working subtitle parsers (SRT/VTT/ASS/DVB/PGS) run through their own crate-local APIs, and nothing routes through plugin-api. What correct looks like: date-stamp the doc ("seam lands with card 5.M1") like subtitles/Cargo.toml already does honestly. Confidence: HIGH — Verification: ran (rg) + read.
F-002 [S4]
Where: crates/plugin-api/src/subtitle.rs:122-130
Code:
```
                    let (cue_text, rest) = match text.split_once('|') {
                        Some((a, b)) if !b.is_empty() => (a.to_string(), b.to_string()),
                        _ => {
                            // No complete pair yet — buffer everything.
                            self.pending = Some(text);
                            return Ok(ParseStatus::Accepted);
                        }
                    };
                    self.pending = Some(rest.clone());
```
What is wrong (test-only): the toy LineParser buffers the ENTIRE remainder into `pending` after one cue and only emits one cue per `next_cues` call — a second `text|ticks` pair inside one chunk waits for the next call even though it is already complete; `rest.clone()` is a needless clone. Harmless for the assertions made, but a future reader copying this "precedent" inherits one-cue-per-call behavior. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| trait SubtitleParser | 26-45 | REAL (unadopted) | S3(F-001) | chunked feed, `None`=flush; Send bound matches "worker thread" claim |
| enum ParseStatus | 49-63 | REAL | — | Accepted/Drained/Recovered{skipped, reason} — counted-recovery vocabulary as documented |
| trait SubtitleSink | 68-77 | REAL (unadopted) | — | no Send bound, documented rationale (codec-api precedent) |
| not_this_format | 80-92 | REAL (unadopted) | — | caps sniffed bytes at 16; typed Unsupported{FormatInfo} at Demux stage; tested shape |
| tests::LineParser | 101-158 | REAL | S4(F-002) | toy parser proving the trait's chunking/drain/recovery shape |
| tests::Collector | 160-169 | REAL | — | trivial sink |
| tests::parses_in_chunks_and_drains | 171-198 | REAL | — | split-mid-line buffering asserted; no early emission asserted |
| tests::damage_is_data_not_silence | 200-220 | REAL | — | Recovered{skipped:1} asserted, cue skipped and counted |
| tests::format_mismatch_error_shape | 222-230 | REAL | — | stage + kind asserted |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| parses_in_chunks_and_drains | YES (for the toy) | asserts Accepted with empty sink after half a line, exactly 1 cue after completion, Drained on flush — but it tests the TEST's parser, not any shipped subtitle format parser |
| damage_is_data_not_silence | YES (for the toy) | exact Recovered match + zero cues |
| format_mismatch_error_shape | YES | asserts the real `not_this_format` output stage/kind — the only test touching production code in the file |
Caveat: all parser-behavior tests exercise `LineParser` defined inside the test module; the production surface of this file is traits + one error constructor. The tests validate the contract's shape, not any real SRT/VTT/ASS parsing (that lives un-wired in crates/subtitles).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Impl crate: `subtitles` (L2)" | subtitle.rs:2 | FALSE (today) | subtitles crate: no plugin-api dep (Cargo.toml:12), no impl of the trait (rg); seam deferred to card 5.M1 |
| "bounded work per call — one chunk per next_cues … never an internal parse-the-whole-file loop" | subtitle.rs:20-23 | REAL as a contract rule; UNVERIFIED for real parsers | no real implementor exists to check |
| "panics are contract violations" | subtitle.rs:23 | UNVERIFIABLE | no shipped implementor |
| "recoverable damage skips the cue and is reported as data" | subtitle.rs:37-39 | REAL (vocabulary) | ParseStatus::Recovered{skipped, reason} + test |

## 7. Dead / duplicated / unreachable code
The entire module is currently un-referenced production code (compiled into plugin-api, linked into the release lane via playback-engine/media-probe dependency chain, never called). Not dead by oversight — a planned seam (5.M1) — but dead today.

## 8. Dependencies & flags
media_types only (SubtitleCue, SharedBytes, EngineError, PipelineStage, ErrorKind/FormatInfo/ErrorContext). No features. Test-only: none beyond std.

## 9. Open questions for the next agent
- Card 5.M1 ("SubtitleParser seam"): does doc 13 actually schedule it, or is this another passed window like doc 06 §3's registry? Check doc 13 wave program.
- When the seam lands, will `subtitles` crate parsers satisfy "one chunk per next_cues" bounded-work rule (their parsers are pull-based today)?
