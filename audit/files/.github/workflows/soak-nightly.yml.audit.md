# AUDIT: .github/workflows/soak-nightly.yml
| Field | Value |
|---|---|
| Auditor | W17-A04 |
| Date | 2026-11-24 |
| Lines / jobs | 265 / 3 (soak, soak-scenarios, soak-lsan) |
| File verdict | REAL (valid YAML, real refs) — zero execution history (never ran; CI billing-blocked) |
| Reachable from shipped entry point? | N/A (CI config); triggered by `schedule: 0 3 * * *` + `workflow_dispatch` only — can never gate a merge |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 yaml parse -> OK (3 jobs); ls scripts/soak-scenarios.sh tests/soak/scenarios-nightly.toml -> exist; grep ci.yml pins -> match; grep KNOWN-LIMITS.md R-16 -> found |

## 1. One-paragraph verdict
This workflow CAN fail and is built so a failure stays visible: no `continue-on-error`, real `cargo test` invocations, timeouts (300/300/120 min), `upload-artifact` with `if: always()`. But it has never executed even once — the file itself says so, and KNOWN-LIMITS.md R-16 corroborates (CI billing-blocked, "every Actions run dies at 0 steps"). So the nightly soak tier, the scenario tier, and the LSAN tier are all paper CI: real plumbing, zero runs. The honesty labeling is unusually good ([PROPOSAL], [VERIFY], [PENDING first real 4 h run], PROVEN-locally-with-paths), and every referenced artifact I checked exists. It is non-blocking by trigger shape (schedule-only), so it can't fail a release — but it also can't catch anything.

## 2. What actually works (verified)
— `python3 -c yaml.safe_load(...)` -> YAML OK, jobs: soak, soak-scenarios, soak-lsan; triggers: schedule, workflow_dispatch.
— Action pins cross-checked against ci.yml:90-93 — identical SHAs (checkout 3d3c42e5…, dtolnay d124f86f…, rust-cache 6323deb…, upload-artifact cf430e03…).
— Harness env contract matches the invocation: crates/playback-engine/tests/soak/main.rs:13-19 documents `SOAK_MINUTES`/`SOAK_LEAK_MB_PER_H`/`SOAK_REPORT` exactly as the workflow sets them; L104-111 bounds-check the minutes.
— Wrapper exists and propagates cargo's exit code (scripts/soak-scenarios.sh tail: `rc=$?; echo ...; exit "${rc}"`) — so the soak-scenarios job genuinely fails on budget violations.
— Sandbox evidence committed: docs/soak/raw/ has 5 scenario reports + calibration + forced-threshold; docs/perf/soak-nightly-20261109.md records the real 2-min CI-short profile.

## 3. Findings
F-001 [S3] — The entire nightly tier has zero execution history; nothing anywhere enforces soak behavior.
Where: .github/workflows/soak-nightly.yml:57-61
Code:
```
# HONESTY NOTE — NONE OF THESE JOBS HAS EVER EXECUTED: CI is
# billing-blocked on this private repository (every Actions run dies
# at 0 steps; docs/KNOWN-LIMITS.md R-16). Every command shape below
# is proven locally where the sandbox allowed it — see the job
# headers for exactly what is proven vs [VERIFY].
```
What is wrong: Honest, but the effect is that the 4 h/5 MB/h design target (L9) has never been measured by this workflow and never will until billing unblocks; the "nightly tier" is a plan, not a running control. The only real soak evidence is 2-min CI-short and ~34-min sandbox runs. Why it matters: memory-leak regressions are undetected between releases. What correct looks like: run it once when billing unblocks, and pin the measured slope in docs/perf/. Confidence: HIGH — Verification: read (R-16 at docs/KNOWN-LIMITS.md:86) + repo checks; the "0 steps" claim itself is UNVERIFIED (no CI access from sandbox).
F-002 [S4] — Duplicated toolchain-assert step.
Where: .github/workflows/soak-nightly.yml:112-123 vs 174-185
Code (both jobs, byte-identical logic):
```
          pin="$(sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml)"
          inst="${{ steps.toolchain.outputs.name }}"
          eff="$(rustc --version | sed -n 's/^rustc \([^ ]*\) (.*)$/\1/p')"
```
What is wrong: Copy-pasted assert block in `soak` and `soak-scenarios`; a future pin bump must be edited twice (the error message at least names this file in both copies, so no wrong-file bug). Confidence: HIGH — Verification: read + diff of the two blocks.

