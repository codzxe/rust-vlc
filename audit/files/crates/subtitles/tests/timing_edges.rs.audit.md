# AUDIT: crates/subtitles/tests/timing_edges.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-23 (repo frozen; audit session 2025-07-14 clock) |
| Lines / functions | 173 / 8 tests |
| File verdict | REAL (compiles, links, passes — verified live; the Wave-0 attempt1 link failure was environmental, not code) |
| Reachable from shipped entry point? | N/A (test) — workspace test lane; code under test (subtitles::model) not reachable from shipped binaries (pgs/mod.rs audit F-002) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `~/.cargo/bin/cargo test -p subtitles --test timing_edges --locked --no-run` → `Finished test profile … in 0.02s`, executable produced (exit 0); `./target/debug/deps/timing_edges-d2ce4d2cf9009541` → 8 passed, 0 failed (exit 0); `./target/debug/deps/timing_edges-e8586f53d0d9c775` → 8 passed (exit 0) |

## 1. One-paragraph verdict
The orchestrator flagged this file: "the test that couldn't link in Wave-0
attempt1". Verified end-to-end: `cargo test --no-run` confirms it compiles
and links right now (exit 0, 0.02 s freshness — no rebuild needed), and both
prebuilt binaries pass all 8 tests. The Wave-0 attempt1 failure log proves
the original breakage was the environment, not this code: `ld terminated
with signal 7 [Bus error]` and `No space left on device` across many crates
in the same run (audit/runs/baseline/cargo-test-workspace.attempt1-diskpoisoned.txt:148-180,
:193-199 — the file is literally named "diskpoisoned"). The test content
itself is a strong wire-level edge suite: hand-written wire bytes (not
self-generated goldens), exact typed-error equality including line numbers,
interval algebra (touching vs overlap vs hull), and a 216-case
alignment×placement×percent saturation sweep. No findings.

## 2. What actually works (verified)
— Compile/link: `cargo test -p subtitles --test timing_edges --locked
--no-run` → `Finished test profile [unoptimized + debuginfo] target(s) in
0.02s / Executable tests/timing_edges.rs
(target/debug/deps/timing_edges-d2ce4d2cf9009541)` — exit 0.
— Run: `./target/debug/deps/timing_edges-d2ce4d2cf9009541` → `test result:
ok. 8 passed; 0 failed; 0 ignored` — exit 0. Second binary
(e8586f53d0d9c775, the one in the passing baseline run) → 8 passed.
— Baseline: audit/runs/baseline/cargo-test-workspace.txt:6129-6152 (all 8
pass in the successful Wave-0 rerun).
— Attempt1 context: cargo-test-workspace.attempt1-diskpoisoned.txt:148-157
(`error: linking with cc failed` / `collect2: fatal error: ld terminated
with signal 7 [Bus error]` / `error: could not compile subtitles (test
"timing_edges")`), alongside :193-199 `No space left on device` for other
crates — disk-poisoning, per the orchestrator's own file naming.

## 3. Findings
None. Checked, with method:
1. Link/compile honesty (the orchestrator's focus) — executed cargo --no-run
   (exit 0) + both binaries (8/8 each). The attempt1 failure is fully
   attributed to the poisoned disk by the baseline logs.
2. Typed-error pins — read: L43-53 asserts the exact
   `WireError::EndBeforeStart { line_no: 2, start_us: 2000, end_us: 1000 }`
   value (not just is_err); L74-106 asserts exact
   `PercentOutOfRange { line_no: 2, value }` for all four sign/field
   combinations. A regression that changes line accounting or error payload
   fails.
3. Golden independence — read: every wire string is hand-written literal
   ("LUMEN-CUES 1\nC 1 2500 2500 tl - -\nP x\n", L34; the touching pair L59;
   the CRLF doc L135) — none produced by serialize_document, so a
   serializer/parser co-drift cannot satisfy them.
4. Interval algebra — read + ran: overlaps/touches/intersection/hull over
   the shared boundary 1000 (L62-68); consistent with half-open semantics.
5. Saturation sweep — read + ran: 9 alignments × 8 placements (incl.
   LineNumber(i64::MAX) and i64::MIN) × 3 positions, each resolved anchor
   asserted within the viewport (L143-172).
6. Header cross-ref claim ("constructors are unit-pinned in
   src/model/time.rs") — verified by rg: model/time.rs:154-227 contains
   zero_duration_is_legal_and_empty, end_before_start_is_typed_invalid,
   touching_is_not_overlap. Claim accurate.
7. No #[ignore]/cfg-gating (rg); no fixtures needed; deterministic.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| zero_duration_is_legal_at_the_wire | 17-30 | REAL | — | constructor + serialize/parse roundtrip equality |
| zero_duration_wire_bytes | 32-40 | REAL | — | hand-written wire, duration_us()==0 |
| end_before_start_is_typed_invalid_at_the_wire | 42-53 | REAL | — | exact typed error incl. line_no |
| touching_is_not_overlap_at_the_wire | 55-70 | REAL | — | overlaps/touches/intersection/hull |
| out_of_range_percent_is_typed_invalid_at_the_wire | 72-107 | REAL | — | 4 exact-error cases (line/position × over/under) |
| boundary_percents_are_legal_at_the_wire | 109-131 | REAL | — | 0/100 legal, full Positioning equality |
| crlf_documents_are_tolerated | 133-140 | REAL | — | \r\n wire parses, no CR in text |
| every_alignment_resolves_within_the_viewport | 142-173 | REAL | — | 216 cases, i64-extreme line numbers, within-viewport asserts |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| zero_duration ×2 | YES | roundtrip equality / hand-written wire bytes |
| end_before_start | YES | exact error variant + payload |
| touching_is_not_overlap | YES | all four algebra ops pinned |
| out_of_range_percent | YES | exact variant + value + line for 4 shapes |
| boundary_percents | YES | full Positioning struct equality |
| crlf_tolerated | YES | CR would leak into text on regression |
| every_alignment_resolves | YES | out-of-viewport anchor or panic fails |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the model constructors are unit-pinned in src/model/time.rs; this suite pins the same edges through serialize/parse" | L1-3 | REAL | model/time.rs:154-227 tests exist; this file's 8 tests all go through wire/resolve (ran) |
| Edge list (zero-duration legal, end<start typed, touching≠overlap, percents typed) | L5-8 | REAL | one test per claim, all passing |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
subtitles::model only; std::error::Error for the Result-returning tests. No
testkit dependency, no features, no ignores.

## 9. Open questions for the next agent
- None for this file. The Wave-0 attempt1 incident needs no code follow-up;
  if audit/verify/ tracks environment incidents, cite the diskpoisoned log.
