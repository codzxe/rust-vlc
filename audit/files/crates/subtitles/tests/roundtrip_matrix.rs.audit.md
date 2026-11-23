# AUDIT: crates/subtitles/tests/roundtrip_matrix.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-23 |
| Lines / functions | 223 / 2 helpers + 5 tests |
| File verdict | REAL — genuine roundtrip proof of the model's internal wire codec, executed |
| Reachable from shipped entry point? | Test file; the code under test (model::wire) is crate-level UNREACHABLE from shipped binaries |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/deps/roundtrip_matrix-346a1a9cd70bc0ff` -> 5 passed, 0 failed (exit 0); same binary `perf_smoke --nocapture` -> "perf smoke: 15840 cues, 1163281 wire bytes, serialize 14445 µs, parse 24270 µs (min of 3)"; baseline cargo-test-workspace.txt:6086-6095 (5 passed) |

## 1. One-paragraph verdict
This is a real roundtrip proof, not an identity tautology: `serialize_document` (writer with
`\\`,`\n`,`\r`,`\t` escaping) and `parse_document` (separate single-pass text parser with its
own WireError type) are two independent implementations, and the matrix roundtrips 176
adversarial shapes — `i64::MIN`/`i64::MAX` timings, CR/LF/tab/backslash text, unicode,
empty/styled payloads — through all 9 alignments, asserting full structural equality AND
byte stability (re-serialize == original bytes). All 5 tests executed and pass. The scope
limitation: it proves the *internal* `LUMEN-CUES 1` codec lossless, not any real subtitle
format (SRT/VTT/ASS) — the module doc itself says this is "the model's canonical interchange
form for tests and debug dumps".

## 2. What actually works (verified)
— verified by `./target/debug/deps/roundtrip_matrix-346a1a9cd70bc0ff` -> `test result: ok.
5 passed; 0 failed` and the perf smoke run printed real numbers (15,840 cues / 1.16 MB wire /
14.4 ms serialize / 24.3 ms parse, min-of-3).

## 3. Findings
F-001 [S4]
Where: crates/subtitles/tests/roundtrip_matrix.rs:1-4 and docs/feature-matrix.toml:1124
Code:
```
//! Card 5.L2 acceptance: **cue → stable serialize → parse → equal**,
//! as a 176-cue × 9-alignment matrix (1584 cues), plus per-cue
//! roundtrip, serialization determinism, and an honest perf smoke
```
```
evidence = "crates/subtitles/src/compositing/** + model (cards 5.L1+5.L2: CueBitmap/CueLayout/place_cue pixel-exact math, 176-cue x 9-alignment roundtrip matrix, timing edges; 5.M2b placeholder glyph metrics: monospace-widest-char, integer ratios, no floats)"
```
What is wrong: The feature-matrix row `textrender-subtitle-rasterizer` lists the roundtrip
matrix inside its evidence string. The test proves the internal wire codec is lossless; it
proves nothing about rasterization or about any real subtitle format. The row's own notes
admit the font stack is not started, so this is inflation-by-association, not fraud — but the
evidence string would survive a reader assuming "roundtrip" means VTT/SRT roundtrip.
What correct looks like: cite the wire roundtrip under a model/interchange row, not under
the rasterizer.
Confidence: HIGH — Verification: read (row + test both read; test executed).

F-002 [S4]
Where: crates/subtitles/tests/roundtrip_matrix.rs:166-176
Code:
```
    let a = serialize_document(cues);
    let mut b = String::new();
    subtitles::model::wire::write_document(cues, &mut b);
    assert_eq!(
        a, b,
        "serialize_document == write_document (same buffer path)"
    );
```
What is wrong: `serialize_document` is a thin wrapper that calls `write_document` (wire.rs
L100-106), so this compares two invocations of the SAME writer — a determinism check in the
literal sense only. It cannot distinguish "deterministic and correct" from "deterministic
and wrong"; correctness coverage comes entirely from the roundtrip equality asserts. The
parenthetical admits "same buffer path". Redundant with the byte-stability assert in
matrix_document_roundtrip (L139-143).
Confidence: HIGH — Verification: read (wire.rs wrapper confirmed) + executed (passes).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| shapes | 26-103 | REAL | — | 6 starts × 7 durations (incl. i64 extremes, zero-duration, negative), 7 line modes, 5 positions, 8 text flavors; saturating end computation keeps CueInterval::new valid |
| matrix | 106-124 | REAL | — | 9 alignments × 176 shapes; count pinned by assert (L161) |
| matrix_document_roundtrip | 127-146 | REAL | — | equality + byte stability per alignment; would fail on any escaping/field loss (mutation reasoning: `\n`/`\t`/`\r`/`\\` text and per-doc distinct alignments) |
| per_cue_roundtrip | 149-163 | REAL | — | each of 1584 cues alone; wire text in the failure message |
| serialization_is_deterministic | 166-176 | REAL | S4 | wrapper-vs-itself (F-002) |
| empty_document_roundtrips | 179-183 | REAL | — | exact magic line pinned |
| perf_smoke_15k_cues | 189-223 | REAL | — | prints, no timing gate (honestly labeled); asserts parse count; real numbers captured |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| matrix_document_roundtrip | YES | parse(serialize(x)) == x with hostile escapes; alignment loss would collapse 9 distinct docs to defaults |
| per_cue_roundtrip | YES | per-cue equality incl. id/interval/positioning/text |
| serialization_is_deterministic | NO (beyond double-run) | same writer called twice (F-002) |
| empty_document_roundtrips | YES | exact bytes pinned |
| perf_smoke_15k_cues | only len | perf numbers printed, not gated — by design and disclosed |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cue → stable serialize → parse → equal, as a 176-cue × 9-alignment matrix (1584 cues)" | L1-3 | REAL | executed: 5/5 pass; count assert L161 |
| "every timing edge (zero-duration, negative starts, the i64 extremes)" | L23-25 | REAL | starts/durations arrays L27-28 |
| "honest perf smoke (not a gate)" | L3-4 | REAL | no timing assert; numbers printed under --nocapture (captured above) |
| wire.rs "[PROPOSAL: this is the model's canonical interchange form for tests and debug dumps]" | wire.rs:44-46 | REAL | scope admission — internal format, not a subtitle format |

## 7. Dead / duplicated / unreachable code
No dead code. The tested codec is product-unreachable (crate-level, see dvb/mod.rs F-008).

## 8. Dependencies & flags
subtitles::model only; no features; test binary prebuilt in baseline.

## 9. Open questions for the next agent
- Is there ANY test that roundtrips a real format (VTT→model→VTT) or is source-format
  roundtrip entirely untested in the crate? (vtt_acceptance parses only — W09-A01's file.)
