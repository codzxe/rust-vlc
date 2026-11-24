# AUDIT: ui/e2e/visual/baselines/linux/onboarding-1280x720-1x-dark-blue.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A02) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — nightly lane only (accent-sample case: ACCENT_SAMPLES carry pr=false) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216, 0 orphans); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 25,374 bytes, sha256 bd2ccb8a314aff025b05bde4d1743d8af0f3b7333df13ac
f093018fda9815914, 2704064 decompressed bytes, 249/704 unique scanlines. Consumer chain:
manifest.ts:142 caseId — this is one of the 16 ACCENT_SAMPLES ("blue" on /onboarding at
1280×720 1× dark, manifest.ts:159-172), diffed at visual.spec.ts:103 →
playwright.visual.config.ts:27; disk/manifest 1:1 (script check). Accent samples are pr:false
→ nightly-only; committed CI consumer is `visual-nightly` (.github/workflows/ui.yml:205).

F-001 [S4] Filename-vs-IHDR mismatch: the file claims 1280×720 but IHDR is **1280×704** — the
spec clips to `#root`'s bounding box (visual.spec.ts:96-102), and the onboarding route's root
is 16 CSS px short at this viewport. The name records the VIEWPORT, not the image. Same
defect class as the grid twin onboarding-1280x720-1x-dark.png (found by W16-A10) — systematic,
not corruption (image itself is a valid, genuine render).
F-002 [S4] No local gate runs the nightly lane (rg full_gate.sh/rc-smoke.sh: zero visual
matches).
F-003 [S4] Consuming job mislabeled: ui.yml:206 "136 cases" vs actual 216 (MATRIX-20261113.md:41).
Confidence: HIGH — Verification: read + script + rg. Live diff UNVERIFIED here (no browser
budget; 5th-re-pin CI proof deferred — MATRIX-20261113.md:153).
