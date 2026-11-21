# AUDIT: ui/src/components/appshell/shellGlyphs.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 185 / 13 (Glyph + 12 exported glyph components) |
| File verdict | REAL |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (all 12 glyphs consumed by components/appshell/AppShell.tsx — rg 1 file each, verified; AppShell wraps every route). Window-control glyphs drive the REAL window-controls seam (client/window-controls.ts → plugin:window|… Tauri commands) inside the desktop shell. SHIPPED DESKTOP BINARY: NO (tauri unbuildable — baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg per-glyph consumers (12/12 → AppShell.tsx only); rg window-controls wiring in AppShell.tsx:81-82,425-457 + client/window-controls.ts:8,54,102-108; read all SVG paths |

## 1. One-paragraph verdict
Pure, correct, fully-consumed SVG glyph module. Every glyph follows the icon
system's shape contract (24×24 viewBox, currentColor ink, round caps, 1.5/2
stroke, aria-hidden decorative, lumen-icon class), all 12 are rendered by
AppShell, and the window-control glyphs sit on buttons wired to the real
Tauri window-controls seam. Nothing fake, nothing dead. The only finding is
structural: twelve hand-rolled glyphs live outside the pinned icon registry
by explicit [PROPOSAL], so the registry's tooling cannot keep their
conventions honest.

## 2. What actually works (verified)
— verified by rg: each of GlyphMinimize/Maximize/Restore/Close/Plus/
ChevronLeft/More/Search/Sort/List/Grid/Playlist has exactly one consumer
(AppShell.tsx); none dead.
— verified by read: Glyph shell implements the documented contract (fill vs
stroke switching for dot glyphs at 44-45, decorative aria-hidden at 49);
every path is well-formed viewBox-safe geometry (coordinates within 0-24).
— verified by rg + read: the window-control consumers dispatch real
plugin:window commands (AppShell.tsx:425-457, error paths logged) through
window-controls.ts's __TAURI_INTERNALS__ probe — in the desktop shell these
are live buttons, not decoration.

## 3. Findings
### F-001 [S4] Twelve glyphs live outside the pinned icon registry — conventions held by hand, not tooling
Where: ui/src/components/appshell/shellGlyphs.tsx:20-22
Code:
```
// [PROPOSAL] promote these into the registry (a follow-up icon-lane card)
// once the frame names are pinned; until then they live with their only
// consumer (appshell).
```
What is wrong: The icon registry (components/icon/icons.tsx) is the pinned
vocabulary the rest of the app draws from (Icon.tsx + icon.css state
protocol); this module re-implements the same SVG shell privately. The
shape contract (viewBox/stroke/caps/aria) is duplicated rather than shared,
so a registry convention change (e.g. stroke width policy, class name) does
not propagate here.
Why it matters: Drift risk only — today the conventions match (verified by
comparison with the Glyph shell vs Icon.tsx's svg attrs).
What correct looks like: The [PROPOSAL] promotion — register the 12 shapes
in icons.tsx and delete this module's private shell.
Confidence: HIGH — Verification: read (both files compared).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Glyph (shell) | 27-55 | REAL | — | fill/stroke switch correct; lumen-icon class rides icon.css tints |
| GlyphMinimize | 58-64 | REAL | — | consumed; 2px stroke per frame |
| GlyphMaximize | 67-73 | REAL | — | hollow square |
| GlyphRestore | 76-83 | REAL | — | two offset squares, path arithmetic sane |
| GlyphClose | 86-93 | REAL | — | × |
| GlyphPlus | 96-103 | REAL | — | rail pinned stop + create row |
| GlyphChevronLeft | 106-112 | REAL | — | |
| GlyphMore | 115-123 | REAL | — | filled dots (filled=true path) |
| GlyphSearch | 126-133 | REAL | — | |
| GlyphSort | 136-146 | REAL | — | descending bars + arrow |
| GlyphList | 149-160 | REAL | — | zero-length trick `M5 6H5.01` for dots with heavier stroke — valid SVG |
| GlyphGrid | 163-172 | REAL | — | 2×2 tiles |
| GlyphPlaylist | 176-185 | REAL | — | only glyph defaulting size 14 (its consumer's row scale) — documented by usage |

## 5. Test honesty
No dedicated test file; AppShell.test.tsx mounts the shell that renders
these glyphs (baseline ui-vitest passed). SVG path data is not unit-testable
meaningfully; visual manifest (e2e/visual) covers the shell's appearance but
did NOT run in baseline (UNVERIFIED visual fidelity — same note as my
onboarding.css audit).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no in-app action may wear a window-control glyph" (w28 fix 2) | header 8-12 | REAL | AppShell.tsx in-app actions import registry glyphs (pip/fullscreen/stop per header) — spot-verified by import list |
| "All glyphs are DECORATIVE (aria-hidden): their buttons carry i18n'd aria-labels" | header 17-18 | REAL | Glyph renders aria-hidden (49); AppShell buttons carry labels (AppShell.test assertions exist) |
| "[PROPOSAL] promote these into the registry" | header 20-22 | REAL (open debt) | F-001 |

## 7. Dead / duplicated / unreachable code
- The private Glyph shell duplicates Icon.tsx's svg shell (F-001) — the
only duplication, documented.

## 8. Dependencies & flags
None beyond react types. No tokens, no client imports, no feature flags.

## 9. Open questions for the next agent
- AppShell.tsx (other agent's file): confirm the maximize/restore toggle
  actually swaps GlyphMaximize↔GlyphRestore on is_maximized events (the
  listen error path at 457 suggests live tracking exists).
