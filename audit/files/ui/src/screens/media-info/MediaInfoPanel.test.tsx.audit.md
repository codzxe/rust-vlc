# AUDIT: ui/src/screens/media-info/MediaInfoPanel.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR2 (recovery; original agent W16-A10) |
| Date | 2026-06-14 |
| Lines / functions | 292 / 4 helpers + 3 describe blocks (13 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test lane — baseline ui-test.txt:63 `✓ src/screens/media-info/MediaInfoPanel.test.tsx (13 tests) 478ms`; code under test ships via the /media-info route (MediaInfoPanel.tsx, 423 L) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `grep -n MediaInfoPanel audit/runs/baseline/ui-test.txt` -> :63; `wc -l` fixtures.ts 159 L / MediaInfoPanel.tsx 423 L; `sed -n 1,30p` fixtures.ts (wire µs durations verified) |

## 1. One-paragraph verdict
Solid fixture-driven suite that pins the honesty contract of the media-info surface: absent fields render "Unknown" rather than invented values, empty vs not-exposed vs unreachable are three DISTINCT verdicts (the w26.k17 mislabel regression is guarded at :254-270), and `media.info-ready` events keep the panel current. The FakeTransport answers the real `media.get-info` command from wire-shaped fixtures — a test double of the boundary, not of the subject. One minor hygiene point: field assertions read concatenated label+value textContent.

## 2. What actually works (verified)
— verified by baseline ui-test.txt:63 (13/13 pass; lane 1705 passed :178-179). Mutation reasoning: if the panel invented a default for an absent title, :199 fails; if a rejecting transport were mislabeled not-exposed, :267 fails; if the first command weren't `media.get-info`, :108 fails.

## 3. Findings
F-001 [S4]
Where: ui/src/screens/media-info/MediaInfoPanel.test.tsx:113-116
Code:
```
    expect(fieldValue(container, "title")).toBe("TitleBig Buck Bunny");
    expect(fieldValue(container, "source")).toBe("Source/media/big-buck-bunny.mkv");
    expect(fieldValue(container, "duration")).toBe("Duration10:35");
    expect(fieldValue(container, "chapters")).toBe("Chapters1");
```
What is wrong: assertions compare `row.textContent` glued without any separator, i.e. they currently pass only because label and value are adjacent text nodes with no whitespace between them; any cosmetic whitespace/CSS-gap refactor silently breaks them (or, worse, a formatting fix makes them keep passing with hidden glyph changes invisible). Why it matters: whitespace-fragile assertions churn; they also assert the DOM lacks a space, which is a rendering accessibility nit the suite has silently pinned as correct. What correct looks like: assert the label element and value element separately. Confidence: HIGH — Verification: read (current behavior verified via passing baseline; the fragility is structural).

## 4. Per-function table (file >100 LOC)
| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FakeTransport | 28-76 | REAL | — | answers real media.get-info cmd from fixture or typed error |
| renderPanel/fieldValue/section | 78-98 | REAL | — | real provider chain; query helpers assert-then-cast |
| fixture rendering suite | 100-190 | REAL | — | 7 tests: sections, stream descriptors, external subs, metadata, multi-stream, live ∞ |
| unknown-field AC suite | 192-223 | REAL | — | absent→Unknown incl. CATALOG GAP size/bitrate :202-204 |
| data flow + honest states | 225-291 | REAL | S4 | empty / not-exposed (real MockTransport :242) / unreachable (dead inline transport :257-260) / event-driven replacement :286-289 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| unknown-field :193-215 | YES | exact string per absent field |
| unreachable verdict :254-270 | YES | asserts data-state="unreachable" AND absence of not-exposed |
| events keep authoritative :272-290 | YES | second event must replace rendered container value |
| duration 10:35 :115 | YES | wire µs 635_000_000 (fixtures.ts:12) must format to 10:35 — units conversion is genuinely under test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| 13 tests run | baseline ui-test.txt:63 | REAL | `✓ ... (13 tests) 478ms` |

## 7. Dead / duplicated / unreachable code
None. `ErrorCode` type (:25) covers both used branches.

## 8. Dependencies & flags
Fixtures (fixtures.ts, 159 L) shaped exactly like the generated MediaInfo — verified µs wire units in header + :12.

## 9. Open questions for the next agent
None.
