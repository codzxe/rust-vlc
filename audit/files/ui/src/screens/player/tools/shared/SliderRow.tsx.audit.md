# AUDIT: ui/src/screens/player/tools/shared/SliderRow.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 145 / 1 memoized component (+1 const + 2 interfaces) |
| File verdict | REAL — clean shared presentational row for the (staged) tool cards |
| Reachable from shipped entry point? | YES (composed by tools/dynamics ToolSliderRow and tools/video rows — both mounted in the ToolsDialog audio/video tabs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | read (full file); rg `SliderRow` consumers (exit 0 — dynamics + video lanes) |

## 1. One-paragraph verdict
A deduplication done right: one generic, memoized, fully-controlled slider row (label/value/ticks) replacing two drifted 1:1 copies (documented at L1-5). Tick paint math is frame-measured with the corrected fill domain (2px inset), and the covered-tick band compares like-with-like (L104-131). The component itself has no defects; its only product-level reality is that every card using it is a staged (engine-unwired) tool, disclosed by the "no engine yet" comment at L68 and the staged markers in the host cards.

## 2. What actually works (verified)
— verified by read: percent math guards range<=0 (L83); `onChangeLive`/`onCommit` both flow to the same controlled sink (L99-100); memoization is type-safe via the `as typeof` cast (L144); tick positions use i/(n+1) with default-on-tick claim consistent with the frames' cited coordinates.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/tools/shared/SliderRow.tsx:68 (context, not a code defect)
Code:
```
  /** Live + commit sink (both drive the same local state — no engine yet). */
  readonly onChange: (value: number) => void;
```
What is wrong: nothing in this file — the row is the presentation layer for controls whose commits reach only card-local React state (see dynamics.css and EqualizerTool audits: no audio-effect/video-adjust commands exist in the catalog, and the SERVED host list would refuse them anyway). Recorded so the deadness is attached to this file too. Confidence: HIGH — Verification: read + prior rg evidence (catalog union + SERVED list).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| COVERED_TICK_BAND | 39 | REAL | — | documented approximation (8px/328px ≈1.2% padded to 1.5%) |
| SliderRowDef / SliderRowProps | 42-72 | REAL | — | generic over label key |
| SliderRowInner | 75-137 | REAL | — | percent guard; frame-domain ticks |
| SliderRow (memo) | 144 | REAL | — | per-row re-render isolation |

## 5. Test honesty
No tests in-file; the dynamics/video lane tests exercise it through the host cards (baseline vitest). Would fail if tick math or value formatting broke — presentation-only assertions, honest.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ToolSliderRow (dynamics) and VideoSliderRow (video) were 1:1 copies that had ALREADY drifted" (L2-4) | header | REAL (UNVERIFIED history) | no git allowed; current tree has one shared module and both lanes composing it (rg) |
| "the class names are the cross-lane contract" (L19-20) | header | REAL | video.css composes onto .dynamics-* (documented, cross-checked in dynamics.css audit) |

## 7. Dead / duplicated / unreachable code
None in-file (this file REMOVED duplication). Host cards are staged surfaces (F-001 context).

## 8. Dependencies & flags
components/slider Slider; dynamics.css class scope (shared by contract). No transport, no feature flags.

## 9. Open questions for the next agent
- If the video-adjust commands (video.set-*) ever land, VideoSliderRow hosts must switch from local card state to engine dispatches — this row needs no change (fully controlled), which is the point of the design.
