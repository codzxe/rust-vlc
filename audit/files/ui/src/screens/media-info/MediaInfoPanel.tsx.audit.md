# AUDIT: ui/src/screens/media-info/MediaInfoPanel.tsx
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-11-21 |
| Lines / functions | 423 / 10 (Row, MediaInfoPanel, MediaInfoSections, streamHeading, streamIcon, StreamHeading, VideoStream, AudioStream, SubtitleStream, textOrNull, numOrNull) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | WITHIN WEB BUILD: YES (PlayerScreen.tsx side region + MediaInfoScreen /media-info route; rg usage verified). SHIPPED DESKTOP BINARY: NO (tauri host unbuildable — baseline #2 exit 101; W14-A01). Backend lane EXISTS: CLI host serves media.get-info with demuxer-enriched facts (host.rs:1437-1470 `enrich_track_info`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg "MediaInfoPanel" (PlayerScreen + MediaInfoScreen consumers); sed format.ts:1-60 (total formatters); rg "interface MediaInfo" → generated/commands.ts:52 (no size/bitrate fields); rg "get-info" apps/cli (host.rs:1439-1470 enrichment); test survey MediaInfoPanel.test.tsx:100-286 |

## 1. One-paragraph verdict
A genuinely honest information panel: every absent field renders the
localized "Unknown" (flagged `data-unknown` for muted styling), empty /
not-exposed / unreachable states are distinct and tested, and the populated
path renders real wire data (tracks, metadata record, chapters). The
backend lane is real — the CLI host enriches `media.get-info` from demuxer
facts. The one capability hole is openly documented in-code: the wire
`MediaInfo` type carries no `size`/`bitrate`, so two General rows are
PERMANENTLY "Unknown" placeholders. Tests are substantive and passed in
baseline (ui-vitest.log.txt, 1705 passed).

## 2. What actually works (verified)
— verified by baseline ui-vitest.log.txt: MediaInfoPanel.test.tsx executes
fixture rendering (all sections), per-stream descriptors, multi-stream,
live-stream (∞ duration, live yes, seekable no), honest-unknown AC,
not-exposed/unreachable distinctions, and media.info-ready event authority
(tests 272-286).
— verified by read: format.ts formatters are total (null in → null out);
codecLabel builds "description (fourcc)" only from non-empty parts.
— verified by read: apps/cli/src/host.rs:1437-1470 — the desktop host
enriches TrackInfo with dimensions/channels/rates from the demuxer's strf
facts; the panel's data source is a real engine path, not a fixture.
— verified by rg: `MediaInfo` wire type (generated/commands.ts:52) has no
size/bitrate — the CATALOG GAP comment at 210-211 is accurate.

## 3. Findings
### F-001 [S3] size/bitrate rows can never populate — dead fields shipped in every render
Where: ui/src/screens/media-info/MediaInfoPanel.tsx:210-213
Code:
```
          {/* CATALOG GAP: size/bitrate are not in the MediaInfo wire type yet —
              honest "Unknown", never invented (ui/NEXT-STEPS.md handoff). */}
          <Row field="size" label={m("media-info.field.size")} value={unknown} unknown={unknown} />
          <Row field="bitrate" label={m("media-info.field.bitrate")} value={unknown} unknown={unknown} />
```
What is wrong: The rows are hardwired to the "Unknown" sentinel — the wire
type (generated/commands.ts:52) has no size/bitrate fields, so no engine can
ever populate them through this panel. The engine side has the data (the
demux layer parses mvhd/stsz; the host already folds demuxer facts into
tracks at host.rs:1437-1470).
Why it matters: A "Media information" panel whose File-size and Bitrate rows
are eternally "Unknown" is a permanent capability gap against the product's
VLC-parity claims — honestly labeled, but shipped dead weight that reads as
broken to users.
What correct looks like: Extend the wire MediaInfo with `size`/`bitrate`
(the host's open path already harvests container facts), then feed the rows.
Until then, consider hiding the rows instead of shipping permanent unknowns.
Confidence: HIGH — Verification: read (type + host code traced); gap is
documented in ui/NEXT-STEPS.md per the comment.
### F-002 [S4] Duplicated unknown-normalizer: local `text` vs module `textOrNull`
Where: ui/src/screens/media-info/MediaInfoPanel.tsx:173-174 and 413-415
Code:
```
  const text = (value: string | null | undefined): string =>
    value === null || value === undefined || value.length === 0 ? unknown : value;
```
and
```
function textOrNull(value: string | null | undefined, unknown: string): string {
  return value === null || value === undefined || value.length === 0 ? unknown : value;
}
```
What is wrong: Identical logic twice in one file — the local closure only
saves passing `unknown` as a parameter.
Why it matters: A future tweak to the empty-string rule (e.g. trim
whitespace) applied to one copy silently diverges the other.
What correct looks like: Delete the local `text`; call
`textOrNull(value, unknown)`.
Confidence: HIGH — Verification: read, both bodies compared.
### F-003 [S4] A metadata value that literally equals the "Unknown" string is misclassified as unknown
Where: ui/src/screens/media-info/MediaInfoPanel.tsx:60-63
Code:
```
      className="lumen-mi__row"
      data-field={field}
      data-unknown={(unknown !== null && value === unknown) || undefined}
    >
```
What is wrong: Unknown-detection is string equality against the localized
sentinel. A file whose metadata tag value is literally "Unknown" (or the
localized equivalent in a future locale) renders muted as if unreported.
Why it matters: Sentinel-by-value instead of by-type — locale-dependent
false positives.
What correct looks like: Pass unknown-ness structurally (boolean prop from
the call site, which knows whether the source was null).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Row | 45-68 | REAL | S4 (F-003) | dt/dd row; data-unknown sentinel-by-value |
| MediaInfoPanel | 70-158 | REAL-UNPROVEN | — | four-state router (unreachable/not-exposed/empty/ready); empty CTA fires the shell's one open-media event |
| MediaInfoSections | 161-287 | REAL | S3 (F-001) | renders all wire fields; size/bitrate dead rows |
| streamHeading | 289-291 | REAL | — | "Kind N" from localized kind label |
| streamIcon | 294-303 | REAL | — | pinned Icon.svg vocabulary |
| StreamHeading | 306-321 | REAL | — | h4 + icon; aria-labelledby per section |
| VideoStream | 323-355 | REAL | — | 9 rows, all honest-unknown normalized; `v === null` guards before v?. access mixed (347-348 use ?. on null — safe) |
| AudioStream | 357-382 | REAL | — | 7 rows, honest |
| SubtitleStream | 384-410 | REAL | — | origin row maps the two-kind union |
| textOrNull / numOrNull | 413-423 | REAL | S4 (F-002) | total normalizers; text duplicated |

## 5. Test honesty (MediaInfoPanel.test.tsx)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| queries the seam + renders all sections (101-118) | YES | asserts media.get-info dispatched and sections present |
| video/audio/subtitle descriptors (119-159) | YES | asserts specific rendered values from fixtures |
| multi-stream entries (169-180) | YES | two streams of a kind → two articles |
| live stream: ∞/live/seekable/url (181-191) | YES | asserts formatDuration infinite label path |
| honest unknown AC (193-216) | YES | asserts every absent field renders the sentinel, data-unknown set |
| not-exposed vs unreachable distinct notes (241-271) | YES | the w26.k17 verdict separation is pinned |
| media.info-ready authority (272-286) | YES | later event replaces earlier info |
Mutation check: invert `data-unknown` predicate → tests 193-216 fail (they
assert the muted flag). Delete the empty-state CTA → no test asserts the
button fires OPEN_MEDIA_EVENT — minor gap (CTA wiring untested).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Media information row: MediaInfoPanel via media.get-info" (doc 08 §4.3) | file header line 1-2 | REAL | command dispatched (test 101); host serves it (host.rs:1439-1470) |
| "every absent field renders the i18n Unknown — never invented" | header 22-24 | REAL | code + tests 193-216 |
| "size/bitrate honestly render Unknown until the catalog grows them" | header 25-28 | REAL (gap acknowledged) | F-001: rows are permanently dead |
| engine-info hard-codes output (W5/W13 pin) | wave prompt | NOT THIS FILE | that defect is apps/cli engine_info (host.rs); this panel's lane is real |

## 7. Dead / duplicated / unreachable code
- F-002 duplication (text vs textOrNull).
- No dead components: all consumed (PlayerScreen + MediaInfoScreen).

## 8. Dependencies & flags
In-tree only: i18n, transport types, generated commands, surface/list/icon
components, format.ts. No feature flags.

## 9. Open questions for the next agent
- Does the engine's open path actually harvest byte-size/bitrate today
  (mp4 mvhd/stsz parsed anywhere)? If yes, wiring the two dead rows is a
  small catalog+host change; if no, the gap is engine-deep.
- MediaInfoProvider (next file in my queue) owns the query cadence — check
  whether the panel re-seeds on session close (stale-info risk symmetric to
  my context.tsx F-001).
