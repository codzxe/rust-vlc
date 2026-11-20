# Wave 19 SUMMARY — Claim ledger B: key docs (PARITY-REPORT, KNOWN-LIMITS, RELEASE, README, MASTER-PROMPT, ROADMAP, PERF-VS-VLC, feasibility-report)

Auditor: W19-A01 (subagent). 180 claims extracted and verdicted; full ledgers under `audit/docs/*.audit.md`; cross-doc contradiction register: `audit/docs/_CROSS-DOC-CONTRADICTIONS.md` (16 entries, both sides cited).

## Verdict distribution (180 claims)
REAL 91 · REAL-UNPROVEN 35 · SHALLOW 8 · PAPER-ONLY 6 · **FALSE 23** · UNVERIFIED 17.

## The premise's named contradictions — audited verdicts
| Premise says | Verdict | Proof |
|---|---|---|
| "PARITY-REPORT says CI billing-blocked; RELEASE.md says unblocked 2026-11-12" | CONFIRMED contradiction, still live at HEAD | release.yml:26 present-tense "NEVER EXECUTED (CI billing blocked)" vs RELEASE.md:145-149 "unblocked 2026-11-12, executed" — one side is false either way; not decidable from inside the tree (UNVERIFIABLE without Actions access) |
| "KNOWN-LIMITS says the parity refresh is owed" | CONFIRMED | KNOWN-LIMITS:17 concedes; PARITY-REPORT quotes 64/252 vs live 66/61/66 |
| "The GUI mounts no <canvas>" | STALE claim, FALSE at HEAD | KNOWN-LIMITS.md:52 says "no `<canvas>` exists in `ui/src`"; MediaSurface.tsx:174 mounts the canvas + putImageData (:225-227) since w31.4 |
| "No audio device output exists (null backend only)" | CONFIRMED TRUE at HEAD | audio-output lib.rs:152-157 [Null, Unsupported]; cpal.rs:43 admitted=false; worklog.md:2294-2296 "audio output remains the null device engine-wide" |
| "An MP4 video cannot be opened properly" | STALE for the CLI at HEAD | executed: open+advance plays H.264+AAC MP4 end to end (100.0%, Ended); GUI runtime display UNVERIFIED; audio null (see W13) |

## Worst FALSE/stale claims (full list in the doc ledgers)
1. PARITY-REPORT §3 "result: PASS" — live feature-audit FAILURE (2 R4 violations).
2. PARITY-REPORT 64/252 symmetric — live 66/61/66.
3. KNOWN-LIMITS:52 no-canvas — canvas exists and is wired.
4. KNOWN-LIMITS Opus unplayable — implemented since 2026-11-09.
5. README URL sources refused — http(s) opens since w31.3.
6. PERF-VS-VLC "h264: parsers only, no decoder" — decoder landed w27, executed by this audit.
7. ROADMAP D-017 "61 commands / 23 events pins the exact counts" — live catalog 65; two control-server tests red over the drift.
8. ROADMAP D-016 "commit 2d06607" — no such revision in git.

## The blunt verdict (W19-A01's, endorsed)
Two strata: the planning stratum (MASTER-PROMPT, feasibility-report) reconciles to the digit; the status stratum is structurally honest about incapacity (no sound anywhere, 0/252 verified, unsigned binaries) while systematically misstating current capability — staleness that loses to the code every wave, and the repo's own w30.10 remediation entry already certified this pattern once before regressing within a day.

## Anti-laziness gate
Orchestrator re-checked 4 citations (PARITY-REPORT.md:109, KNOWN-LIMITS.md:52, worklog.md:2294-2296, release.yml:26-28) — all verbatim-faithful. PASS. audit/verify/fabrications.md still empty.
