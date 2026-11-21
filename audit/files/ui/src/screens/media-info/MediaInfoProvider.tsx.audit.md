# AUDIT: ui/src/screens/media-info/MediaInfoProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 98 / 3 (MediaInfoProvider, seed effect, subscribe effect, useMediaInfo) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (mounted by MediaInfoScreen — the /media-info route — and via index barrel; panel consumed in PlayerScreen's side region). Backend lane: media.get-info real (host.rs serves with demuxer-enriched facts). SHIPPED DESKTOP BINARY: NO (baseline #2; W14-A01) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 1 |
| Commands run | rg MediaInfoProvider mounts; read both effects; cross-ref my context.tsx F-001 (idle-reset family); MediaInfoPanel.test.tsx coverage check (272-286 event authority) |

## 1. One-paragraph verdict
A small, clean state seam: seeds from `media.get-info`, stays authoritative
from `media.info-ready`, separates not-exposed from unreachable verdicts
(tested). Two real state-lifetime defects: `info` is never cleared when a
session closes (the panel keeps rendering the previous media's facts while
nothing is open — contradicting the header's own "`info: null` means
nothing open" contract), and `notExposed`/`unreachable` latch forever —
once unreachable, later-arriving info is shadowed by the unreachable note.

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt (1705 passed): MediaInfoPanel.test
272-286 pins that `media.info-ready` events keep the panel authoritative
(full info → sparse info replacement through this provider).
— verified by read: the seed effect's cancellation flag (49, 57, 73-75) is
correct for transport changes; the verdict split (notExposed vs unreachable,
61-70) matches the panel's three-state rendering.
— verified by rg: mounted on the /media-info route + player side region.

## 3. Findings
### F-001 [S3] `info` survives session close — stale media facts render while nothing is open
Where: ui/src/screens/media-info/MediaInfoProvider.tsx:79-83 (vs header 6-7)
Code:
```
  // Stay authoritative from `media.info-ready` events (new media opened).
  useEffect(() => {
    return transport.subscribe((event: Event) => {
      if (event.kind === "media.info-ready") setInfo(event.info);
    });
  }, [transport]);
```
(and the header contract, lines 6-7:)
```
// honestly-labelled unknowns. `info: null` means "no media info reported"
// (nothing open / command failed), never a fabricated MediaInfo.
```
What is wrong: No event clears `info` — not `session.state-changed` to
idle, not session.close. After the user stops/closes media A, the panel
keeps rendering A's tracks/duration/title while the engine is idle. The
header's claim ("info: null means nothing open") is only true before the
first media, not after the last one closes.
Why it matters: Same stale-session-state family as my context.tsx F-001
(S2 there because the seekbar ACTS on stale duration; here it is a read-only
panel, so S3). The product's honesty doctrine is the file's own header —
the code violates it.
What correct looks like: subscribe also to `session.state-changed` and
clear `info` on `idle`.
Confidence: HIGH — Verification: read (no clearing path exists); no test
covers post-close behavior.
### F-002 [S3] `notExposed`/`unreachable` latch forever — recovered state keeps the wrong verdict
Where: ui/src/screens/media-info/MediaInfoProvider.tsx:61-70
Code:
```
          if (result.error.code === "feature-not-exposed") setNotExposed(true);
          // Other error codes (not-found / not-ready / …) = nothing open:
          // keep `info: null` — the honest empty state.
      } catch {
        // Host unreachable (w26.k17, j17 P3-8) — its OWN honest verdict, not
        // the not-exposed one: the command layer may expose the command
        // while the transport is dead.
        setUnreachable(true);
```
What is wrong: Both flags only ever set true. If the initial probe rejects
(transport briefly down) and the transport then recovers — evidenced by a
`media.info-ready` arriving through the live subscription — the panel STILL
renders the unreachable note first (MediaInfoPanel renders `unreachable`
before `notExposed` before `info`, lines 86-120 of my panel audit): real
data shadowed by a stale verdict.
Why it matters: A transient failure permanently mislabels the surface;
the honest-verdict design (j17 P3-8) has no recovery half.
What correct looks like: On any successful seed or `media.info-ready`
event, clear both flags.
Confidence: HIGH — Verification: read; panel render order verified in
MediaInfoPanel.tsx:86-120.
### F-003 [S4] Per-instance requestId counter (echo-only ids, duplicate wire ids)
Where: ui/src/screens/media-info/MediaInfoProvider.tsx:41,50-54
Code:
```
  const nextRequestId = useRef(0);
  ...
      nextRequestId.current += 1;
```
What is wrong: Same pattern as context.tsx:356-359 and
NetworkStreamScreen:100 — every provider counts from 1, duplicate ids across
components on one transport. Harmless (request_id echo-only, dispatch.rs:63)
but defeats log correlation.
What correct looks like: Shared module-level counter.
Confidence: HIGH — Verification: read + dispatch.rs echo verified in my
context.tsx audit.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MediaInfoProvider | 39-90 | REAL-UNPROVEN | S3 (F-001/F-002) | clean seam; state-lifetime holes |
| seed effect | 48-76 | REAL | — | cancelled-flag correct; verdict split correct |
| subscribe effect | 79-83 | REAL | S3 (F-001) | no idle clearing |
| useMediaInfo | 92-98 | REAL | — | throws without provider (canonical pattern) |

## 5. Test honesty (MediaInfoPanel.test.tsx — the provider's only coverage)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| media.info-ready keeps panel authoritative (272-286) | YES | full → sparse replacement through the provider |
| not-exposed note with MockTransport (241-253) | YES | verdict flag pinned |
| host unreachable renders its OWN note (254-271) | YES | the j17 P3-8 split pinned |
Mutation check: make the seed effect swallow completions → tests 101-118
(panel renders all sections) fail. Gaps: NO test for post-close staleness
(F-001) and NO test for flag recovery (F-002) — both untested lifecycles.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "`info: null` means 'no media info reported' (nothing open / command failed), never a fabricated MediaInfo" | header 4-7 | SHALLOW | F-001: after close, non-null info with nothing open |
| "engine-owned, UI-rendered" (doc 08 §4.2) | header 1 | REAL | events + command are the only sources |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
client (useClient), transport types. No feature flags. The context is
provided at screen level (not app level) — each mount re-probes; acceptable.

## 9. Open questions for the next agent
- Does the engine emit any event on session close that could clear info
  (e.g. `session.state-changed` to idle is available on the same
  subscription — the fix is one case away)?
- Should the provider mount app-wide instead of per-screen (two mounts =
  two probes)? PlayerScreen + MediaInfoScreen both mount it today.