## 4. Per-job table
| Job | Lines | Can it fail? | Verdict | Notes |
|---|---|---|---|---|
| soak | 100-148 | YES (cargo test exit propagates; timeout 300) | REAL-UNPROVEN (never executed) | 4 h @ 5 MB/h ceiling = extrapolation, labeled [PROPOSAL] |
| soak-scenarios | 164-203 | YES (wrapper `exit "${rc}"`) | REAL-UNPROVEN (never executed) | command shape proven at sandbox scale per docs/soak/raw/ |
| soak-lsan | 234-265 | YES (LSAN exit 23 wrapped in 101) | REAL-UNPROVEN (never executed; not even sandbox-runnable) | nightly toolchain off the stable pin — documented, skips the pin-assert on purpose |

## 5. Test honesty
N/A (workflow). Local "proven" claims cross-checked to committed artifacts (docs/soak/raw/*, docs/perf/soak-nightly-20261109.md §3 "The REAL 2-min CI-short profile").

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "NONE OF THESE JOBS HAS EVER EXECUTED" | soak-nightly.yml:57 | REAL (consistent w/ R-16; direct CI state UNVERIFIED) | docs/KNOWN-LIMITS.md:86 |
| "SOAK_MINUTES drives the REAL wall duration" | soak-nightly.yml:6-7 | REAL (code read; not executed here) | tests/soak/main.rs:13-19 |
| "all five scenarios green [sandbox]" | soak-nightly.yml:155-158 | REAL-UNPROVEN | docs/soak/raw/ 5 reports exist; not re-run (cargo-build-budget) |
| "5 MB/h … first run may or may not bless" | soak-nightly.yml:11-15 | honest-unknown | no 4 h measurement exists anywhere |

## 7. Dead / duplicated / unreachable code
The toolchain-assert duplication (F-002). Nothing dead: every step is live YAML, just never triggered. Note `cron: "0 3 * * *"` (L76) is live config.

## 8. Dependencies & flags
Pinned actions identical to ci.yml (verified). CARGO_TARGET_DIR=target overrides committed .cargo/config.toml (L95-97 — documented). soak-lsan sets RUSTFLAGS `-Z sanitizer=leak` + LSAN_OPTIONS detect_leaks=1 (L238-240).

## 9. Open questions for the next agent
- Has ANY workflow in this repo ever completed a run? The worklog archive mentions "run #1 (da2e830) completed SUCCESS" (archive-2026-10.md:7693) — that predates/contradicts the "every Actions run dies at 0 steps" blanket claim. Reconcile which is true (R-16 vs. successful CI runs recorded in worklog).

## ADDENDUM (post-audit cross-reference, 2026-11-24)
worklog.md:1480 records "Actions billing unblocked — workflows now execute" (run 43189096326 era), and rounds 11/15 + tag v0.1.0 ran green. So this file's L57-61 "NONE OF THESE JOBS HAS EVER EXECUTED / every Actions run dies at 0 steps" and the cited R-16 are STALE on the current tree (unblocked ~2026-11-13). Whether the soak jobs themselves have since executed on the 03:00 nightly schedule remains UNVERIFIED (no CI access). Also stale: L95-97's comment claims .cargo/config.toml pins "the sandbox-path build.target-dir" — the committed config says "Deliberately NO [build] target-dir" (.cargo/config.toml comment block). Upgrades F-001 context; severity stands (no soak-run evidence exists either way).
