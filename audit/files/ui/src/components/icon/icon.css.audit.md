# AUDIT: ui/src/components/icon/icon.css
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 160 / ~20 rules |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — imported by Icon.tsx, used by every control surface (Button/IconButton/menu rows/ControlBar) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | full read; rg token count (16 var() uses); corruption sweep (none) |

## 1. One-paragraph verdict
Clean, well-documented icon stylesheet: token-driven colors, frame-measured geometry, deliberate (not accidental) stylelint disables with justification comments, reduced-motion respected, dead host-state variants pruned with the pruning documented (w26.k04 j04-6). No corruption, no raw hex outside comments.

## 2. What actually works (verified)
— verified by read: state cascade (hover/active/open/focus-visible/disabled), two-tone detail repainting with a documented light-theme token-lane workaround, hit-area presets 24/30×32/44, `pointer-events: none` delegating hits to hosts.

## 3. Findings
F-001 [S4]
Where: ui/src/components/icon/icon.css:64-70
What is wrong: the `:root[data-theme="light"]` override exists only because tokens.css lacks a light `--color-accent-strong` — theme knowledge in a component sheet (self-admitted handoff). Confidence: HIGH — Verification: read.

## 4. Per-function table
Rule groups: .lumen-icon base (L8-20), host state variants (L23-46), two-tone (L48-77), emphasis strokes (L79-90), disabled leaf (L92-97), reduced-motion ×2 (L99-102, L156-159), wrapper + presets + hover/active (L104-154). All REAL; none dead (the pruned --selected/--disabled host variants are documented as removed).

## 5. Test honesty
Icon tests exist in lane (not in my list); states pinned per comments. UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the selected and disabled HOST variants were pruned — zero producers anywhere (only --open is ever applied, ControlBar.tsx:814/1040)" | comment L27-30 | REAL-UNPROVEN | rg of ControlBar cited lines not re-run; no --selected producer found in my sweeps |

## 7. Dead / duplicated / unreachable code
`.lumen-icon--max` — catalog-reserved, no production producer (self-admitted, kept for API freeze).

## 8. Dependencies & flags
Token layer. No env vars.

## 9. Open questions for the next agent
- None material.
