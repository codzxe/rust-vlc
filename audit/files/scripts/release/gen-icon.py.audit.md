# AUDIT: scripts/release/gen-icon.py
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 80 / 4 (die, parse_color, chunk, main) |
| File verdict | REAL (executed; determinism, validity, and log cross-verification all proven) |
| Reachable from shipped entry point? | YES — scripts/packaging/build-deb.sh/build-appimage.sh call gen-icon (committed lane log:2-3, :5 show its output inside the deb/AppImage lanes) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | 4 executions of the script (exit 0/0/1/1), sha256sum, file, CRC validator, `python3 --version` — outputs below |

## 1. One-paragraph verdict
This script does exactly what it claims and I proved every property it advertises: executed twice with the same color it produced byte-identical files (same SHA-256); the output is a structurally valid PNG (512x512 8-bit RGBA, all three chunk CRCs verified recomputed); invalid color specs are rejected with exit 1; and the byte count (1819 for `#4f8cff`) reproduces the committed packaging-evidence log line exactly — the committed lane log is honest. The one defect is a docstring lie about minimum Python: the `tuple[int, int, int]` annotation makes it Python 3.9+ only, not 3.6.

## 2. What actually works (verified)
- Determinism (executed): `python3 scripts/release/gen-icon.py <a.png> '#4f8cff'` twice → both `7574265e9f4e6766dd5fcd9f11f83ad1228eb0c6afbbc17534f772928a890595` — identical.
- Validity (executed): `file a.png` → "PNG image data, 512 x 512, 8-bit/color RGBA, non-interlaced"; chunk walker: IHDR 13 / IDAT 1762 / IEND 0, all CRCs OK.
- Log cross-verification: output "gen-icon: wrote … (512x512 RGBA, fill #4f8cff, 1819 bytes)" — byte-count identical to docs/packaging/evidence/linux-lane-20261110.log:2-3 "gen-icon: wrote /tmp/tmp.eSBxHiy39x/root/usr/share/icons/hicolor/512x512/apps/lumen.png (512x512 RGBA, fill #4f8cff, 1819 bytes)".
- Input validation (executed): `'#4G8cff'` → "gen-icon: error: color '#4G8cff' has non-hex digits" exit 1; `'4f8cff'` (no #) → "is not #rrggbb" exit 1.

## 3. Findings
F-001 [S4]
Where: scripts/release/gen-icon.py:19
Code:
```
runs on every CI image with Python >= 3.6.
```
What is wrong: `def parse_color(spec: str) -> tuple[int, int, int]:` (:35) uses PEP 585 builtin generic syntax evaluated at definition time; there is no `from __future__ import annotations`. On Python 3.6–3.8 this raises `TypeError: 'type' object is not subscriptable` at import. Why it matters: the docstring's portability claim is false; the script only runs on ≥3.9 (sandbox: 3.12.16). What correct looks like: either claim "Python >= 3.9" or add the future import. Confidence: HIGH — Verification: read (annotation semantics; not executed on a 3.6 interpreter — no such interpreter present).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 30-32 | REAL | — | stderr + SystemExit(1); observed twice |
| parse_color | 35-42 | REAL | — | executed with valid + 2 hostile inputs; all correct |
| chunk | 45-51 | REAL | — | big-endian length + CRC32; validated by recomputation |
| main | 54-75 | REAL | — | full run executed; IHDR fields (8-bit RGBA, no interlace) verified by `file` |

## 5. Test honesty
No test file exists for gen-icon.py. Mutation reasoning: if CRC or IHDR packing were broken, my executed chunk validator would have failed — the script's output is externally checkable and passed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Deterministic: … same input color, same bytes, every run" | gen-icon.py:14-16 | **REAL** | two executions, identical SHA-256 (above) |
| "Stdlib only (zlib + struct)" | :18 | REAL | imports: struct, sys, zlib (:25-27) |
| "runs on every CI image with Python >= 3.6" | :19 | **FALSE** | F-001 — needs ≥3.9 |
| "a generated placeholder … (clean-room rule: no VLC assets)" | :9-10 | REAL | output is a flat-color PNG — no borrowed art possible; verified by executing |

## 7. Dead / duplicated / unreachable code
None. `# type: ignore[return-value]` (:40) is noise (tuple of 3 ints from a generator of 3) — cosmetic.

## 8. Dependencies & flags
Python 3 (≥3.9 effective), stdlib only: struct, sys, zlib. No Pillow, no Rust deps — as claimed. Args: output path + `#rrggbb`.

## 9. Open questions for next agent
- gen-ico.py / gen-wxs.sh siblings (same dir) — were they validated the same way? No committed log line cross-verifies them the way the lane log does for this one.
- When assets/brand/ lands, product.toml `[brand]` switches to real assets — the stager's preference logic (stage-linux-payload.sh) is the seam to re-audit then.
