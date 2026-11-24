# AUDIT: ui/src/screens/media-info/MediaInfoPanel.tsx
| Field | Value |
|---|---|
| Auditor | W15-A06 |
| Date | 2026-02-27 |
| Lines / functions | 423 / 9 (Row, MediaInfoPanel, MediaInfoSections, streamHeading, streamIcon, StreamHeading, VideoStream, AudioStream, SubtitleStream, textOrNull, numOrNull) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — App.tsx /media-info route -> MediaInfoScreen -> MediaInfoPanel; also mounted in PlayerScreen side region (rg: 2 mounts) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `rg -n "media.get-info" MediaInfoProvider.tsx` (real send at L53-55); `rg -n "MediaInfo = " src/generated/commands.ts` (L52 — no size/bitrate fields, catalog-gap comment honest); `rg -c "it("` MediaInfoPanel.test.tsx (16 tests); full read |

## 1. One-paragraph verdict
Honest, genuinely-wired read-only panel: `useMediaInfo()` sends the real `media.get-info` command over the client transport (MediaInfoProvider.tsx:53-55, session-scoped, no args), renders only engine-reported values, and flags every absent field `data-unknown` with a single i18n "Unknown" — no fabricated numbers anywhere. The one user-visible dead surface: the General card renders `size` and `bitrate` rows that are PERMANENTLY "Unknown" because the generated wire type genuinely lacks those fields (verified: `MediaInfo = { chapters; container?; duration; isLive; metadata; seekable; source; title?; tracks }` — commands.ts:52). Two dead rows shipped per design, disclosed in-comment as a catalog gap with a NEXT-STEPS handoff.

## 2. What actually works (verified)
— verified by `rg`/`sed`: MediaInfoProvider.tsx:53-55 sends `{ cmd: "media.get-info" }` via transport.send and handles `feature-not-exposed`/unreachable verdicts distinctly; 16 tests in MediaInfoPanel.test.tsx; baseline vitest 1705 passed. The unreachable/notExposed/empty/ready four-state rendering is distinct per state (L86-157).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/media-info/MediaInfoPanel.tsx:212-213
Code:
```
          <Row field="size" label={m("media-info.field.size")} value={unknown} unknown={unknown} />
          <Row field="bitrate" label={m("media-info.field.bitrate")} value={unknown} unknown={unknown} />
```
What is wrong: size and bitrate rows render the unknown sentinel unconditionally — the values can never be anything but "Unknown" because the generated MediaInfo type (src/generated/commands.ts:52) has neither field. Honest (flagged `data-unknown`, muted styling) but permanently dead rows in the shipped UI. Why it matters: a user opening Media Info on any file sees two fields that can never populate — surface area with zero information capacity. What correct looks like: add `size`/`bitrate` to the Rust catalog + regen (the file's own comment records this handoff in ui/NEXT-STEPS.md), or hide rows until the catalog grows. Confidence: HIGH — Verification: read + generated type inspected.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Row | 45-68 | REAL | — | data-unknown honest sentinel |
| MediaInfoPanel | 70-158 | REAL | — | 4-state: unreachable / not-exposed / empty / ready; empty CTA dispatches OPEN_MEDIA_EVENT (the app's one open funnel) |
| MediaInfoSections | 161-287 | REAL | S3 | F-001 dead rows; metadata entries rendered as-is |
| streamHeading/streamIcon/StreamHeading | 289-321 | REAL | — | numbered sub-headings, registry glyphs |
| VideoStream | 323-355 | REAL | — | all fields null-guarded |
| AudioStream | 357-382 | REAL | — | channels/sampleRate/bitDepth honest |
| SubtitleStream | 384-410 | REAL | — | origin row maps external-subtitle vs stream |
| textOrNull/numOrNull | 413-423 | REAL | — | sentinel helpers |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| MediaInfoPanel.test.tsx (16 it) | YES | fixture-driven (fixtures.ts), asserts unknown-flagging and section rendering (test ids media-info-empty/-unreachable/-not-exposed) |
Not executed individually — baseline vitest 1705 passed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the command result carries no `size`/`bitrate` fields (see generated/commands.ts MediaInfo)" (L25-27) | header | REAL | commands.ts:52 verified — no size/bitrate |
| "every absent field renders the i18n 'Unknown'" (L22-23) | header | REAL | textOrNull/numOrNull + Row data-unknown (L62) |

## 7. Dead / duplicated / unreachable code
- The size/bitrate rows (F-001) — reachable UI, permanently-inert values.
- No dead functions.

## 8. Dependencies & flags
Wire types from src/generated/commands.ts via src/client/transport.ts (single import-site rule, documented L1-5). No flags. No local state — provider-fed, session-scoped.

## 9. Open questions for the next agent
- Does the ENGINE side actually implement `media.get-info` with real demux data (crates/commands)? The UI is wired; the backend truth is another domain's audit. UNVERIFIED here.
- The empty-state CTA fires OPEN_MEDIA_EVENT — confirm PlayerScreen listens (the ControlBar audit already saw the same seam used by the app menu: consistent).
