# AUDIT: ui/src/components/appshell/shellGlyphs.tsx
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 185 / 13 (Glyph shell + 12 exported glyph components) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (AppShell.tsx:92-96 imports; every one of the 12 exports has exactly 1 non-test consumer — AppShell) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg per-glyph consumer count (12/12 live); rg AppShell window-control handlers; sed quote verification |

## 1. One-paragraph verdict
Clean decorative SVG glyph set — 12 components, every one consumed by AppShell,
all aria-hidden with the a11y contract pushed to the host buttons (which carry
i18n labels). The window-control glyphs (minimize/maximize/restore/close) sit
on buttons wired to REAL Tauri window operations (AppShell.tsx:613-643:
onOsMinimize/onOsToggleMaximize/onOsClose; window-controls.ts documents
plugin:window|minimize etc. with permission grants) — these are among the few
UI elements backed by a real host backend. Zero findings.

## 2. What actually works (verified)
— rg: all 12 exports consumed exactly once (AppShell.tsx) — no dead glyphs.
— AppShell.tsx:632 swaps GlyphRestore/GlyphMaximize on osMaximized state —
the restore glyph is reachable, not dead art.
— Glyph shell contract (40-53): stroke currentColor, round caps/joins,
aria-hidden — matches the documented icon system (doc 08 §1.4).

## 3. Findings
None. Checked: dead exports (none — 12/12 consumed); a11y (all decorative,
host labels verified at AppShell 613-643); path geometry sanity (all within
24×24 viewBox, no negative coords, restore's two-square path plausible);
[PROPOSAL] promotion note (line 20-22) is tracked debt, not a defect.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Glyph (shell) | 27-55 | REAL | - | fill/stroke switch via `filled` |
| GlyphMinimize/Maximize/Restore/Close | 58-93 | REAL | - | OS chrome, 2px stroke, Tauri-wired |
| GlyphPlus | 96-103 | REAL | - | rail/create row |
| GlyphChevronLeft/More/Search | 106-133 | REAL | - | sidebar header |
| GlyphSort/List/Grid | 136-172 | REAL | - | sidebar meta row |
| GlyphPlaylist | 176-185 | REAL | - | playlist rows |

## 5. Test honesty
No tests in this file; AppShell tests owned elsewhere.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the REAL OS chrome glyphs ... Player.svg topbar" | shellGlyphs.tsx:2-3 | REAL | AppShell mounts them on Tauri-backed buttons |
| "no in-app action may wear a window-control glyph" | shellGlyphs.tsx:10-12 | REAL | rg: window glyphs only in the winctl cluster |

## 7. Dead / duplicated / unreachable code
None. [PROPOSAL] registry promotion (20-22) is acknowledged future work.

## 8. Dependencies & flags
React only. Pure presentational; no tokens, no flags.

## 9. Open questions for the next agent
window-controls.ts owner: confirm the Tauri permission set actually grants
core:window:allow-minimize/toggle-maximize/close in the shipped bundle.
