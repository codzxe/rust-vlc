# AUDIT: crates/subtitles/src/model/wire.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-24 |
| Lines / functions | 518 / 19 |
| File verdict | REAL (as its declared test/debug interchange purpose; all its consumers are tests and they pass) |
| Reachable from shipped entry point? | NO — pub use at model/mod.rs:34; callers are tests/{timing_edges,roundtrip_matrix,fuzz_survival}.rs only; no product crate depends on `subtitles` (rg over every Cargo.toml: only fuzz/Cargo.toml:50) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `cargo test -p subtitles --test roundtrip_matrix` → 5 passed (exit 0); `--test timing_edges` → 8 passed (exit 0); `--test fuzz_survival` → 4 passed (exit 0); sed quote verification |

## 1. One-paragraph verdict
A clean, genuinely round-tripped binary-ish text wire format for the cue model: serialize/parse with escaped one-line records, typed errors carrying line numbers, `Copy` header state, and real test coverage — 176×9 alignment matrix round-trip with byte stability, per-cue round-trip, fuzz-mutation idempotence (all executed by me, all green). The "infallible by construction" claims check out against the model constructors: `CueInterval::new` (time.rs:48-53) only rejects `end < start`, which `parse_header` already rejected; `Percent::new` (position.rs:41-46) checks the identical `0..=100` domain; `CueId::new`/`Cue::new` are infallible. The defects are small: the "each source byte is visited exactly once" claim is false on one error path, the serializer allocates throwaway `String`s per cue, and one defensive branch is dead. Its own header honestly scopes it as a test/debug dump format ([PROPOSAL], line 52-54), so lack of product reachability is by design — the crate-wide dead-code fact is filed in the eia608.rs audit (F-001 there).

## 2. What actually works (verified)
— verified by `cargo test -p subtitles --test roundtrip_matrix` → 5 passed incl. `matrix_document_roundtrip` (equality + byte stability) and `serialization_is_deterministic`; `--test timing_edges` → 8 passed (zero-duration wire bytes pinned); `--test fuzz_survival` → 4 passed (mutated docs never panic, parse→serialize→parse idempotent, both accept/reject directions bite >100 times).

## 3. Findings
F-001 [S3]
Where: crates/subtitles/src/model/wire.rs:39-42 vs 290-294 and 324-326
Code (39-42, verbatim):
```
//! * **Single-pass parser** — [`parse_document`] walks the input once
//!   via `str::split('\n')` (zero-copy `&str` slices, no line vector,
//!   no regex, no intermediate collections); each source byte is
//!   visited exactly once.
```
Code (290-294 + 324-326, verbatim):
```
    if header.is_some() {
        return Err(WireError::MissingPayload {
            line_no: line_count(src),
        });
    }
```
```
fn line_count(src: &str) -> usize {
    src.split('\n').count()
}
```
What is wrong: The EOF missing-payload error path calls `line_count(src)`, a second full scan of the entire input, so on that path every byte is visited twice — the engineering claim "each source byte is visited exactly once" is false. The loop's own `enumerate` already produced the final index; throwing it away and re-scanning is pure waste on malformed input. Why it matters: the file's perf claims are its selling point ("Engineering notes (speed + memory care)"); a doc claim contradicted by the code is exactly what this audit exists to catch. What correct looks like: track `last_line_no` from the loop and use it. Confidence: HIGH — Verification: read.

