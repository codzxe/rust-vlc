# CLAIM AUDIT: docs/gates/phase5-exit.md
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| Verdict counts | TRUE: 3 TRUE-AT-TIME/RESOLVED-LATER: 2 UNVERIFIED: 6 FALSE-NOW (dated TRUE-then): 1 |
| Internal contradictions found | 0 |

Note: W17-A10's earlier copy sits at audit/docs/gates/phase5-exit.md.audit.md
(missing the `docs/` path segment); this is the canonical recovery copy,
re-verified by W17-R1 (logs dir re-listed, perf files re-listed, baseline
rows re-grepped, worklog cross-refs re-grepped).

## 1. Blunt verdict
A dated phase-exit snapshot (Lead sign-off 2026-10-24) whose evidence
partially survives and partially does not. What checks out TODAY: the
fuzz-registry gate it cites is real and green in the orchestrator baseline
(35 targets now vs 32 claimed then — clean evolution), all four perf/audit
evidence files exist (breadth.md, algorithm-audit.md, memory-audit.md,
phase5-records-audit.md), and its honest-caveat list is exactly the list
that was later worked off (TrueHD's unblock decision record landed —
worklog.md:354 "truehd-bitstream-io-unblock (+117)"; the open-path caveat
is obsolete — the current baseline MP4 smoke opens H.264+AAC MP4 through
`lumen open`, Opening→Playing→Stopped, exit 0). What does NOT survive: the
central gate claim "3213 passed / 0 failed" has NO committed log — its
evidence-chain citation is "this session", and docs/gates/logs/ contains
zero phase-5 run logs (verified: only phase3/phase4/phase7/merge logs) —
while the current tree is 4744 passed / 2 failed. fmt "clean" is likewise
TRUE-then/FALSE-now (baseline exit 1). No fabrication indicators — the doc
flags its own gaps — but the green rows rest on an unverifiable session
narrative.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "every number below is a real run logged in `worklog.md` and `docs/gates/logs/` where applicable" | :5-6 | PARTIAL | `ls docs/gates/logs/` -> 28 logs, NONE phase-5 (phase3/phase4/phase7/merge-*/rebrand only); test/clippy/fmt rows rest on "this session" |
| 2 | "`cargo test --workspace --all-features` — 3213 passed / 0 failed" | :12 | UNVERIFIED / STALE | no in-tree phase-5 log; current baseline: 4744 passed / 2 failed (cargo-test-workspace.txt, summed by awk: "sum passed: 4744") |
| 3 | "`cargo clippy ... -D warnings` — 0 warnings" | :13 | TRUE-CORROBORATED-NOW | baseline full_gate.txt:31 `clippy: CLEAN` on the current tree |
| 4 | "`cargo fmt --all -- --check` — clean" | :14 | FALSE-NOW (TRUE-then) | baseline full_gate.txt:29 `fmt: FAILED` (cargo-fmt.txt exit 1) |
| 5 | "fuzz registry diff EMPTY (32 targets ...); in-crate survival suites 1393/0" | :15 | TRUE-mechanism / count evolved / 1393/0 UNVERIFIED | baseline full_gate.txt:57 `fuzz-registry: CLEAN — 35 targets, diff empty, corpora present`; 1393/0 has no committed log |
| 6 | "records audit: 40 records, 83 format rows, zero missing" | :16 | UNVERIFIED | docs/gates/phase5-records-audit.md exists; current tree: docs/dependencies/records = 22 files + docs/formats = 53 — the 40/83 definitions live inside that audit doc (other agent's domain); not reconcilable by count alone |
| 7 | "perf snapshot — `docs/perf/breadth.md` + raw CSVs `docs/perf/raw/r1/`" | :17 | TRUE-existence | ls -> breadth.md + raw/ present; contents not audited here |
| 8 | "algorithm audit (5.X1) hotspot tables / memory audit (5.X2) before/after tables" | :18-19 | TRUE-existence | docs/perf/algorithm-audit.md + memory-audit.md (+ memory-audit-20261023.md) all present |
| 9 | Caveat 2: "TrueHD (K1) admission blocked ... OOM-kills compiling bitstream-io 4.10.0's macro expansion ... Unblock = upstream fix or Lead-approved ~60-line vendor patch" | :58-61 | TRUE-AT-TIME / RESOLVED-LATER | the decision record landed: worklog.md:354 "truehd-bitstream-io-unblock (+117)"; my File-5 audit verified it byte-matches parent record + matrix row still blocked x4 |
| 10 | Caveat 3: "the CLI's `open` still routes the D-018 first vertical only; per-container host wiring ... is the next card" | :62-64 | TRUE-AT-TIME / RESOLVED-LATER | baseline mp4-smoke: `lumen open` on H.264+AAC MP4 walks Opening→Playing→Stopped, exit 0 (mp4 container now routes; decode still first-vertical codecs per engine-info) |
| 11 | Caveat 1: "VLC oracle absent sandbox-wide — every differential is vs ffmpeg 7.1.5" | :56-57 | TRUE-consistent | every differential record audited this recovery batch (ac3 README, cpal, truehd parent) references ffmpeg as oracle |
| 12 | "47 parent cards, all resolved" | :21 | UNVERIFIED | card ledger is worklog narrative; no machine-checkable ledger in the doc |

## 3. Contradictions
None internal. Cross-time note: the doc's green rows (fmt clean, 0 failed)
vs the CURRENT baseline red rows are 2026-10-24 → 2026-11-23 evolution
drift, not contemporaneous lying.

## 4. Fabrication / inflation indicators
None visible. The caveats section is the opposite of inflation — VLC oracle
absent, OOM-blocked TrueHD, unwired open-path, mp2 ±1 LSB class, AV1 policy
gap, 3-OS/soak/nightly infra-blocked are all disclosed. The structural
weakness: three green gate rows carry no committed logs ("this session") —
for a doc that cites logs "where applicable", the applicability judgment is
the author's.

## 5. Matrix rows
N/A — phase gate doc.
