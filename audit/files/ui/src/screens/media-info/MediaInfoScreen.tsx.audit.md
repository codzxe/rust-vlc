# AUDIT: ui/src/screens/media-info/MediaInfoScreen.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 29 / 1 (MediaInfoScreen) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — route /media-info (App.tsx:108) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | read; MediaInfoProvider trace (media.get-info dispatch verified at provider L53-70) |

## 1. One-paragraph verdict
A 29-line thin screen: page h1 + the shared MediaInfoPanel under MediaInfoProvider — and the provider is REAL bridge code: it sends `media.get-info` (session-scoped, no args) over the client transport, maps the completed `media-info` result into state, and distinguishes feature-not-exposed / other-errors / transport-unreachable into three honest verdicts (empty state, not-exposed, unreachable). One of the genuinely engine-wired screens.

## 2. What actually works (verified)
— verified by read of MediaInfoProvider.tsx:53-70: `transport.send({requestId, cmd: "media.get-info"})` + result discrimination (`completed.kind === "media-info"` / `error.code === "feature-not-exposed"` / catch → unreachable). NOT executed against a live engine — REAL-UNPROVEN at runtime, REAL by code path.

## 3. Findings
Zero findings. Checked: heading dedup (showTitle=false with the route h1), provider mounting, no side effects on documentElement (header claim verified — no documentElement touch in 29 lines).

## 4-9. (compact)
Per-function: MediaInfoScreen REAL. Tests: media-info lane (UNVERIFIED). Doc claims: "the shared panel (also mounted in the player shell's media-info region)" — consistent with MediaInfoPanel importers. No dead code. No env vars. Open question: none.

