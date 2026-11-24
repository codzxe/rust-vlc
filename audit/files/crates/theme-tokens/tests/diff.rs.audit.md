# AUDIT: crates/theme-tokens/tests/diff.rs
| Field | Value |
|---|---|
| Auditor | W11-A06 |
| Date | 2026-11-23 |
| Lines / functions | 126 / 5 (+2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test) — runs under `cargo test --workspace` (scripts/full_gate.sh:21 `cargo test --workspace --all-features`; baseline cargo-test-workspace.txt:6310-6322). Guards the shipped artifacts themes/generated/tokens.css, crates/theme-tokens/src/generated/tokens.rs, ui/src/tokens/generated/tokens.css. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `target/debug/deps/diff-081ea81a32cdde49` (run in crates/theme-tokens) -> 2 passed, exit 0 (my own execution of this exact test); read tools/xtask/src/cmd/tokens.rs (subprocess bridge, exit status forwarded); baseline full_gate.txt. |

## 1. One-paragraph verdict
This is the rarest thing in the repo: a drift gate with a built-in sensitivity proof. `committed_outputs_match_regeneration` re-derives all three committed artifacts from themes/tokens.json and compares them; `the_comparison_detects_drift` mutates one primitive and proves the comparison is not vacuous. I executed the test binary myself — 2 passed, exit 0. The single defect is precision of the contract wording: the header claims "byte equality" but the comparison is `str::lines()`-based, which strips `\r` and tolerates a missing final newline — EOL-only drift would pass this gate.

## 2. What actually works (verified)
— verified by my own run: `target/debug/deps/diff-081ea81a32cdde49` from crates/theme-tokens -> `test committed_outputs_match_regeneration ... ok`, `test the_comparison_detects_drift ... ok`, exit 0. All three artifacts byte-identical to regeneration (modulo F-001's EOL caveat).
— the UI bridge copy (ui/src/tokens/generated/tokens.css) is included in the gate (lines 99-102), closing the "ui.yml has no tokens step" gap from the Rust side — verified the file exists and matches (my `diff` in the tokens.css audit: IDENTICAL).
— the mutation target `#191919` (gray.700) is chosen because it is a unique `$value` — the comment explains why a bare hex was rejected (appears in descriptions too); this is deliberate test design, not luck.

## 3. Findings
F-001 [S4]
Where: crates/theme-tokens/tests/diff.rs:17-18 vs 40-52
Code:
```
//! Byte equality, first differing line reported — the same contract as
//! `tokens-gen --check`, but running everywhere `cargo test` runs.
```
and
```
    let mut line = 1;
    for (x, y) in a.lines().zip(b.lines()) {
        if x != y {
            return Some(line);
        }
```
What is wrong: `str::lines()` treats `\r\n` as a line ending (strips the `\r`) and makes the final line ending optional, so the comparison is line-based, not byte-based: a committed artifact converted to CRLF, or one missing its final trailing newline, still passes `first_diff_line == None`. The claim "the same contract as `tokens-gen --check`" is inaccurate — tokens-gen's `drift()` (bin/tokens_gen.rs:280) compares whole `String`s (true byte equality). Practical risk is low (git EOL policy + tokens-gen --check in CI both back this up; and the emitted documents end "}\n" consistently), but this gate specifically would not catch EOL-only corruption. What correct looks like: compare `committed.as_bytes() == expected.as_bytes()` for the gate, keep first_diff_line only for the message. Confidence: HIGH — Verification: read (std `str::lines` semantics) + ran the test.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| workspace_root | 28-30 | REAL | — | CARGO_MANIFEST_DIR/../.. |
| committed_source | 32-36 | REAL | — | reads themes/tokens.json |
| first_diff_line | 40-52 | REAL | S4 | line-based, \r-stripping (F-001); prefix+length case handled |
| assert_matches | 54-70 | REAL | — | actionable panic on unreadable file; line number in message |
| panic_wrapper | 75-78 | REAL | — | documented clippy::panic deny workaround |
| committed_outputs_match_regeneration | 81-104 | REAL | — | 3-artifact gate, executed by me |
| the_comparison_detects_drift | 107-126 | REAL | — | sensitivity proof via single-value mutation; asserts mutation applied |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| committed_outputs_match_regeneration | YES | any value/structure change in tokens.json without regeneration changes the emitted bytes on some line |
| the_comparison_detects_drift | YES | proves the equality path is sensitive to a one-value mutation (would catch a stubbed/constant emitter) |
Both executed by me (exit 0). This is the pattern the fakeness checklist wants: the test proves its own detector works.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| committed outputs "diff-tested by crates/theme-tokens/tests/diff.rs" | emit.rs:364-366, tokens.css:5-6, lib.rs:19-20 | REAL | this file is exactly that; I ran it |
| gate covers the UI bridge copy | diff.rs:14-15, 95-98 | REAL | third assert_matches target verified identical |
| "Byte equality ... the same contract as tokens-gen --check" | diff.rs:17-18 | FALSE (line-based; EOL/trailing-newline blind) | F-001 |
| "running everywhere cargo test runs (ci.yml gate job, full_gate.sh #3)" | diff.rs:5-6 | REAL | scripts/full_gate.sh:21 cargo test --workspace --all-features; baseline workspace test ran it (6310) |

## 7. Dead / duplicated / unreachable code
None. 126 lines, all load-bearing.

## 8. Dependencies & flags
- std-only + theme_tokens lib (emit, parse). No features. Runs in the default test lane.

## 9. Open questions for the next agent
- Is `cargo xtask tokens --check` (the true byte-compare) actually scheduled in ci.yml beyond the build canary? I found the tokens-gen canary (ci.yml:589) but did not trace a `--check` invocation — UNVERIFIED whether the byte-level gate runs per-commit in CI or only via full_gate/manual runs.
