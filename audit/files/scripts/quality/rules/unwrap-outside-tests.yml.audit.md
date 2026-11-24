# AUDIT: scripts/quality/rules/unwrap-outside-tests.yml
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / patterns | 23 / 2 patterns (unwrap, expect) with test-scope exemption |
| File verdict | REAL — EXECUTED: the rule runs under the exact pinned CLI and returns 38 hits on this tree |
| Reachable from shipped entry point? | NO — quality rule; consumed by scripts/quality/03_astgrep.sh (canonical CI path) and astgrep_scan.ps1 (Windows driver) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `npx --yes -p @ast-grep/cli@0.51.3 ast-grep scan --rule <this rule> --json crates apps tools tests` -> 38 hits (full list captured to audit/scratch/W17-A04/unwrap-hits.json); `sed -n '/[workspace.lints.clippy]/,/^\[/p' Cargo.toml` -> unwrap_used/expect_used = deny |

## 1. One-paragraph verdict
The rule is real and I ran it with the repo's own pinned ast-grep version: 38 hits, deterministic, and it does catch what it exists to catch — the intentional workspace-lint canary (`crates/_template/src/lint_canary.rs:30 maybe.unwrap()`) and 25 genuine unwrap/expect sites in the standalone vta harnesses. But a third of its output is noise: the `$RECV.expect($MSG)` pattern matches ANY method named `expect`, so 12 hits are custom byte-parser methods (`self.expect(b'{')`) in xtask and the first-vertical test file — not Option/Result panics. The message's "hits are escapes from [the workspace lint]" is also misleading for the vta harnesses, which are standalone crates never under that lint.

## 2. What actually works (verified)
— EXECUTED: `npx --yes -p @ast-grep/cli@0.51.3 ast-grep scan --rule scripts/quality/rules/unwrap-outside-tests.yml --json crates apps tools tests` -> 38 hits (version 0.51.3 — the same pin as both drivers).
— True positive proven: hit #1 is the deliberate canary at crates/_template/src/lint_canary.rs:30 (`maybe.unwrap() // clippy::unwrap_used — denied via [workspace.lints]`), gated `#[cfg(lint_canary)]` in lib.rs:18 — the rule catches it regardless of cfg, as a text-level scanner should.
— Workspace-lint claim verified: Cargo.toml `[workspace.lints.clippy]` `unwrap_used = "deny"` / `expect_used = "deny"` — the message's premise is real.

## 3. Findings
F-001 [S3] — The `expect` pattern matches any method named expect; 12 of 38 hits are false positives.
Where: scripts/quality/rules/unwrap-outside-tests.yml:7-10
Code:
```
rule:
  any:
    - pattern: $RECV.unwrap()
    - pattern: $RECV.expect($MSG)
```
Executed evidence (from the run): `tools/xtask/src/cmd/compat_matrix.rs:1059 self.expect(b'{')`, `tools/xtask/src/cmd/config_validator.rs:786 self.expect(b'{')`, `tests/integration/first-vertical/tests/baselines_consistency.rs:197 self.expect(b'{')` (+ 9 more of the same shape) — these are custom `fn expect(&mut self, b: u8)` byte-parser methods that RETURN the matched byte; they cannot panic on None/Err. What is wrong: 12/38 (32%) of the rule's output misdirects triage toward non-panicking code. What correct looks like: type-aware pattern (ast-grep metavariable constraints on the receiver type) or restrict the pattern to `Option`/`Result` receivers. Confidence: HIGH — Verification: ran (full hit list) + read (the expect methods in xtask are hand-written parser helpers).
F-002 [S4] — Message overstates scope: vta-harness hits are not "escapes" from the workspace lint.
Where: scripts/quality/rules/unwrap-outside-tests.yml:4-6
Code:
```
message: >-
  unwrap()/expect() in non-test code: any None/Err becomes a panic in the
  shipped path. Workspace lint denies this; hits are escapes from it.
```
What is wrong: 25 of 38 hits are in tools/vta/* — standalone crates with their own `[workspace]` (ruopus/dnx/ac3/vp9), which are not compiled under the repo's [workspace.lints] at all; they are not escapes, they are out of scope. "Shipped path" is wrong for them too (harnesses, never shipped). Confidence: HIGH — Verification: ran (hit list) + read (tools/vta/ruopus/Cargo.toml `[workspace]`).
F-003 [S4] — File-level test context unrecognized: `tests/`-directory files are flagged.
Where: scripts/quality/rules/unwrap-outside-tests.yml:11-22 (the exemption)
Code:
```
    not:
      inside:
        stopBy: end
        any:
          - kind: mod_item
            has:
              kind: identifier
              regex: '^tests?$'
```
What is wrong: the exemption knows inline `mod tests` and `#[test]`-attributed functions (and, in practice, `#[cfg(test)]` via the attribute regex), but a whole test FILE under `tests/integration/first-vertical/tests/` (baselines_consistency.rs) is not recognized as test scope — 4 false positives of the F-001 class are additionally in test context. The scan targets include `tests` in the path list (astgrep_scan.ps1:12, drivers), so this occurs in every run. Confidence: HIGH — Verification: ran (4 hits in that file).

## 4. Per-pattern table
| Pattern | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `$RECV.unwrap()` | 9 | REAL | — | all true positives (canary + harnesses) |
| `$RECV.expect($MSG)` | 10 | SHALLOW (name-match only) | S3 | 12 false positives (custom methods) |

## 5. Test honesty
N/A (rule). Executed directly — stronger than a test. Deterministic re-run possible with the same npx pin.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "Workspace lint denies this" | rule message L6 | REAL | Cargo.toml [workspace.lints.clippy] unwrap_used/expect_used = deny |
| "hits are escapes from it" | rule message L6 | PARTLY FALSE | 25 vta hits were never under the lint (standalone crates) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
ast-grep CLI 0.51.3 (pinned in both drivers); rule language Rust; severity warning.

## 9. Open questions for the next agent
- CROSS-FILING for the 03_astgrep.sh owner: on hosts without real ast-grep, common.sh's `sg_run` fallback `command -v sg` grabs util-linux's `sg` (a login utility) — on THIS sandbox /usr/bin/sg is util-linux 2.41.5, not ast-grep. The npx fallback is then skipped. That's a live bug in the driver, not this rule.
