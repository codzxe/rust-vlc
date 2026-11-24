# CLAIM AUDIT: docs/gates/phase5-exit.md
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Claims extracted | 12 |
| Verdict counts | TRUE: 3 TRUE-AT-TIME/RESOLVED-LATER: 2 UNVERIFIED: 6 FALSE-NOW(but dated): 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A dated phase-exit snapshot (Lead sign-off 2026-10-24) whose evidentiary
artifacts partially survive and partially do not. What checks out TODAY: the
fuzz-registry gate it cites is real and green in the orchestrator baseline
(35 targets now vs 32 claimed then — clean evolution), the perf/audit
evidence files all exist (breadth.md, algorithm-audit.md, memory-audit.md,
phase5-records-audit.md), and its honest-caveat list is exactly the list that
was later worked off (TrueHD unblock recorded in worklog:354; open-path
container wiring landed — the current MP4 smoke opens H.264+AAC MP4 through
`lumen open`, which caveat 3 said did not exist yet). What does NOT survive:
the central gate claim "3213 passed / 0 failed" has NO committed log — the
evidence chain's citation for it is "this session" (docs/gates/logs contains
zero phase-5 run logs), and the current tree is 4744 passed / 2 FAILED, so
the "0 failed" state is neither re-verifiable nor current. fmt clean ditto:
TRUE then, FALSE now (baseline exit 1). No fabrication indicators — the doc
flags its own gaps (VLC oracle absent, policy gaps) — but the verification
burden rests on a worklog narrative that a hostile auditor cannot re-run.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "every number below is a real run logged in worklog.md and docs/gates/logs/ where applicable" | :5-6 | PARTIAL | docs/gates/logs/ contains NO phase-5 run log (ls | rg phase5 -> empty); the test/fmt/clippy rows rest on "this session" (worklog narrative) |
| 2 | "cargo test --workspace --all-features: 3213 passed / 0 failed" | :12 | UNVERIFIED / STALE | no in-tree log; current baseline: 4744 passed / 2 failed (cargo-test-workspace.txt) — count evolved +2 failures since |
| 3 | "cargo clippy ... 0 warnings" | :13 | TRUE-CORROBORATED-NOW | baseline full_gate.txt:31 `clippy: CLEAN` on the current tree |
| 4 | "cargo fmt --all -- --check: clean" | :14 | FALSE-NOW (TRUE-then) | baseline full_gate.txt:29 `fmt: FAILED`; cargo-fmt.txt diff in crates/plugin-api/tests/testkit_wiring.rs:40 |
| 5 | "fuzz registry diff EMPTY (32 targets)" | :15 | TRUE (mechanism) / count evolved | scripts/fuzz_registry_check.sh exists; baseline full_gate.txt:57 `fuzz-registry: CLEAN — 35 targets` — 32→35 consistent growth; in-crate 1393/0 UNVERIFIED |
| 6 | "records audit: 40 records, 83 format rows, zero missing" | :16 | UNVERIFIED | docs/gates/phase5-records-audit.md exists; current dirs: docs/dependencies/records = 22 files, docs/formats = 53 — cannot reconcile 40/83 without the audit doc's definitions (other agent's domain) |
| 7 | "perf snapshot ... docs/perf/breadth.md + raw CSVs docs/perf/raw/r1/" | :17 | TRUE-existence | all four perf/audit files exist (ls); contents not audited here |
| 8 | "algorithm audit (5.X1) hotspot tables / memory audit (5.X2) tables" | :18-19 | TRUE-existence | docs/perf/algorithm-audit.md + memory-audit.md exist |
| 9 | Caveat 2: "TrueHD ... OOM-kills compiling bitstream-io 4.10.0 ... Unblock = upstream fix or Lead-approved vendor patch" | :58-61 | TRUE-AT-TIME / RESOLVED-LATER | worklog.md:354 records `truehd-bitstream-io-unblock (+117)` — the unblock card landed post-phase-5 |
| 10 | Caveat 3: "CLI's `open` still routes the D-018 first vertical only; per-container host wiring ... is the next card" | :62-64 | TRUE-AT-TIME / RESOLVED-LATER | current MP4 smoke (spec §13 baseline): `lumen open` on H.264+AAC MP4 walks Opening->Playing->Stopped, exit 0 — container wiring exists now (decode still first-vertical codecs) |
| 11 | Caveat 1: "VLC oracle absent sandbox-wide — every differential is vs ffmpeg 7.1.5" | :56-57 | TRUE-consistent | every differential record audited this wave (ac3, cpal) uses ffmpeg as reference |
| 12 | "47 parent cards, all resolved" | :21 | UNVERIFIED | card ledger lives in worklog narrative; no machine-checkable ledger in the doc |

## 3. Contradictions
None internal. Cross-time note: the doc's green gate rows (fmt clean, 0
test failures) vs the CURRENT baseline red rows are evolution drift, not
contemporaneous lying — the doc is dated 2026-10-24 and the baseline is
2026-11-23.

## 4. Fabrication / inflation indicators
None visible. The caveats section is the opposite of inflation — it lists
the gaps (VLC oracle absent, OOM-blocked TrueHD, unwired open-path, ±1 LSB
class, AV1 policy gap, 3-OS/soak/nightly infra-blocked). The one structural
weakness: the evidence chain's three green gate rows have no committed logs
("this session") — for a phase-exit gate that cites logs "where applicable",
the applicability judgment is the author's.

## 5. Matrix rows
N/A — phase gate doc.
