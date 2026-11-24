# AUDIT: scripts/quality/20_formal.sh
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 33 / 3 phases (20a availability, 20b catalogue, 20c conditional run) |
| File verdict | PAPER-ONLY for the Kani half (catalogue prose; zero harnesses exist); the surrounding formal tier's Miri half is real wiring |
| Reachable from shipped entry point? | YES — `.github/workflows/audit.yml:279` "Kani catalogue (harnesses to write; no proofs yet)" runs `bash scripts/quality/20_formal.sh || true` (formal job, nightly, continue-on-error). NOT wired into 99_run_all.sh's tiers. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | grep kani-harness markers over crates/ (0 hits); audit.yml formal job read (:258-282); 18_ub_checks.sh miri-mode interface check |

## 1. One-paragraph verdict
This script is an aspiration with a runner's shape. Its Kani half cannot do work today: zero `cfg(kani)` harnesses exist anywhere in crates/ (verified by grep), the run branch requires an argument no caller ever passes (the CI formal job invokes it bare, and 99_run_all.sh does not include it at all), and the CI step is wrapped in `|| true` inside a continue-on-error job. Everything it would check is therefore still prose — an 8-row catalogue of harnesses "to write in the fix program". To its credit, every one of those facts is self-labeled honestly ("no proofs yet", the not_run ledger records kani). The formal tier's OTHER half — Miri over media-types/codec-api/packet-queue via 18_ub_checks.sh's real `miri [crate]` mode — is genuine wiring, though CI-billing-blocked from ever having run.

## 2. What actually works (verified)
— verified by interface check: scripts/quality/18_ub_checks.sh:3/30-35 supports `miri [crate]` (`cargo +nightly miri test` with per-crate PKG_FLAG, log to $OUT/ub/miri.log) — the formal job's Miri calls (audit.yml:272-277) are real commands.
— verified by read: 20a's `need cargo-kani` path records a not_run entry with the install recipe (common.sh:36-43) — the "not run" is logged, never invented as pass.
— bash -n clean (trivial file).

## 3. Findings
F-001 [S3]
Where: scripts/quality/20_formal.sh:27-32
Code:
```
if command -v cargo-kani >/dev/null 2>&1 && [ "$#" -gt 0 ]; then
  cargo kani --harness "$1" --output-format terse 2>&1 | tee "$OUT/kani-$1.log" | tail -30
else
  not_run "kani" "no harness committed yet and/or kani not installed - harnesses are source changes (fix program)"
fi
```
What is wrong: the run branch is dead code in every real invocation: (a) no Kani harness exists in the tree (`grep -rln 'cfg(kani)|kani::proof' crates apps` -> no files), so there is no `$1` a caller could usefully pass; (b) the only caller (audit.yml:279) passes no arguments; (c) 99_run_all.sh wires no tier to it. The script is a catalogue printer with an unreachable-if-kani-absent branch. Why it matters: the quality program's "formal verification" tier is, at the Kani level, documentation — a reader of the tier list could believe proofs run nightly. What correct looks like: either land the first harness or demote the tier name to "formal (Miri only; Kani pending)". Confidence: HIGH — Verification: read + grep (0 harnesses). Mitigation: audit.yml's step name itself says "(harnesses to write; no proofs yet)" and not_run records it.
F-002 [S4]
Where: scripts/quality/20_formal.sh:7-12
Code:
```
if ! need cargo-kani; then
  if command -v kani >/dev/null 2>&1; :; else
```
What is wrong: the availability probe checks both `cargo-kani` and a bare `kani` binary, but the run branch (20c) only honors `cargo-kani` — if only `kani` existed, 20a stays quiet while 20c still refuses. Inconsistent probes; also when cargo-kani IS present, 20a prints nothing (no positive confirmation). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: .github/workflows/audit.yml:279 (caller)
Code:
```
        run: bash scripts/quality/20_formal.sh || true
```
What is wrong: a cannot-fail step (`|| true`) inside a continue-on-error job (:262) — the catalogue step can never redden CI. Self-labeled, but it completes the pattern: the entire Kani tier is non-blocking three ways over. Confidence: HIGH — Verification: read.

## 4. Per-function table (file <100 LOC; phases)
| Phase | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| 20a availability | 6-12 | REAL (probe) | S4 | inconsistent kani/cargo-kani probes (F-002) |
| 20b catalogue | 14-25 | PAPER-ONLY | S3 | 8 harnesses to write; 0 exist today |
| 20c conditional run | 27-32 | UNREACHABLE in practice | S3 | dead branch (F-001); honest not_run fallback |

## 5. Test honesty
Nothing to test — the script contains no verification of anything. Its honesty instrument is the not_run ledger (which fires on every real invocation). The catalogue's properties (overflow totality, monotonicity, bit-reader bounds) are the RIGHT properties to want — they simply are not written.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Kani catalogue (harnesses to write; no proofs yet)" | audit.yml:279 | REAL (accurate self-description) | grep: 0 kani harnesses in tree |
| "T20 - bounded model checking with Kani (proofs, not samples)" | 20_formal.sh:2 | SHALLOW (title promises what the body cannot yet do) | same grep |

## 7. Dead / duplicated / unreachable code
20c's run branch is unreachable in all current wirings (F-001). No duplication.

## 8. Dependencies & flags
cargo-kani (never installed in CI formal job — no install step for it); writes $OUT/kani-*.log only on the dead branch; not_run writes the ledger.

## 9. Open questions for the next agent
- Has the Miri half (18_ub_checks.sh miri over 3 crates) ever actually run? CI billing-blocked; no committed docs/audit/reports/ub/miri.log in tree => UNVERIFIED, likely never.
- The 8-row catalogue overlaps 17_proptests.sh's 15-row wishlist (4 shared properties) — two aspiration lists for the same fix program, neither started. S4-level drift risk if they diverge.
