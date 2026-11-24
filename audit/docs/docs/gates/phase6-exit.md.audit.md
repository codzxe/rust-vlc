# CLAIM AUDIT: docs/gates/phase6-exit.md
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Claims extracted | 10 |
| REAL: 5 · REAL-UNPROVEN: 1 · SHALLOW: 0 · STUB: 0 · PAPER-ONLY: 0 · FALSE (stale vs. current repo): 3 · UNVERIFIED: 1 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
This is the honest one. Unlike phase1 (PASS on uncommitted evidence) and phase3 (a failing committed log quoted as pass), phase6-exit.md asserts NOTHING as passed: every gate row is PENDING, the verdict is PENDING, and it explicitly says runs "are to be executed at gate time, not asserted here" — which matches the repo reality (no phase-6 log exists in docs/gates/logs). Its defects are all staleness: wrong line citations for the RTSP refusal (the refusal is real but at different lines), a 0/11 protocol-access count that is now 1/11, and a "TLS pending Lead decision" that ADR-011 has since resolved.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Status: **DRAFT — PENDING LEAD DECISION**" | :3 | **REAL** | self-label; corroborated: `ls docs/gates/logs/` → no phase6 file (phase3, phase4, phase7, merge logs only) |
| 2 | "Gate-line runs (test/clippy/fmt/fuzz-registry) are **PENDING** — to be executed at gate time, not asserted here" | :6-7 | **REAL** | the gate table (:13-16) is all "PENDING / to run at gate" — nothing faked |
| 3 | "parity snapshot | 60/252 implemented-or-verified per desktop OS; web 0/252; strict verified-only 0/252" | :17 | **REAL-UNPROVEN (drifted)** | uncommitted session run; orchestrator baseline (later): windows 66/252, linux 61/252, web 0/252, verified-only 0/252 — the strict 0/252 headline still holds, the 60 grew |
| 4 | Issue ledger rows citing commits (`c6a401e`, `b045721`, `372d9e1`, `27217e1`, `2834c5b`, …) | :23-27 | **UNVERIFIED** | git forbidden; no in-repo artifact cross-checks these hashes |
| 5 | "#14 … **HONEST: not in main.** … lives on branch `w10/14-netfuzz` … NOT counted as landed until merged." | :28 | **REAL-UNPROVEN** | branch existence needs git; the honesty structure (unmerged work excluded from "landed") is the correct posture |
| 6 | "#15 RTSP/RTP — DEFERRED with typed refusal … pinned by `crates/streaming/src/schemes.rs:557-577`, `:677-682`" | :34-36 | **FALSE line pointers (real refusal, wrong location)** | `sed -n '557,577p'` → `file:` URI normalization tests; `sed -n '677,682p'` → `https://` malformed refusal — neither is RTSP. The actual refusal: schemes.rs:695-696 (`("rtsp://cam.example.org/stream.sdp", "rtsp")`, `("rtp://224.1.1.1:5004", "rtp")` refusal vectors) and :812-817 (userinfo-bearing rtsp URL refuses, secret redacted) |
| 7 | "6.H1 security findings — 1 Medium (same-host https→http scheme downgrade followed) + 3 Low … `docs/investigations/network-security-review.md`" | :38-41 | **REAL** | network-security-review.md:10 "**1 Medium, 3 …**", :22 "### F1 — Same-host scheme downgrade followed (Medium)"; additionally :39 says the Medium is now "**RESOLVED (wave 19, `w19/sec-medium`)**" — the doc's "recommend scheduling" is overtaken in the safe direction |
| 8 | "Protocol-access category: 0/11 implemented (all blocked/deferred rows …)" | :42-43 | **FALSE (stale)** | tomllib recount today: protocol-access = 11 rows; linux/windows each: 1 implemented (`protocol-http-https`, w31.3 evidence: apps/cli/src/demux_host.rs open_http_source, ureq transport), 8 blocked, 2 not_started |
| 9 | "#8 TLS — pending Lead decision (ADR-011 input …). No verdict recorded here." | :32-33 | **FALSE (stale)** | docs/adr/ADR-011-tls-https.md exists and allowlist.md:68 records "discharged by ADR-011 accepted 2026-11-09" — the decision has since been made and the ureq row landed |
| 10 | "… `docs/investigations/rtsp-rtp-feasibility.md`" | :35-36 | **REAL** | `ls docs/investigations/` → rtsp-rtp-feasibility.md present |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "Protocol-access category: 0/11 implemented" (:42) | feature-matrix.toml today: protocol-http-https implemented on windows/linux/macos | The matrix wins (tomllib recount above); the doc is a pre-http-admission snapshot. |
| "#8 TLS — pending Lead decision" (:32) | ADR-011 accepted 2026-11-09 (allowlist.md:68; docs/adr/ADR-011-tls-https.md exists) | ADR-011 wins; the doc's "pending" predates the acceptance. |

## 4. Fabrication / inflation indicators
- None. The doc refuses to assert unrun gates (:6-7), excludes unmerged work from "landed" (:28), and makes its strict-verification 0/252 debt the headline (:51-53) — the opposite of inflation.
- The only quality failure is evidence hygiene: the RTSP line citations point at the wrong tests (claim 6) — likely line drift after edits, but a verifier following the doc's pins lands on file:/https tests and could conclude the refusal doesn't exist.

## 5. Matrix rows
N/A — not a matrix chunk (parity snapshot cross-checked against the orchestrator baseline as in claim 3).

## 6. What actually works (verified)
- `sed -n '557,577p'` / `677,682p` crates/streaming/src/schemes.rs (exit 0) — proved the cited pins wrong; `rg -n -i "rtsp|rtp"` found the real refusal vectors (:695-696, :812-817).
- python3 tomllib recount of protocol-access rows (exit 0) — claim 8.
- `ls docs/investigations/`, `rg` in network-security-review.md (exit 0) — claims 7, 10.

## 7. Open questions for next agent
- Did the phase-6 gate ever actually close? The doc is still DRAFT/PENDING and no phase-6 log exists — if any later doc claims "phase 6 exited green", that claim has no artifact behind it.
- The parity numbers 60 vs baseline 66/61 quantify seven months of drift since this draft — any doc citing phase6's 60/252 today is wrong.
