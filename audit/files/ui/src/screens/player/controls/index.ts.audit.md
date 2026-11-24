# AUDIT: ui/src/screens/player/controls/index.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 17 / 0 (barrel: ControlBar + state-mapping exports) |
| File verdict | REAL (live barrel over real transport mapping) |
| Reachable from shipped entry point? | YES — ControlBar renders in PlayerScreen (the /player transport band); state-mapping consumed by MiniPlayer/NowPlayingCard (my audits) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; consumer cross-refs from my MiniPlayer/PlaylistBrowse/Seekbar audits (mapTransportControls/seekDisableReason call sites) |

## 1. One-paragraph verdict
Clean barrel: the ControlBar component plus the state-mapping helpers (mapTransportControls / seekDisableReason / controlCommandOf + types) that my sibling audits verified as real engine-state mappings (MiniPlayer L122-123, NowPlayingCard L166-167). The header's claim "every control invoking a real command per doc 08 §5" is consistent with the player context actions those controls call (playback.* via context.tsx dispatch).

## 2. What actually works (verified)
— verified by cross-refs: state-mapping functions consumed at MiniPlayer.tsx:62/122, PlaylistBrowse.tsx:166; ControlBar in PlayerScreen.

## 3. Findings
None.

## 4. Per-function table
n.a. — barrel.

## 5. Test honesty
ControlBar.test.tsx + state-mapping tests exist (controls lane).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every control invoking a real command per doc 08 §5" | L2-3 | REAL-UNPROVEN | consistent with context.tsx actions; ControlBar depth belongs to its lane |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
None.

## 9. Open questions for the next agent
None.
