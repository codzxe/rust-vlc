# AUDIT: ui/src/screens/media-info/MediaInfoProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 99 / 2 (provider + hook) |
| File verdict | REAL — genuinely wired to the engine in the shipped shell (media.get-info is SERVED; media.info-ready is engine-emitted) |
| Reachable from shipped entry point? | YES (App.tsx:320 /media-info -> MediaInfoScreen mounts the provider) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg `media.info-ready` crates/playback-engine/src/events.rs (exit 0 — L105 "**emitted** (3.B4)"); rg SERVED list membership (media.get-info present, apps/cli/src/dispatch.rs:47); read full file |

## 1. One-paragraph verdict
The counter-example to the playlist deadness: this provider is wired to a real backend end-to-end in the shipped desktop shell. `media.get-info` is one of the 15 SERVED commands (apps/cli/src/dispatch.rs:47), and `media.info-ready` is a real engine-emitted event (events.rs:105, "once per open, before the first → playing") — so the /media-info screen shows genuine engine-reported source/duration/track data. Error handling is honest and differentiated: `feature-not-exposed` -> notExposed note; transport rejection -> separate unreachable verdict (w26.k17); other typed errors (not-ready = nothing open) -> honest null. Zero findings. Checked and clean: request id monotonicity, cancelled-flag hygiene on unmount, subscribe cleanup, context guard, no fabricated fallback info anywhere.

## 2. What actually works (verified)
— verified by rg + read: both data paths land on real surfaces (served command + emitted event); the three-state honesty model (info/notExposed/unreachable) has no disguised default.

## 3. Findings
None. Checked: mount-effect race (cancelled flag L57/L73-75), event subscribe leak (returns unlisten L80-82), error-code branching (L61-65), null semantics (L22-26). All correct.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MediaInfoState (iface) | 20-35 | REAL | — | honest tri-state |
| MediaInfoProvider | 39-90 | REAL | — | served command + emitted event |
| useMediaInfo | 92-98 | REAL | — | named guard |

## 5. Test honesty
MediaInfoPanel/Screen tests exist in the lane (mount fake transports). Would fail if the info mapping broke. UNVERIFIED beyond baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "only engine-reported state — the media.get-info command result and media.info-ready events" (L4-5) | header | REAL | both verified live (SERVED + events.rs:105) |
| "`info: null` means 'no media info reported' ... never a fabricated MediaInfo" (L6-7) | header | REAL | read — no default info object exists |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
client/ClientProvider + transport types. No feature flags.

## 9. Open questions for the next agent
- MediaInfoPanel/Screen owners: does the panel surface `unreachable` distinctly (the provider's w26.k17 contract)? If the panel only renders notExposed, the unreachable verdict is dead state.
