# AUDIT: ui/src/screens/settings/areas/audio-fixtures.ts
| Field | Value |
|---|---|
| Auditor | W15-R1 (crediting W15-A07) |
| Date | 2026-11-24 |
| Lines / functions | 27 / 1 fn (makeFixtureTransport) + 1 type (FixtureResponder) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — test seam: imported only by 4 test files (audio/video/subtitles/effects .test.tsx); the sole production mention is a comment (audio.tsx:282). Not in the shipped bundle (unreachable from app imports). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg (consumers, CommandTransport iface, never-touch assertions) -> hits cited; sed -n '10,14p' -> quote verified; baseline ui-test.txt:62 |

## 1. One-paragraph verdict
A correct, minimal test double: a configurable `CommandTransport` that records every sent request and replies synchronously from a responder callback, mirroring the real transport's gen semantics (`++generation` per send, transport.ts:69). Its honesty contract is real and exercised — sibling area tests use it to prove video/subtitles NEVER send commands (`expect(transport.sent).toHaveLength(0)`, video.test.tsx:135, subtitles.test.tsx:212). Only defect: the file name says "audio" but it serves four areas' tests.

## 2. What actually works (verified)
— verified by rg: 4 test-file importers, zero production importers (audio.tsx:282 is a comment) — the seam stays out of the shipped bundle.
— verified by interface conformance read: `send`/`subscribe` match CommandTransport (transport.ts:38-40); gen increments per send exactly like MockTransport (transport.ts:69).
— verified by baseline ui-test.txt:62 `✓ audio.test.tsx (11 tests)` — the fixture transport carries passing tests.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/settings/areas/audio-fixtures.ts:1-4 (file name vs consumers)
Code: (verbatim, sed-verified quote of the signature block)
```
export type FixtureResponder = (request: CommandRequest) => CommandResponse["result"];

export function makeFixtureTransport(
  respond: FixtureResponder,
): CommandTransport & { readonly sent: readonly CommandRequest[] } {
```
What is wrong: the file is named `audio-fixtures` but is imported by video.test.tsx:11, subtitles.test.tsx, effects.test.tsx:19 — it is the settings-areas fixture module, not audio's. Why it matters: misleading name sends the next maintainer to the wrong lane; a future "video-fixtures.ts" would duplicate this. What correct looks like: rename to areas test-seem (e.g. `testTransport.ts`), keeping the react-refresh rationale (comment lines 2-3 — keeping helpers out of component modules) intact. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| makeFixtureTransport | 12-27 | REAL | — | records `sent`, per-send gen, sync respond; subscribe is a no-op unsubscribe |
| FixtureResponder (type) | 10 | REAL | — | used in-file; exported but no external importer (in-file only — acceptable for a seam) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| video.test.tsx:133-135 / subtitles.test.tsx:210-212 (never touches transport) | YES | a smuggled engine call in those areas pushes to `sent` and breaks `toHaveLength(0)` — the fixture makes absence provable |
| audio.test.tsx:65-104 (respond-based cases) | YES | responder returns canned results per command; wrong command or wrong payload fails downstream assertions on rendered state |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Kept out of audio.tsx so the component module only exports components (react-refresh)" | :2-3 | REAL | rg: audio.tsx contains no fixture import/export; only comment :282 |
| "shared with the video/subtitles area tests, which assert those areas never touch the transport at all" | :3-4 | REAL | video.test.tsx:135 + subtitles.test.tsx:212 assert `sent` length 0 |

## 7. Dead / duplicated / unreachable code
None. (Deliberately unreachable from production — that is its purpose; see reachability row.) No duplication with client/transport.ts MockTransport: this one is responder-configurable and records `sent`; MockTransport is the canned full client (transport.ts:49).

## 8. Dependencies & flags
Types only from client/transport (CommandRequest/Response/Transport). No feature gates, no runtime deps, no i18n.

## 9. Open questions for the next agent
None.
