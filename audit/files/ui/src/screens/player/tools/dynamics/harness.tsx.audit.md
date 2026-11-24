# AUDIT: ui/src/screens/player/tools/dynamics/harness.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 53 / 2 components |
| File verdict | REAL (dev-only tooling, isolated) |
| Reachable from shipped entry point? | NO by design: rg — only harness.test.tsx consumes it; no app route imports |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumer + dynamics staging checks (exit 0) |

## 1. One-paragraph verdict
A clean dev-only vision harness composing the three dynamics tools (Compressor/Spatializer/StereoWidener) in frame cells (collapsed over expanded) for the screenshot comparison loop. Isolation verified: zero production imports. Context for the master question (verified in DynamicsCard.tsx:18-19, 91-92, 138): the tools it renders are LOCAL-STATE-ONLY, visibly marked staged with `role="note"` lines and `data-pending-command` markers — no catalog command exists for audio dynamics (the EQ-family gap Wave 7 established).

## 2. What actually works (verified)
— rg: no app-route import path reaches this file.
— The harness's plain-string captions are dev-only frame names (lint exemption documented in-line).

## 3. Findings
None.

## 4. Per-function table
N/A (<100 LOC): HarnessCell + DynamicsToolsHarness reviewed in full.

## 5. Test honesty
harness.test.tsx renders it to static HTML for screenshots (the lane's comparison loop); pins composition, not product behavior — appropriate for a dev tool.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "DEV-ONLY vision harness (never imported by app routes)" | L1 | TRUE | rg isolation |

## 7. Dead / duplicated / unreachable code
None (unreachable by design and documented).

## 8. Dependencies & flags
The three dynamics tools + harness.css. No gates.

## 9. Open questions for the next agent
- None (the tools' staging honesty is noted here; their files belong to another agent).
