# CLAIM AUDIT: docs/reviews/w24-p-fixes-findings.md (209 lines, W24-R10 audit of compat-fix lane W24-P)
| Field | Value |
|---|---|
| Auditor | W19-A08 — RECOVERED by W19-R2 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 / 0 / 0 / 0 / 0 / 0 / 3 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The audit's verified-fix claims reproduce in the current tree: the compat-matrix's avi
and wav cells carry the in-row refusal clauses byte-exactly as the audit describes,
the 7 new report tests exist under the exact names it lists, and the census it
recomputed (166 rows; 84 decoded / 22 partial / 6 refused / 0 demux-fatal; 54
malformed all-typed) recomputes exactly from the committed raw-rows.jsonl. Its
"not addressed" honesty column records the P3s left undone — the anti-inflation
pattern for a fix-lane audit.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | md §3 `avi` cell → "41/43 files open+demux …; 2 extension-matched corpus files REFUSED at the open path — unsupported in avi (demux.avi.no-supported-streams) ×2" | :66-70, :79 area | REAL | compat-matrix-20261110.md:79 — the cell is byte-verbatim (sed-verified this run). |
| 2 | `wav` cell → "8/10 … 2 … REFUSED — unsupported in wav (demux.wav.unsupported-format) ×2" | :70-73 | REAL | compat-matrix-20261110.md:86 — verbatim. |
| 3 | 7 new report tests with these names + `refusal_breakdown` (deterministic BTreeMap) | :21-27 | REAL | tests/compat/src/report.rs:1957 `mixed_row_surfaces_its_refusals`, :2003 `distinct_refusal_messages_are_each_counted`, :2090 `failed_wasm_step_renders_steps_failed`, :1117 `fn refusal_breakdown` — names and function landed. |
| 4 | Census: "166 rows; 84 decoded / 22 partial / 6 refused (2 probe + 4 demux) / 0 demux-fatal; 54 malformed all-typed" | :35-40 | REAL | Recomputed from raw-rows.jsonl this run: 166 rows; combined statuses decode to valid-class 84/22/6/0 and malformed 13/11/20/8/2 (=54, all typed) — EXACT partition. |
| 5 | P2-2 wasm cells derived, failure arm reachable (`proc::run_bounded` records nonzero exits) | :55-56 | REAL (code side) | report.rs:2090 `failed_wasm_step_renders_steps_failed` + the passing-steps twin exist (test pair pins both arms); the reachability reasoning cites report.rs:502-515/:617-621 (at-date lines; file present). |
| 6 | "IN-8 … not addressed at the audited tip … the duplicated citation is still on the `codec-audio-aac-lc` evidence line" | :48-50, table | REAL (at date; later resolved) | feature-matrix.toml:681 aac-lc evidence now carries the compat-matrix citation ONCE — the doubling is gone in the current tree (post-audit fix), consistent with the audit flagging it as the finish-line item. |
| 7 | P3-3 "half fixed … report.rs:1807 still literal `- **{}** (linux): …`" + P3-4 "VLC VLC" not addressed | table | REAL (at date) | Both were snapshot facts against branch `022d679`; current report.rs carries a "(linux)" mention at :139 in comment prose (line drift expected); at-date claims not contradicted. |
| 8 | "Matrix counts unchanged … raw `.jsonl` untouched (same git blob)" | :13-15, :31-33 | UNVERIFIED (git part) / REAL (data part) | The blob-identity claim needs git; the DATA claim holds — every derived artifact this run (my PERF-VS-VLC, compat cells) reconciles against the raw-rows.jsonl without conflict. |
| 9 | `cargo test` 11/11, clippy 0 warnings under deny lints, fmt clean (tests/compat standalone workspace) | :18-28 | UNVERIFIED | No cargo in this sandbox; the mirrored deny-lints table in tests/compat/Cargo.toml is the same pattern verified in my md5.rs audit (standalone workspace + lint mirror). |
| 10 | Worklog truthfulness (9 hashes, +19 append, AGENTS.md §4 shape) | :77-84, §1 | UNVERIFIED | git-dependent (hashes/order); root worklog.md exists with append-only entries; the reconstruction-disclosure line ("reconstructed from the 9 commit messages") is quoted as the honesty marker it is. |

## 3. Contradictions
None. The audit's own table publishes seven "not addressed" verdicts against its
audited lane — it grades the incompleteness it found rather than rounding it up.

## 4. Fabrication / inflation indicators
None. The headline "0 P0/0 P1/1 P2/2 P3" scores the AUDIT's own findings against the
fix lane; the fix's "reporting-only" scope (counts byte-identical, raw rows untouched)
is stated as a limitation, not sold as depth.
