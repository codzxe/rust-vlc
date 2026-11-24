# AUDIT: ui/src/screens/player/tools/video/CropTool.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A09) |
| Date | 2026-11-24 |
| Lines / functions | 95 / 9 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — vitest baseline ui-test.txt:145 (9 tests, 316ms); subject mounted via tools/video/harness.tsx + settings/areas/effects.tsx |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg "set-crop" (catalog HAS VideoSetCrop), rg useClient/send in CropTool.tsx (none), it-count 9 == baseline |

## 1. One-paragraph verdict
Frame-pinning suite, all green in the baseline: collapsed shell + aria wiring, four labeled
number inputs at "0"/"px", two link toggles defaulting to locked, no separators (negative
assertion), pair-mirroring edits, unlink independence, digit sanitization ("1a2b"→"12", linked
pair follows), and the staged-markers test. The sharp edge versus its ColorsTool sibling: the
pending command here — `video.set-crop` — EXISTS in the engine catalog
(crates/commands/src/catalog/ids.rs:127 `VideoSetCrop => "video.set-crop"`), yet CropTool.tsx
(213 LOC) contains no useClient/send: the card never dispatches the real command it names.

## 2. What actually works (verified)
— verified by citation audit/runs/baseline/ui-test.txt:145 `✓ src/screens/player/tools/video/
CropTool.test.tsx (9 tests) 316ms` (1705-passed run); 9 it() blocks on disk match.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/tools/video/CropTool.tsx:173 + catalog/ids.rs:127 (test :87-94)
Code (test, verbatim):
    expect(card).toHaveAttribute("data-staged-tool", "crop");
    expect(card).toHaveAttribute("data-pending-command", "video.set-crop");
Evidence (catalog, rg): `VideoSetCrop => "video.set-crop"` (ids.rs:127; command.rs:225-226).
What is wrong: unlike ColorsTool's fictional `video.set-colors`, the crop command is REAL in the
engine's command catalog — but the UI card never sends it (no useClient, no send, no transport
import in CropTool.tsx). The user adjusts crop values and the engine's crop capability is
never engaged. Why it matters: the backend capability + the frontend control both exist and
are not connected — a wider gap than a not-yet-implemented command, though the visible "staged"
note keeps it honest to the user. What correct looks like: wire the card's commit to
`video.set-crop { crop: CropSpec }` (the generated commands.ts already types it) or drop the
card until wired.
Confidence: HIGH — Verification: read (rg both sides).

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| ships collapsed (11) | REAL | — | aria + hidden body |
| defaultOpen (21) | REAL | — | |
| four inputs 0 px (30) | REAL | — | |
| two link toggles locked (38) | REAL | — | aria-pressed true |
| no separators (46) | REAL | — | negative count |
| linked pair mirrors (53) | REAL | — | other pair untouched |
| unlink independent (64) | REAL | — | |
| digit sanitization (76) | REAL |— | "1a2b"→"12", link follows |
| honest staging (87) | REAL | S3 | F-001's subject fact |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 9 | YES | DOM-bound (values, aria states, class counts); e.g. sanitization regression → :81-82 fail |
The suite does not pretend dispatch — it pins the staged markers instead (honest posture). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the staged markers" honest-staging pattern | header 4 | REAL | test 87-94 |

## 7. Dead / duplicated / unreachable code
None in the test file.

## 8. Dependencies & flags
In-repo (CropTool). No transport dependency — consistent with the staged pattern (F-001).

## 9. Open questions for the next agent
Does ANY shipped surface dispatch video.set-crop today (rg: only the catalog + generated types)?
If not, the engine crop path is UNREACHABLE from the UI despite being fully cataloged.
