# AUDIT: ui/e2e/visual/baselines/linux/dev-tokens-1920x1080-1x-dark.png
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A02) |
| Date | 2026-11-24 |
| File verdict | REAL (asset) — live diff UNVERIFIED in this sandbox |
| Reachable from shipped entry point? | As data: YES — nightly lane only (1920×1080 → pr=false) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | python3 audit/scratch/W16-RR3/batchmeta.py; manifest↔disk id check (id present; 216=216, 0 orphans); rg over gates/workflows |

## 1. Verdict (data audit)
Real screenshot baseline: 19,774 bytes, sha256 5329bfdaac921dfeea4a27214f0f59b1c3dd4d3c783c34b
89412e37fb9b2022c, 1405684 decompressed bytes, 148/244 unique scanlines, single IDAT set with
IEND (no ancillary chunks). Consumer chain: manifest.ts:142 caseId ("/dev/tokens" ×
1920×1080 × 1× × dark, pr=false) → visual.spec.ts:103 → playwright.visual.config.ts:27;
disk/manifest 1:1 (script check). Committed CI consumer: `visual-nightly`
(.github/workflows/ui.yml:205).

F-001 [S4] Largest filename-vs-IHDR mismatch in the set: claims 1920×1080, IHDR is
**1920×244** — the /dev/tokens page's `#root` box is only 244 CSS px tall, and the harness
clips to it (visual.spec.ts:96-102). The image is a genuine render of a short page; the NAME
is what lies (viewport ≠ content height). Confidence: HIGH (IHDR parsed by script).
F-002 [S4] No local gate runs the nightly lane (rg full_gate.sh/rc-smoke.sh: zero matches).
F-003 [S4] Consuming job mislabeled: ui.yml:206 "136 cases" vs actual 216.
Live diff UNVERIFIED here (no browser budget; 5th-re-pin CI proof deferred —
MATRIX-20261113.md:153).
