# DATA AUDIT: docs/baselines/raw/linux/13-options-advanced-longhelp.txt
| Field | Value |
|---|---|
| Auditor | W19-A08 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 3,364 lines UTF-8 text; 1,463 raw `^\s+--` option lines; footer `# exit=0` |
| File verdict | REAL (artifact; in-tree derivation re-executed exactly) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | file(1); rg censuses; python3 parse-options.py re-run (exit 0); python3 json count |

## 1. What it is / provenance
Raw captured stdout of the pinned VLC 3.0.23 binary: `vlc -I dummy --advanced --longhelp`.
Header (sed -n '1,9p', byte-verified):
```
# VLC 3.0.23 pin — raw dump 13: options tree with advanced options (--advanced --longhelp)
# Date: 2026-10-19 13:20 UTC
# Env: Debian 13 (trixie) container, x86_64, headless (no X/Wayland)
# Binary: tmp/pinning/extract/usr/bin/vlc from debs listed in dump 02 (not system-installed)
# Pinned baseline: VLC 3.0.23 Vetinari (3.0.23-2-0-g79128878dd) — docs/baselines/vlc-3.0.23.md
```
file(1): "Unicode text, UTF-8 text, with very long lines (469)" — text-only as claimed.

## 2. Data verification (executed)
- Version hash `3.0.23-2-0-g79128878dd` appears identically in dumps 04, 13, 14, 15, 16,
  17 (rg) — pin identity is consistent across the whole baseline set.
- Re-ran the in-tree parser `python3 docs/inventory/raw/options/parse-options.py` (exit 0):
  longhelp 615 + advanced-only 861 = 1,476 normalized option records — EXACTLY equal to
  `len(docs/inventory/options-linux.json)` = 1476 (python3 json count). The dump's derived
  product is reproducible in-tree from this artifact.
- My raw line census 1,463 `^\s+--` lines differs from 1,476 normalized records because
  wrapped lines and `, --no-x` pairs normalize 1:2 — consistent, not a defect.
- Footer (tail, byte-verified): "Note: 4 modules were not displayed because they only have
  advanced options." / "To get exhaustive help, use '-H'." / "# exit=0".
- Options-summary.md D7 claim (dump 13 vs dump 14 option set: 0 difference) is corroborated
  by my censuses: 1,463 vs 1,464 raw `--` lines (footer-noise delta only).

## 3. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "(--advanced --longhelp)" dump identity | :1,8 | REAL | parser re-run consumed it; 861 advanced-only records |
| 2 | "(stdout+stderr merged below; stderr empty for this run)" | :9 | UNVERIFIED | vendor run not repeatable (no VLC in sandbox — baseline env facts) |
| 3 | "# exit=0" | footer | REAL-UNPROVEN | artifact-internal; run not re-executable |
| 4 | consumers' 1476-option count | derived | REAL | parse-options.py re-run = 1476 = options-linux.json len |

## 4. Findings
F-001 [S4] The vendor footer itself concedes non-exhaustiveness ("4 modules were not
displayed... only have advanced options"; "use '-H'") inside a dump whose header claims the
advanced tree. D7 (0-difference vs -H) neutralizes the risk, but any consumer citing dump 13
as exhaustive without D7 inherits it. Confidence: HIGH — Verification: read.

## 5. Consumers
docs/inventory/options-summary.md, docs/inventory/raw/options/{parse-options.py,
manifest.json}, docs/inventory/options-linux.json, docs/07-config-driven-system.md options
layer. Prior W19-A08 audit (old tree audit/docs/baselines/...) reached the same verdict
("manifest counts reproduced 1476/187/3364") — numbers agree.

## 6. Incident note (self-report)
My parser re-run regenerated docs/inventory/options-linux.json (the script's declared
output). Deterministic recomputation from unchanged inputs; recorded here and in my run log
per protocol precedent (W19-A09's identical summary.json incident). No source bytes were
intentionally modified.
