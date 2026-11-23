# AUDIT: crates/subtitles/tests/vtt_fuzz.rs
| Field | Value |
|---|---|
| Auditor | W09-A03 |
| Date | 2026-11-15 |
| Lines / functions | 205 / 3 fns + 3 tests |
| File verdict | REAL (deterministic fuzz-lite that actually asserts invariants + determinism) — inside the UNREACHABLE island crate |
| Reachable from shipped entry point? | N/A (test file); tested crate `subtitles` compiled by no shipped host |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | prebuilt `target/debug/deps/vtt_fuzz-7b8ccb82eebade1a` -> 3 passed / 0 failed, 1.03s (exit 0); rg VttError enum (1 variant); sed quote verification |

## 1. One-paragraph verdict
A real, honest fuzz-lite: 4000 seeded mutation rounds over a genuinely varied 64-cue seed
(tags, escapes, settings, damage bait) plus 3000 random buffers, asserting no-panic, model
invariants on every accepted document (non-reversed intervals, percents ≤ 100, 1-based line
numbers), and parse determinism (parse twice, assert_eq). The `unreachable!` claims about
`VttError` are accurate today (enum has exactly one variant, verified). The 199KB
unterminated-tag bomb is covered and pinned to drop the tail. All 3 tests pass (executed).
Gaps are minor: single seed family, `\n`-only line endings, and the brittle unreachable
canaries.

## 2. What actually works (verified)
— verified by `./target/debug/deps/vtt_fuzz-7b8ccb82eebade1a` ->
`test result: ok. 3 passed; 0 failed; ... 1.03s` — both arms exercised (suite asserts
`accepted > 100` and `rejected > 100` internally at :128-132 and passed).
— verified by rg: `pub enum VttError` (vtt/mod.rs:108-112) has only `MissingHeader`, so the
`unreachable!("VttError has no other variant today")` arms are currently truthful.

## 3. Findings
F-001 [S4]
Where: crates/subtitles/tests/vtt_fuzz.rs:124-126 (and :154-156)
Code:
```
            Err(VttError::MissingHeader) => rejected += 1,
            Err(_) => unreachable!("VttError has no other variant today"),
```
What is wrong: a future second `VttError` variant turns this suite into a panic-based
failure with no message about which round/input tripped it (no input dump). Brittle canary
duplicated across both tests. What correct looks like: `Err(other) => panic!("round {round}:
unexpected {other:?}")` with the offending text included.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: crates/subtitles/tests/vtt_fuzz.rs:35-68
Code:
```
        "WEBVTT - fuzz seed\n\nNOTE comment\n\nSTYLE\n::cue { color: gold; }\n\nREGION\nid:r width:50%\n\n",
```
What is wrong: the seed (and hence every mutation) uses `\n`-only line endings and no BOM —
real-world WebVTT is frequently CRLF and BOM-prefixed; neither path is fuzzed here (the
random-buffer test can accidentally produce CRLF only by luck). Coverage gap, not a defect
in what is covered. Confidence: HIGH — Verification: read (only "\n" literals in the seed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Lcg (xorshift64*) | 12-31 | REAL | — | deterministic; below(0) guarded |
| seed_document | 33-68 | REAL | — | 64 cues, 3 id shapes, 6 setting shapes incl. hostile `position:200%`, 4 text shapes incl. unterminated tag |
| mutated_documents_never_panic | 70-133 | REAL | S4 | 4000 rounds, 4 mutation kinds, header-biased byte flips, <200KB cap, invariants + determinism |
| random_buffers_never_panic | 135-158 | REAL | — | 3000 x ≤400B, half printable/half wild |
| large_document_under_200kb_is_survivable | 160-186 | REAL | — | 160KB doc + 199KB '<' bomb, tail-drop pinned |
| check_invariants | 188-205 | REAL | — | interval order, percent ranges, 1-based damage lines |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| mutated_documents_never_panic | YES | invariant asserts (reversed interval, out-of-range percent) + determinism assert_eq + both-arms counters |
| random_buffers_never_panic | YES (weaker) | invariants on accepted docs; rejection path only via MissingHeader |
| large_document_under_200kb_is_survivable | YES | zero-damage assert + cue count + total_len()==0 tail-drop pin |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every input under 200 KB — must never panic, and the only typed failure is the missing-header error" | :1-5 | REAL | cap enforced :111-113; single-variant enum verified; 3/3 pass |
| "Anything accepted satisfies the model invariants (valid intervals, in-range percentages)" | :4-6 | REAL | check_invariants executed on every accepted doc (test passed) |
| "parsing is deterministic (same input → same document)" | :5-6 | REAL | :121-122 assert_eq, executed |

## 7. Dead / duplicated / unreachable code
None in-file (the two `unreachable!` arms are live canaries, see F-001).

## 8. Dependencies & flags
Dev-consumer of `subtitles::vtt` + `testkit::Must`. No features. No unsafe.

## 9. Open questions for the next agent
- Does the VTT parser accept CRLF/BOM inputs at all (vtt/timestamp.rs, vtt/mod.rs owners)?
  If yes, add a CRLF+BOM seed here; if no, that's a parser finding, not a fuzz finding.
- `VttError` non_exhaustive? If not, adding a variant is a breaking change that this suite
  will meet with a panic (F-001).