F-002 [S4]
Where: crates/subtitles/src/model/wire.rs:103-114
Code (verbatim):
```
    let p = &cue.positioning;
    // Header: all Copy values, one formatted write into the buffer.
    let line_tok = match p.line {
        None => "-".to_string(),
        Some(LinePlacement::Percent(v)) => format!("p{}", v.get()),
        Some(LinePlacement::LineNumber(n)) => format!("n{n}"),
    };
    let pos_tok = match p.position {
        None => "-".to_string(),
        Some(v) => format!("{}", v.get()),
    };
```
What is wrong: The comment "all Copy values, one formatted write into the buffer" is followed by two to four heap allocations per cue (`to_string`/`format!` temporaries) before the single `writeln!`. Contradicts the parse-side "heap allocation starts only at the text payload" framing when read as a whole-file property. Blast radius: 2 short-lived allocations per serialized cue — measurable only at the 15k-cue perf-smoke scale. What correct looks like: write the tokens directly into `out` (push_str + itoa-free paths already exist for the integers via `write!`). Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/subtitles/src/model/wire.rs:264-269
Code (verbatim):
```
                if st.remaining == 0 {
                    return Err(WireError::BadValue {
                        line_no,
                        field: "segment overflow",
                    });
                }
```
What is wrong: Dead branch. `styled` is only ever `Some` with `remaining >= 1` (the `count == 0` case completes immediately at 242-244; `remaining` reaches 0 only inside the 272-283 arm which takes `styled`), so a `T` line can never observe `st.remaining == 0`. Harmless defensive code, but it is unreachable and untested. Confidence: HIGH — Verification: read + state-machine reasoning over all arms.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| serialize_document | 88-92 | REAL | — | capacity hint 48 B/cue |
| write_document | 95-101 | REAL | — | magic + cues |
| write_cue | 103-149 | REAL | S4 | F-002 temporaries |
| escape_into | 152-162 | REAL | — | 4-escape set |
| unescape_into | 166-183 | REAL | — | trailing `\` → typed BadEscape; no panic paths (byte indices ASCII-only) |
| parse_document | 191-296 | REAL | S3/S4 | F-001 second scan on error path; F-003 dead branch; state machine sound (C/P/S/T) |
| reject_unterminated | 309-318 | REAL | — | typed counts |
| strip_cr | 320-322 | REAL | — | |
| line_count | 324-326 | REAL | — | second-pass offender (F-001) |
| payload_body | 329-343 | REAL | — | bare tag → "" |
| parse_header | 349-391 | REAL | — | full validation before alloc; ExtraFields check |
| parse_line_spec | 393-415 | REAL | — | `-`/`p<0-100>`/`n<i64>` (negatives legal per doc) |
| parse_percent_opt | 418-423 | REAL | — | |
| parse_percent | 425-438 | REAL | — | "Unreachable" Err arm verified identical-domain vs position.rs:41-46 |
| parse_segment | 440-469 | REAL | — | 3-flag + hex color + escaped text |
| split_first_token | 471-480 | REAL | — | |
| field_str | 482-490 | REAL | — | |
| parse_field | 492-500 | REAL | — | generic, typed BadInt |
| build_cue | 504-517 | REAL | — | Err arm structurally dead (verified vs time.rs:48-53); maps to EMPTY not a panic |

## 5. Test honesty
No in-file tests; the integration suites cover it and I ran them:
| Test (file) | Would it fail if code were broken? | Reason |
|---|---|---|
| matrix_document_roundtrip (roundtrip_matrix) | YES | 9-alignment × 176-shape equality + byte-stability assertions |
| per_cue_roundtrip (roundtrip_matrix) | YES | count pinned to `N_SHAPES * 9` |
| serialization_is_deterministic | YES | serialize_document vs write_document byte equality |
| empty_document_roundtrips | YES | exact `"LUMEN-CUES 1\n"` bytes |
| perf_smoke_15k_cues | NO (not a gate) | asserts only cue count; timings recorded, not asserted — honest label |
| zero_duration_* / touching_* (timing_edges, 8) | YES | exact wire bytes + typed error pins |
| mutated_documents_never_panic (fuzz_survival) | YES | accepts>100/rejects>100 bite assertions + idempotence |
| random_buffers_never_panic | YES | 2000 wild buffers through parse_document |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cue → serialize → parse → equal (card 5.L2 acceptance)" | wire.rs:1-2 | REAL | roundtrip_matrix run: 5 passed |
| "each source byte is visited exactly once" | wire.rs:39-42 | FALSE on one path | line_count re-scan (F-001) |
| "re-serializing a parsed document reproduces the accepted bytes" | wire.rs:48-50 | REAL | `matrix_document_roundtrip` byte-stability assert (run, passed) |
| "heap allocation starts only at the text payload" (parse side) | wire.rs:35-38 | REAL | parse_header is slice-only until String::with_capacity at 234/466 |
| "[PROPOSAL: canonical interchange form for tests and debug dumps]" | wire.rs:52-54 | REAL | only test files consume it (rg) |

## 7. Dead / duplicated / unreachable code
F-003 (segment-overflow dead branch). `expected` field of `StyledPending` duplicates `remaining` info but is used in the error payload (legit). File itself unreachable from product binaries (see header field; filed at crate level in eia608.rs audit F-001).

## 8. Dependencies & flags
Std-only; no feature gates; no unsafe; no unwraps on input paths (`ok_or` everywhere); allocations bounded by input size (styled `remaining` cannot preallocate: `Vec::new()` at 248).

## 9. Open questions for the next agent
- The wire format has no version-drift story beyond `LUMEN-CUES 1` magic — fine for tests, but if anyone wires it into debug dumps of shipped builds, revisit F-001/F-002.
- `Cue` id is `u64` from wire — collision handling is out of scope here; check whoever aggregates parsed docs.
