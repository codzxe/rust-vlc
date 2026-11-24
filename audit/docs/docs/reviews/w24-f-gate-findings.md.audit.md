# CLAIM AUDIT: docs/reviews/w24-f-gate-findings.md
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 10 |
| REAL 7 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A review-of-review whose own numbers I re-verified exactly: the test-log recount (306
`test result:` lines, 4418/4/49, TEST_EXIT=101), the deny wildcard count (30, and the
gate report's 32 was indeed wrong), the MF retraction in hw/mod.rs, and the OF-5 honesty
fix in the matrix (present today with a self-citing correction note). Its OF-1 conflict
markers are gone from App.tsx today (fix landed). The only unverifiables are
historical-git claims (commits/worktrees I cannot resolve without running git) and the
since-deleted scripts/cargo-env.sh. High-quality review; no fabrication.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | test log recount: 306 `test result:` lines, 4418 passed / 4 failed / 49 ignored, exit 101, 4 failing targets | :46, :172-175 | REAL | python3 over docs/gates/logs/phase7-exit-test.log: 306 anchored lines, sums 4418/4/49; `TEST_EXIT=101` at :6399 — all exact |
| 2 | P3-1: gate report's "32 wildcard warnings" is wrong; the deny log contains 30 | :59-65 | REAL | rg -c '^warning\[wildcard\]' docs/gates/logs/phase7-exit-deny.log -> 30 — exact |
| 3 | OF-5: matrix note claimed VT/MF/VA-API "landed … compile-verified" while MF was retracted (no MF code ever existed) | :224-230 | REAL (fix confirmed in-tree) | docs/feature-matrix.toml:416 now carries the correction verbatim: "2026-11-10 (gate OF-5 honesty fix, issue #51): this note previously claimed VT/MF/VA-API all landed and compile-verified … — the Media Foundation claim was false (no MF code ever existed …); corrected to match the hw/mod.rs truth table"; hw/mod.rs:51 "**Media Foundation** (Windows, 7.B2): **NOT IMPLEMENTED**" |
| 4 | OF-1: conflict markers at ui/src/App.tsx:15-24, :100-104 (P0; UI typecheck broken) | :206-211 | REAL (fixed since) | rg '<<<<<<<\|>>>>>>>' ui/src/App.tsx -> 0 matches today (fix-in-flight the review verified landed) |
| 5 | OF-2: settings pin drift — schemas/settings.toml 27 vs 32 entries, tests assert 27 | :212-215 | REAL-at-time (historical) | schemas/settings.toml now has 121 `key =` rows (more settings landed since); the old `assert_eq!(schema.len(), 27` is gone from settings-schema/src/lib.rs — the pin-bump fix path resolved differently |
| 6 | O-1: scripts/cargo-env.sh rewritten by w24/a sets CARGO_TARGET_DIR=/tmp/my-project/target | :147-148 | UNVERIFIED | scripts/cargo-env.sh does not exist in today's tree (renamed/removed later); claim was historical |
| 7 | 11 phase7-exit-*.log files exist and are referenced; task brief's "12 logs" was the discrepancy | :159-163 | REAL | ls docs/gates/logs/ -> 11 phase7-exit-* files present |
| 8 | all 21 other commits cited in §3/§4 verified to exist with matching subjects | :73-74 | UNVERIFIED | requires git history access (forbidden for agents); not re-checkable |
| 9 | "no secrets, tokens, credential-bearing URLs, or private keys" in all 11 logs | :280-282 | REAL (spot) | rg for token/secret/private key patterns over docs/gates/logs/phase7-*: no hits (spot-check, not full 11-log scan — 3 logs sampled in depth: test, deny, xtask) |
| 10 | the gate report's VT/MF/VA-API labels "conservative and correct", no overclaim found | :14-16 | REAL | corroborated by the matrix's own OF-5 correction note (claim 3) and by the intra-matrix state today (see §3 below) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none internal) | — | — |
| Cross-matrix note (not this doc's contradiction, discovered during this audit): the codec-video-h264 row (docs/feature-matrix.toml:414-417) says `partial` (VT/VA-API compile-verified lanes) for the SAME crates/codecs/src/video/h264/hw/** substrate that my chunk's row #174 platform-hwdecode-h264 (:2427) calls `not_started` on every platform. Both cannot be right; the codec row (partial, rich notes, OF-5-corrected) is the accurate one, making row 174's "no work yet" even more clearly false. Filed as F-MATRIX-174 addendum in audit/docs/feature-matrix.toml.rows-170-182.audit.md |

## 4. Fabrication / inflation indicators
- None — this review *found* the one overclaim (OF-5, the false MF "compile-verified"
  claim) and forced its correction; the correction is verbatim in the matrix today.

## 5. Matrix rows
Touches matrix row codec-video-h264 (:413-417): its OF-5 finding + fix verified REAL.
The parallel platform-hwdecode-h264 row contradiction is logged in my chunk audit.

## 6. Open questions for the next agent
- Row-174 vs codec-row-416 status split for the same hw code needs Lead reconciliation
  (F-MATRIX-174, severity S2).
