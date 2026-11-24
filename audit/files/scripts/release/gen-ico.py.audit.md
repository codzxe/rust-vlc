# AUDIT: scripts/release/gen-ico.py
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / functions | 100 / 3 (die, parse_color, chunk, png_rgba_square, main) |
| File verdict | REAL — executed; deterministic (two runs byte-identical AND byte-identical to the committed shipped artifact) |
| Reachable from shipped entry point? | N/A — asset generator; its output is COMMITTED at apps/desktop-tauri/icons/icon.ico (tauri.conf.json:31 references it) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | gen-ico.py x2 (#4f8cff) -> identical sha256 a8a8c582580de1d0…; bad-color run -> die; full ICO+PNG byte-level parse of the committed artifact; uniformity check across all 256 scanlines |

## 1. One-paragraph verdict
A correct, minimal, stdlib-only Vista-style ICO generator. I executed it
twice — byte-identical outputs (sha256 a8a8c582580de1d0…), and that hash is
EXACTLY the sha256 of the committed apps/desktop-tauri/icons/icon.ico: the
shipped file is provably this script's output for the current brand accent
(#4f8cff, product.toml's PRODUCT_ACCENT_COLOR). Byte-level parse of the
committed artifact confirms every format claim in the docstring: ICONDIR
(reserved 0, type 1, count 1), ICONDIRENTRY with width/height bytes 0 (=
256), planes 1, bpp 32, correct size/offset, an embedded PNG with proper
magic, IHDR 256x256 8-bit RGBA, zlib IDAT, and 256 filter-0 scanlines of a
uniform #4f8cff FF fill. Bad color input dies with a clear error. The one
gap: nothing regenerates or drift-gates this artifact — if the brand accent
changes in product.toml, the committed icon.ico silently goes stale (no gate
compares them; the file is a one-shot commit, not a build-time generation).

## 2. What actually works (verified)
- `python3 scripts/release/gen-ico.py t.ico '#4f8cff'` -> `wrote t.ico (256x256 PNG-in-ICO, fill #4f8cff, 687 bytes)`; second run -> identical sha256 `a8a8c582580de1d0580a49d4a611d4db…` (determinism).
- `sha256sum apps/desktop-tauri/icons/icon.ico` -> SAME hash — the shipped artifact IS this script's output.
- Committed-artifact parse (python3 struct+zlib): ICONDIR `res=0 type=1 count=1`; ENTRY `w=0 h=0 pal=0 planes=1 bpp=32 size=665 off=22`; PNG magic ok; `len(png)==size` True; IHDR `(256,256,8,6,0,0,0)`; IDAT decompresses to 256 scanlines, `first px RGBA=4f8cffff`, `all 256 rows uniform: True`.
- `gen-ico.py bad.ico nothex` -> `gen-ico: error: color 'nothex' is not #rrggbb (brand.accent_color format)` (die path exercised).
- Exec bit present (rwxrwxr-x) — directly invocable, unlike gen-icon.py.

## 3. Findings
F-001 [S4]
Where: scripts/release/gen-ico.py (whole) + apps/desktop-tauri/icons/icon.ico
Code:
```
Usage:
  gen-ico.py <output.ico> <#rrggbb>
```
What is wrong: the generated artifact is COMMITTED once and never re-derived
— no workflow/gate invokes gen-ico.py (rg over scripts/ + .github/ +
worklog: zero invocations, only references). If product.toml's accent color
changes, the shipped placeholder icon drifts silently (no gate compares the
ICO's fill color to the config; a consumer of the icon sees a stale color).
Blast radius is a cosmetic placeholder, hence S4. What correct looks like: a
drift check (regenerate to temp + cmp) in the packaging gate, or generate at
build time. Confidence: HIGH — Verification: rg (no callers) + hash equality
proving one-shot provenance.

## 4. Per-function table (mandatory; file >100 LOC — file is 100/101 lines, table given anyway)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 31-33 | REAL (executed on bad color) | — | stderr + SystemExit(1) |
| parse_color | 36-43 | REAL (executed) | — | #rrggbb validation + hex parse |
| chunk | 46-52 | REAL (executed) | — | big-endian len + tag + data + crc32 — correct PNG chunk framing |
| png_rgba_square | 55-64 | REAL (executed) | — | IHDR >IIBBBBB correct; filter-0 scanlines; zlib level 9 |
| main | 67-96 | REAL (executed) | — | ICONDIR+ENTRY struct.pack verified by parse; usage guard |

## 5. Test honesty
No in-repo tests; my executions are the test (determinism, format parse,
uniform pixels, error path). Mutation reasoning: a wrong width/height byte or
offset would break my parse assertions; a color bug would show in the pixel
dump.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Deterministic: no timestamps, zlib level 9 on fixed scanlines — same input color, same bytes, every run" | :15-17 | TRUE | two runs -> identical sha256 |
| "ONE PNG-compressed 256x256 RGBA entry ... width/height bytes encode 256 as 0" | :12-15 | TRUE | committed-artifact parse: w=0/h=0, PNG 256x256 RGBA |
| "tauri-build Windows resource step requires icons/icon.ico (release dry-run 43406493996)" | :4-7 | UNVERIFIED (run ID) / TRUE-consistent | tauri.conf.json:31 lists icons/icon.ico; file exists |
| "not a borrowed third-party icon (clean-room rule)" | :8-10 | TRUE | bytes are a solid accent fill, self-generated (hash-identical) |

## 7. Dead / duplicated / unreachable code
None. No callers at all (the F-001 point), but that is provenance-by-commit,
not dead code.

## 8. Dependencies & flags
Stdlib only (struct, sys, zlib). CLI: <output.ico> <#rrggbb>. Output shape
verified byte-level.

## 9. Open questions for the next agent
- If the brand assets (assets/brand/icon.ico per config-validator) ever land,
  does tauri.conf.json's icons list switch to them, and does anything retire
  this placeholder? The drift concern (F-001) only matters while the
  placeholder ships.
