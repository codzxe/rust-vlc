# AUDIT: ui/src/client/wire-time.ts
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-23 |
| Lines / functions | 38 / 3 (secondsToUs, usToSeconds, durationToSeconds) + US_PER_SECOND |
| File verdict | REAL-UNPROVEN (correct unit seam; consumed by both the demo transport and the real player context) |
| Reachable from shipped entry point? | YES — screens/player/context.tsx:28 (every time-tick/duration/seek/snapshot conversion on the REAL event path) + client/demo-transport.ts:32 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | full read; grep consumers (context.tsx:205,254,284,384,441,456,460; demo-transport.ts:414,577,...); Duration type check (generated/commands.ts:38) |

## 1. One-paragraph verdict
The single µs<->seconds conversion seam, and it is actually single: the REAL player context converts every wire Timestamp through it (time-tick L441, duration-known L456, seek-completed L460, snapshot L284, buffered L205, seek send L384), and the demo transport speaks the same µs contract — no ms/µs/seconds split anywhere. Constants named, unknown/infinite durations -> null (honest), `Duration.kind === "known"` matches the generated union (commands.ts:38). No findings. This is a verified-good bridge-client file: the unit-drift class of bug (spec checklist 6.2) is structurally prevented here.

## 2. What actually works (verified)
— verified by grep consumer map (above) + read: 1e6 factor correct both directions; durationToSeconds discriminates known/unknown|infinite -> null.

## 3. Findings
None. Checked: factor, direction, Duration union handling, consumer coverage (real path context.tsx + demo path), no inline 1_000_000/1e6 duplicates in player/playlist providers (grep: none).

## 4. Per-function table
n.a. — <100 LOC (3 trivial functions).

## 5. Test honesty
Wire-unit behavior covered via context/transport tests (other agents' files); the k17 lane log claims the three-way unit split was fixed and tests were added — UNVERIFIED depth here.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every conversion crosses the wire edge HERE, through named constants" | header L15-17 | REAL | grep: all µs conversions in context.tsx + demo-transport.ts route through this module |
| "relative seek deltas are the only true milliseconds (SeekTarget.relative.deltaMs)" | L11-12 | REAL-UNPROVEN | consistent with generated commands.ts SeekTarget; consumer check belongs to context lane |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
generated/commands Duration type. No feature flags.

## 9. Open questions for the next agent
None.
