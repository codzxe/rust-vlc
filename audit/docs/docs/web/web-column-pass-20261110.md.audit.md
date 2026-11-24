# CLAIM AUDIT: docs/web/web-column-pass-20261110.md (34 lines)
| Field | Value |
|---|---|
| Auditor | W19-A05 |
| Claims extracted | 4 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 3 REAL / 1 REAL-UNPROVEN (dated histogram) / 0 / 0 / 0 / 0 / 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The honesty pass reproduces: my independent histogram of the web column today is
not_started 207 / blocked 26 / unsupported 15 / partial 4 / implemented 0 — one row
moved (blocked -> not_started/implemented family) since 2026-11-10 (doc said 206/27/15/4).
The headline claim — ZERO web-implemented rows, "the conservative truth" — is exactly
true and consistent with the parity report (web 0/252 on every category). Reason
coverage claim (27/27 + 15/15) reproduces as 41/41 blocked+unsupported rows carrying
blocked_reason/notes/evidence today.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "implemented (web) | 0 | none claimed — the conservative truth" | result table | REAL — REPRODUCED | My parse: web implemented = 0; parity-report web column 0.0% everywhere (baseline :30-50). |
| 2 | "blocked 27: 27/27 carry blocked_reason ... unsupported 15: 15/15 carry row reasons" | result table | REAL (41/41 today) | My parse over all blocked+unsupported rows: every one carries a reason field. |
| 3 | "partial 4: 4/4 carry evidence + notes (playback-frame-step ...)" | result table | REAL | Same parse: the 4 partial web rows carry evidence strings. |
| 4 | "No web status was changed by this pass ... records the verification" | result § | REAL-UNPROVEN (intent) | Consistent with the pass being docs-only. |

## 3. Contradictions
None. (206 vs 207 not_started = one row's status changed post-pass by a later card, not a
pass error.)
## 4. Fabrication / inflation indicators
None — the pass is explicitly a no-change verification record.
