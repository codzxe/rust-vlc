# AUDIT: ui/src/components/slider/Seekbar.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 141 / 1 (Seekbar component) |
| File verdict | REAL-UNPROVEN (truthful pending/buffer semantics; exercised by committed tests; not executed by me) |
| Reachable from shipped entry point? | YES — rendered by ControlBar (player transport), MiniPlayer.tsx:256, PlaylistBrowse NowPlayingCard:194; all on /player routes |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; grep consumers; Seekbar.test.tsx inventory (12 its, earlier command) |

## 1. One-paragraph verdict
Correct, honest seek control. The knob previews scrub (local), locks at the caller-passed `pending` target after commit, and the FILL always tracks engine-reported position — no fabricated jumps (the three-phase contract in the header matches the code exactly). Buffer ranges are engine data rendered as clamped sub-regions with a duration>0 gate (the documented Infinity->100% fix, w26.k03); duration<=0 renders an honestly inert, aria-disabled, untabbable bar; the interaction engine is the audited interaction.ts. No findings.

## 2. What actually works (verified)
— verified by read: knob = dragValue ?? pending ?? value precedence; fillPercent from `value` only; buffer clamp chain `clamp(..., 0, 100)` and `width clamp(..., 0, 100 - start)`; inert gate; forwardRef + displayName; aria-valuemax 0-guard. Seekbar.test.tsx (12 its) covers commit/pending per earlier inventory; baseline UI pass.

## 3. Findings
None. Checked: pending/scrub/engine precedence, buffer math division-by-zero (gated), step adaptivity (duration/1000, floor 0.001), aria (valuenow/text/min/max/disabled, tabIndex -1 inert), keyboard path (interaction.ts), style var injection, ref forwarding.

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Seekbar | 60-139 | REAL-UNPROVEN | — | see §2 |

## 5. Test honesty
Seekbar.test.tsx 12 its — commit/pending semantics asserted (would fail if the fill jumped to the request). Not executed by me.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the component never invents buffer progress" | L6-8 | REAL | buffered is caller data, mapped 1:1 with clamps |
| "No fabricated jumps anywhere" | L49-53 | REAL | fill = engine value only (L94) |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
interaction.ts (audited clean), formatTime, slider.css. No feature flags.

## 9. Open questions for the next agent
None.
