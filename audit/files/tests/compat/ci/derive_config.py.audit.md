# AUDIT: tests/compat/ci/derive_config.py
| Field | Value |
|---|---|
| Auditor | W16-A05 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Lines / functions | 187 / 5 (fail, load_committed_config, live_claims, derive_lanes, main) |
| File verdict | REAL (executed by this audit: happy path + 2 refusal paths, exit codes verified) |
| Reachable from shipped entry point? | N/A — CI tooling; invoked by .github/workflows/compat-matrix.yml:151 (`"$PY" tests/compat/ci/derive_config.py …`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 derive_config.py linux <audit path> (exit 0); platform=web (exit 1); argc=3 (exit 1); rg workflow consumer |

## 1. Verdict
A small, loud, check-first CI config deriver that does exactly what its docstring claims — I ran
all three paths myself. Claims are refreshed from the live feature-matrix, the native lane is
re-attributed explicitly, this platform's blocked lane is dropped, and the output schema stays
compat-matrix/config/2. No silent failure path exists: every problem calls fail() -> exit 1
before any file is written.

## 2. What actually works (verified — executed this run)
- Happy path: `python3 tests/compat/ci/derive_config.py linux audit/runs/W16-RR1.derived-config.json
  audit/runs/rr1-reports` -> exit 0; log: `claims refreshed from the live
  docs/feature-matrix.toml (35 rows)`; `lanes = linux-native, web-wasm, windows, macos`.
- Output inspected (python3 json): claims[0] carries all four platform columns straight from
  the live matrix; the native-decode lane is `platform: linux` (id linux-native); the linux
  blocked lane is DROPPED (only windows/macos blocked remain, each with the sibling-job
  citation); report paths point under the reports dir. Matches the docstring's four bullets.
- Refusals: `platform=web` -> exit 1 with "platform must be one of windows/linux/macos …
  `web` is the steps lane"; argc != 4 -> exit 1 printing full usage to stderr. Loud, nothing
  written (the happy-path write happens only after all checks).
- tomllib import guard fails loudly on <3.11 (:52-56); file is invoked via "$PY" in the
  workflow (not the executable bit — mode is 644, irrelevant).

## 3. Findings
None. Zero-findings list (checked, how): schema pinned and checked (:78-79); JSON/TOML parse
errors both loud (:74-77, 90-91); claim rows absent from the live matrix fail with the row
list (:94-99); missing platform columns in a matrix row fail (:103-105); exactly-one-native
lane invariant enforced (:141-142); no shell usage (no injection surface); output written only
after validation; no environment variable reads; committed evidence never touched (verified:
only out_config's parent is makedirs'd — the reports dir is referenced, not written).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fail | 65-67 | REAL | — | stderr + exit 1 |
| load_committed_config | 70-80 | REAL | — | schema pin checked |
| live_claims | 83-107 | REAL | — | drop/refresh/validate columns |
| derive_lanes | 110-143 | REAL | — | re-attribute, drop own blocked lane, invariant |
| main | 146-182 | REAL | — | argv validation, writes only after checks |

## 5. Test honesty
No tests exist for this script itself (the compat crate's Rust tests cover the runner's
report generation, not this deriver). I executed it directly — the evidence above IS the run.
UNVERIFIED: the full CI lane (compat-matrix.yml) executing end-to-end on a hosted runner.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "claims are REFRESHED from the LIVE docs/feature-matrix.toml" | :20-29 | REAL | my run: 35 rows refreshed |
| "the blocked lane whose id equals <platform> is DROPPED" | :33-36 | REAL | linux lane absent from output |
| "nothing is written when a check fails" | :44-45 | REAL | refusal paths exited before write |
| "web is the steps lane, never a native sweep — refused" | :9-11 | REAL | exit 1 on web |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
stdlib only (json/os/sys/tomllib). No env vars read.

## 9. Open questions for the next agent
None blocking. Cross-ref: tests/compat Rust unit tests are dead-in-CI per W16-A02's audit of
report.rs (F-001 S3) — this script, by contrast, IS wired into compat-matrix.yml:151.
