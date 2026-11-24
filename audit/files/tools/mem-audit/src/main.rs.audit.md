# AUDIT: tools/mem-audit/src/main.rs
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 262 / 3 fns (main, run, serial) + 2 tests + 4 sibling modules |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev instrument binary `lumen-mem-audit` (root workspace member, Cargo.toml members L41); not shipped in lumen-cli/desktop release lane. Tests ARE run by CI: baseline cargo-test-workspace.txt L3290-3293 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `ls tests/fixtures/generated/valid/` (fixture avi_mjpeg_pcm.avi = 425,516 B exists, matches test comment L239); `rg -n "mem-audit" Cargo.toml` → members L41; `sed -n '3288,3296p' audit/runs/baseline/cargo-test-workspace.txt` → `9 passed; 0 failed` incl. both tests below |

## 1. One-paragraph verdict
A real measurement instrument entry point: counting global allocator installed at L75-76, argument parsing with typed usage errors (exit 64), per-file walk with repeat dimension-stability, per-class summaries. Its two tests are HONEST and RUN (committed baseline, not just claimed): the fixture test pins the real AVI decode path twice and asserts deterministic counts + peak stability + streaming growth bound; the garbage test asserts the typed-refusal contract. The only defects are a swallowed fs::write in the garbage test (could silently degrade to a missing-file test) and zero coverage of the CLI parsing body it deliberately extracted for testability.

## 2. What actually works (verified)
- `sed -n '3288,3296p' audit/runs/baseline/cargo-test-workspace.txt` → `test tests::fixture_walks_twice_with_stable_peak ... ok`, `test tests::garbage_file_is_a_typed_refusal ... ok`, `test result: ok. 9 passed; 0 failed` — committed run, package lumen-mem-audit exit_code=0.
- Fixture existence verified: `ls` shows tests/fixtures/generated/valid/avi_mjpeg_pcm.avi (425,516 B — matches the inline comment L239 "425,516 B").
- run(): --repeat bounded 1..=1000 (L106), unknown `--` flags rejected with exit 64 (L113-116), CLASS= overrides parsed via report::parse_entry (L117).

## 3. Findings
F-001 [S4]
Where: tools/mem-audit/src/main.rs:253-255
Code (verbatim, verified `sed -n '253,257p'`):
```
        let path = std::env::temp_dir().join("lumen-mem-audit-garbage.bin");
        std::fs::write(&path, [0x37u8, 0x13, 0x00, 0x42, 0xff, 0xee].repeat(64))
            .unwrap_or_default();
```
What is wrong: `unwrap_or_default()` swallows the write error; on a full/unwritable temp dir the test would exercise a MISSING file instead of garbage bytes and still pass (both produce Verdict::Refused + error). The test then believes it proved "garbage bytes are a typed refusal" when it may have proved "missing file is a typed refusal". Why it matters: swallowed-error tests degrade silently — exactly the class this audit exists to catch. What correct looks like: `.unwrap()` or assert the write succeeded.
Confidence: HIGH — Verification: read + `sed -n '253,257p'` verbatim match.
F-002 [S4]
Where: tools/mem-audit/src/main.rs:89-200 (fn run)
What is wrong: the doc comment calls run() "The testable body of `main`" (L89-90), but no test covers it: no test exercises exit-64 parsing, --repeat bounds (1..=1000), --self-check dispatch, --demux-only, CLASS= override syntax, or the per-class summary aggregation. Why it matters: the extracted-for-testability surface is untested; regressions in the CLI (the instrument's actual user interface) fail no gate. What correct looks like: run(vec![]) → MEMNOOP line + exit 0; run with unknown flag → 64. Confidence: HIGH — Verification: read; `rg -n "fn run\(" tools/mem-audit/src/main.rs` + module tests list (2 tests only, both walk-level).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 84-87 | REAL | — | thin wrapper, exit code from run() |
| run | 90-200 | REAL (baseline-verified output path) | S4 | parse/dispatch/aggregate logic untested (F-002) |
| testutil::serial | 46-67 | REAL | — | poison-recovered global lock; correct rationale (one global allocator, multi-threaded test runner) |
| tests::fixture_walks_twice_with_stable_peak | 211-246 | REAL | — | honest; asserts real decode counts, determinism, ±64KiB peak stability, growth < size |
| tests::garbage_file_is_a_typed_refusal | 250-261 | REAL | S4 | F-001 swallowed write |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixture_walks_twice_with_stable_peak | YES | asserts v_dec==v_pkts / a_dec==a_pkts (any dropped packet fails), determinism equality, peak abs_diff ≤ 64KiB (a leaking walk fails), growth < 425,516 (buffering-whole-file fails). Verified run: baseline L3290 ok. |
| garbage_file_is_a_typed_refusal | YES (weakly) | asserts Refused + typed error + 0 packets; would fail if garbage decoded or panicked. Weakened by F-001: on write failure it silently tests the missing-file variant instead. Verified run: baseline L3291 ok. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Since card 5.X2a the walk drives every landed demux family (AVI full decode; mkv/mp4/ogg demux drilldowns; wav + PCM decode)" | module doc L13-15 | REAL-UNPROVEN | walk.rs not in my assignment; UNVERIFIED which families walk.rs drives |
| "Exit codes: 0 the requested runs completed ... 64 usage" | module doc L28-29 | REAL | run() returns 64 on all three usage arms (L102-103, 108-110, 114-115), 0 otherwise (L199) |
| "Never panics on bad input — usage errors are typed lines + exit 64" | module doc L82-83 | REAL-UNPROVEN | no unwrap/panic in run() body (read); runtime UNVERIFIED |

## 7. Dead / duplicated / unreachable code
None found. USAGE const L79-80 includes `--demux-only` (flag handled L99) — consistent.

## 8. Dependencies & flags
Root workspace member (Cargo.toml L41); sibling modules allocator/report/selfcheck/walk (not in assignment). unsafe posture documented in header (global allocator) — allocator.rs not my file, UNVERIFIED.

## 9. Open questions for the next agent
- walk.rs: does walk_with(demux_only) actually implement the mkv/mp4/ogg drilldowns the header claims?
