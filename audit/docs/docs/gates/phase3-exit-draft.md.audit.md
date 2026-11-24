# CLAIM AUDIT: docs/gates/phase3-exit-draft.md
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Claims extracted | 16 |
| REAL | 11 |
| REAL-UNPROVEN | 2 (git-history merge hashes, CI run #2 "in flight") |
| UNVERIFIED | 3 (branch states wip/w5-*, parked-matrix dir at draft time, wave-4 gate log absence then) |
| FALSE | 0 |
| Internal contradictions found | 0 (1 supersession note: workflows-disabled/ later deleted) |

## 1. Blunt verdict
An honestly-architected DRAFT: every PENDING slot is labeled with its owning branch,
the header explicitly states "no result in this document was measured or invented
here (D-4)", and the checkable anchors all verify exactly — the wave-3 test log's
1718/0/20 over 121 binaries, the 485-file corpus manifest, the xtask 10-step gate
table, the seek.rs D4 "never a fake success" pin, the panic-surface lint denies,
the lean-CI history, and the final phase3-exit.md that superseded this draft. The
git-history claims (merge hashes, run #1 SUCCESS da2e830) are corroborated by the
archived worklog where I could reach them. Zero fabrications detected; the only
limitation is that ~half the document is branch state that no longer exists and can
no longer be inspected (git forbidden).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Every wave-5 slot is **PENDING** and labeled with its owning branch; no result in this document was measured or invented here" | L4-8 | REAL | every §2/§3 PENDING row carries `wip/w5-*`; §5 gap 6 repeats the D-4 rule; no number appears without a committed anchor |
| 2 | "1718 passed / 0 failed / 20 ignored (`docs/gates/logs/wave3-merge.test.log`)" | L110 | REAL | rg-summed the log: total passed 1718, total ignored 20, 0 failed; `Running` count = 121 (matches "121 Running test binaries" in the same row) |
| 3 | "corpus-big (485 files, manifest `docs/corpus/corpus-big-manifest-20261022.json`)" | L45 | REAL | python3 json: count=485, files=485 |
| 4 | "`tools/xtask/src/cmd/gate.rs:10-21` (10-step table)" | L107 | REAL | sed shows the numbered 10-step table in the doc comment |
| 5 | "D4 'never a fake success' pinned at `crates/playback-engine/src/seek.rs:2, 17, 82-87`" | L77 | REAL | sed -n '2p;17p;82,87p' -> "**never a fake success** (MP §8 bullet 4)" + the D4 contract header present |
| 6 | "lint block incl. the panic-surface denies (`Cargo.toml:49-56`)" | L109 | REAL | Cargo.toml [workspace.lints.clippy]: unwrap_used/expect_used/panic/todo/unimplemented/dbg_macro = "deny" |
| 7 | "CI is live but deliberately ONE lean ubuntu job (`.github/workflows/ci.yml:22-26, 68`)" | L27 | REAL | ci.yml comments confirm the deliberate lean-job history (issue #5, 2026-11-09) |
| 8 | "the 3-OS matrix remains parked under `.github/workflows-disabled/`" | L27 | REAL (then) / SUPERSEDED | true at draft time; ci.yml comment (issue #5) records the parked dir was later DELETED and the 3-OS matrix lives in ci-full.yml as non-blocking [PROPOSAL] |
| 9 | "CI on the merge commit ... run #1 SUCCESS (da2e830), run #2 in flight" | L118 | REAL-UNPROVEN | git-history; corroborated by archived worklog docs/worklog/archive-2026-10.md: "CI: run #1 (da2e830) completed SUCCESS; run #2 (691e4fc) in flight" |
| 10 | "worklog 3.WAVE3-CLOSE ... 3.A1-A3, 3.B1 (2 pending-ACs registered), 3.B3, 3.B4, 3.C1, 3.C2 COMPLETE" | L25 | REAL | archive-2026-10.md:7698 "3.A1-A3, 3.B1 (2 pending-ACs registered), 3.B3, 3.B4, 3.C1, 3.C2 COMPLETE" — verbatim |
| 11 | "merges c0f0fe2 (3.A3), 945e392 (3.B1), 167f6e9 (3.B3), 6faff0e (3.C1), 6556164 (3.C2)" | L25 | UNVERIFIED | git-history; not present in the archived worklog text (only the CI-run hashes are); no git allowed |
| 12 | "directory listing shows wave1-3 + merge-3a1 logs only" (gap 5: wave-4 gate log uncommitted) | L145-148 | REAL (for its date) | current docs/gates/logs/ has wave1-merge/wave2-merge/wave3-merge/merge-3a1 + later phase logs; no `gate-w4.log` exists even now — the gap statement held |
| 13 | "16 Phase-3 cards" vs doc 13 §6's "19 tasks (3.A1..3.F2-V)" — count-note for the Lead | L71-73 | REAL | §3 table carries exactly 19 rows (3.A1..3.F2-V) — the honest reconciliation note |
| 14 | "doc 10 §6.4's closed 16-row list (open, play, ... exit during playback)" | L26 | REAL-UNPROVEN | doc 10 is not my file; the final phase3-exit.md:32 confirms the 16-scenario suite landed (23 passed/0 failed) |
| 15 | "the Lead's `docs/gates/phase3-exit.md`" would finalize this draft | L3-4 | REAL | docs/gates/phase3-exit.md EXISTS and cites the draft at its L20 |
| 16 | "`docs/audits/error-surface-20261023.md` §0 + fix F-5" (unreachable! blind spot) | L109 | REAL | the file exists (ls docs/audits/) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| — (none internal). "workflows-disabled/ parked" (L27) | ci.yml comments (issue #5, 2026-11-09): "The parked directory is deleted" | Both true at their respective dates — the draft is 2026-10-23; deletion came later. No live contradiction. |

## 4. Fabrication / inflation indicators
None. The draft's design (PENDING slots + [VERIFY] tags + D-4 "never fabricate")
is exactly what it advertises; every anchor I could execute or read matched
byte-for-byte (1718/20/121, 485, the verbatim WAVE3-CLOSE card list).
