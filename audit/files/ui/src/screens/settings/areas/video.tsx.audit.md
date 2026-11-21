# AUDIT: ui/src/screens/settings/areas/video.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 170 / 4 (useAreaMsg/hintMsg consts, ResetPictureRow, VideoArea, CodecsFold) |
| File verdict | REAL-UNPROVEN as UI (schema-driven, live controls, working in-session reset/apply) — but the 13 `video.*` keys it edits have ZERO engine consumers (SHALLOW as a settings feature) |
| Reachable from shipped entry point? | NO at HEAD (host crate unbuildable, W14-A01 E0425). Mounted at /settings/video via areas registry; jsdom-tested in baseline #23. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg -c '^key = "video\.' schemas/settings.toml` → 13 (all frame rows seeded, contrast with subtitles' 0); `rg -rln "wallpaper" crates apps --glob '*.rs'` → none; `rg -ln "start-fullscreen" crates apps --glob '*.rs'` → none — no engine code reads any video.* value; `rg -n "resetField" SettingsProvider.tsx` → :134 (draft-only reset); areas/index.tsx registry mount verified; sed quote verification. |

## 1. One-paragraph verdict
Unlike the subtitles area (0 schema keys, all staged), the video area is the real thing on the UI side: all 13 frame rows exist in schemas/settings.toml, SettingField renders them live, the Reset Picture row performs a genuine draft reset through the settings store, and the codecs fold is an honest in-flow disclosure. The hole is below the UI: no Rust code anywhere reads `video.enable`, `video.start-fullscreen`, `video.wallpaper`, `video.deinterlace`, or any other `video.*` key — the values are write-only to the engine (and per W11 they never persist host-side either). "Enable video OFF" would not disable video; "Start videos in fullscreen ON" does nothing; the five picture sliders adjust nothing. The screen edits a settings store whose video half has no consumer — the W11 disease one layer up.

## 2. What actually works (verified)
— verified by `rg -c '^key = "video\.' schemas/settings.toml` → 13: every row the module renders is schema-backed (contrast: subtitles area has 0).
— verified by reading SettingsProvider.tsx:134-139: resetField genuinely resets the draft to schema defaults (then the dirty-gated apply bar commits) — real in-session behavior, tested (video.test… in baseline #23's 1705-pass set).
— verified by reading areas/index.tsx: VideoArea + CodecsFold compose the /settings/video route; AdvancedDisclosure is the shell's ONE shared component (the dedup claim at :156-161 checks out).

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/settings/areas/video.tsx:119-146 (+ schemas/settings.toml:822-971; engine consumer census: none)
Code (video.tsx:131-135):
```
          <section key={section.id} className="settings-section" data-section={section.id}>
            <h2 className="settings-section__title">{m(section.title as MessageKey)}</h2>
            {visible.map((entry) => (
              <SettingField key={entry.key} entry={entry} hint={VIDEO_HINTS[entry.key]} />
            ))}
```
What is wrong: The 13 `video.*` schema keys this area renders (enable / start-fullscreen / disable-screensaver / wallpaper / hue / brightness / contrast / saturation / gamma / deinterlace / deinterlace-method / aspect-ratio / crop — schemas/settings.toml:822-971) have no consumer in any Rust crate: `rg -rln "wallpaper" crates apps --glob '*.rs'` → nothing; `rg -ln "start-fullscreen" …` → nothing. The UI writes drafts and (on Apply) sends config.set-setting to a host that never persists them (W11 S2: process-local BTreeMap) and never reads them back. Every control on this screen is live-looking and functional-looking while editing values that influence no behavior in any shipped binary.
Why it matters: This is the W8 `audio.set-eq` / W11 pattern with the widest blast radius in the settings family: a whole "Video Defaults" screen, honestly schema-driven, whose entire effect is a localStorage draft. A user who disables video or sets fullscreen-start sees no change, ever.
What correct looks like: engine-side consumers (open path reads start-fullscreen; the video pipeline reads enable/adjust values) plus host persistence — until then, mark the area's rows staged the way subtitles.tsx does, honestly.
Confidence: HIGH — Verification: consumer census (rg over crates/apps), W11 SUMMARY cross-ref, schema read.
Severity: S2 (cross-ref W11's persistence S2 — this is the distinct no-consumer half).

### F-002 [S4]
Where: ui/src/screens/settings/areas/video.tsx:52-53
Code:
```
/** The ⓘ hint msgid cast through the registry seam (the laneMsg precedent). */
const hintMsg = (key: I14Key): MessageKey => key as string as MessageKey;
```
What is wrong: A double-cast that defeats the type system to pass lane keys as MessageKeys. If the lane key is ever merged into the catalog under a different id, the cast hides the mismatch until runtime (makeAreaMsg would return the fallback silently).
Why it matters: The lane-key seam is otherwise carefully built (identical-before/after-merge contract); this cast is the one place drift compiles fine and fails quietly.
What correct looks like: type the VIDEO_HINTS map as the union `MessageKey | I14Key` and let SettingField accept both (the dialogs' SubtitlesLabelKey precedent).
Confidence: HIGH — Verification: read; cast semantics traced.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useAreaMsg / hintMsg | 46-53 | REAL | S4 | F-002 double-cast |
| VIDEO_HINTS | 55-66 | REAL | — | 8 frame circles reproduced; sliders/reset carry none (honest) |
| search-terms note | 68-72 | REAL | — | documents a RETIRED feature honestly (cross-area search walks schema only) |
| PICTURE_KEYS | 74-81 | REAL | — | the five generated schema constants (order = schema order) |
| ResetPictureRow | 89-112 | REAL | — | genuine draft reset loop; never a dead control (verified store call) — but resets values nothing consumes (F-001) |
| VideoArea | 119-146 | REAL | S2 | F-001; schema-driven sections; per-row hints |
| CodecsFold | 163-170 | REAL | — | shared AdvancedDisclosure; collapsed by default (frame-true) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| video.test… / codecs.test.tsx (baseline #23) | partial | render/section/hint/reset assertions are mutation-sensitive; nothing can test engine consumption because there is none |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the schema seed carries every row" | video.tsx:4-5 | REAL | 13/13 keys in schemas/settings.toml:822-971 |
| "A real behavior: resets the five picture keys … never a dead control" | video.tsx:86-87 | REAL as UI / SHALLOW as effect | resetField verified; but the reset values feed no consumer (F-001) |
| "an ACTUAL in-flow disclosure" (codecs fold) | video.tsx:16-20 | REAL | AdvancedDisclosure composition verified |

## 7. Dead / duplicated / unreachable code
No dead code in-file (the retired search-term block is a comment documenting removal). Unreachable from shipped binaries (W14-A01).

## 8. Dependencies & flags
Generated schema constants (SETTING_VIDEO_*), SettingsProvider, SettingField, AdvancedDisclosure, CodecsArea, lane keys w26.gfixB/w26.i14. No flags. typecheck green (#22).

## 9. Open questions for the next agent
- Engine lane: is reading `video.*` at session-open (start-fullscreen, enable) on any roadmap (ui/NEXT-STEPS.md)? That would retire half of F-001.
- codecs.tsx auditor: same no-consumer question for the codecs fold's rows.
