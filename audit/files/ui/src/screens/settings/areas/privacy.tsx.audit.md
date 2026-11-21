# AUDIT: ui/src/screens/settings/areas/privacy.tsx
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 137 / 4 (useAreaMsg binding, PRIVACY_HINTS, ClearHistoryRow, PrivacyArea) |
| File verdict | REAL (composition + a11y work; but the screen's live rows are settings wired to nothing — S2) |
| Reachable from shipped entry point? | YES in the web UI (route /settings/privacy → SettingsScreen → areas/index.tsx:246 `privacy: PrivacyArea`; nav test areas.test.tsx:228-231). NO desktop binary exists (W14-A01 E0425). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | Read areas/shared.tsx (makeAreaMsg + schemaSectionViews — filters featureExposed, no search filter); Read SettingField.tsx:1-120 (row grammar, live controls wired to useSettings); sed schema.ts:746-900 + :2592-2660 (privacy sections history 3 rows + network 5 rows; telemetry constants SETTING_PRIVACY_TELEMETRY/CRASH_REPORTS/USAGE_STATS exist at :686-746 with featureExposed:true but are in NO SETTINGS_AREAS section and NOT in SETTINGS_ENTRIES :2514-2521 — the ui=false exclusion is real); rg `save-recent-items\|remember-playback-position\|never-remember-matching\|fetch-metadata-online\|submit-lastfm\|lastfm` over crates/ + apps/ --type rust → **ZERO matches — no engine code reads ANY of the 8 privacy keys**; rg same keys in ui/src outside generated/i18n → ZERO functional consumers; rg `history` in generated/commands.ts → no clear/history command (ClearHistoryRow's claim TRUE); Read i18n/keys/w26.i13.ts + Grep messages.ts:2749-2751/:2626 (all ClearHistoryRow keys resolve centrally); Read areas.test.tsx:159-163 (9 ⓘ assertion) + :228-231 (nav); Read i18n/keys/w26.gfixB.ts:64-66 (duplicate lane copies). Baseline: ui-vitest.log.txt 136 files / 1705 passed. |

## 1. One-paragraph verdict
The composition is careful: schema-driven sections via the shared layer, per-row ⓘ hints on all 8 schema rows plus the hand Clear-history action row (test-pinned at 9), the former telemetry section genuinely absent from the UI (the ui=false constants exist in generated/schema.ts but are wired into no area — verified), and ClearHistoryRow correctly disables its button with an honest reason instead of a dead-looking live control. The S2 is the screen's core product truth: all 8 schema rows render as LIVE, writable controls whose keys have ZERO consumers anywhere — rg over every Rust crate and app returns no reader of `privacy.save-recent-items`, `privacy.remember-playback-position`, `privacy.fetch-metadata-online`, `privacy.submit-lastfm`, or any last.fm key, and the UI itself has no recents/resume store reading them either. A user toggling "Save recently played items" or typing a last.fm password gets a successful SettingApplied write into an in-memory map that nothing reads and that never persists (cross-ref W15-A08 SettingsProvider F-001 S2) — while the equally-absent recording/casting/streaming features one screen over render honestly DISABLED with staged reasons. Same absence, opposite presentation: this screen's rows promise privacy behavior the product cannot perform.

## 2. What actually works (verified)
— verified by areas.test.tsx:159-163 (9 ⓘ dots on /settings/privacy — 8 schema + Clear action), :228-231 (nav to Privacy with aria-current), and the routed-render assertions at :46 (privacy in the area list) + baseline ui-vitest (136 files, 1705 passed).
— verified by reading shared.tsx:73-92: schemaSectionViews("privacy") returns exactly the two sections (history 3 + network 5) with featureExposed filtering — the telemetry "data" section cannot leak in because it is not in SETTINGS_AREAS at all (schema.ts:2625-2640 read).
— verified by rg: generated/commands.ts contains NO history/clear command — ClearHistoryRow's header claim "no playback-history store or clearing command exists in the catalog yet" is TRUE, and the button honestly disables with the reason string (messages.ts:2751 "History clearing arrives with the playback-history store").
— verified by Grep messages.ts: all five ClearHistoryRow keys (settings.privacy.clear/clear-history/clear-unavailable, settings.hint.privacy.clear-history, settings.field.more-info) resolve in the central catalog — no undefined-label risk.

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/settings/areas/privacy.tsx:116-134 (the live schema rows), with the keys at schema.ts:746-900
Code:
```ts
            {visible.map((entry) => (
              <SettingField key={entry.key} entry={entry} hint={PRIVACY_HINTS[entry.key]} />
            ))}
```
(what those rows are, from the central catalog, messages.ts:2506-2510)
```ts
  "settings.privacy.save-recent-items": "Save recently played items",
  "settings.privacy.remember-playback-position": "Remember the playback position",
  "settings.privacy.fetch-metadata-online": "Fetch titles and cover art from the internet",
  "settings.privacy.submit-lastfm": "Submit played tracks to last.fm",
```
What is wrong: All 8 rows render as live, writable controls (Switch/TextField via SettingField → useSettings → config.set-setting). The keys they write have ZERO consumers in the entire product: `rg 'save-recent-items|remember-playback-position|never-remember-matching|fetch-metadata-online|submit-lastfm|lastfm' crates/ apps/ --type rust` → no matches; the same keys in ui/src outside generated/i18n → no matches. There is no recents store, no resume logic, no metadata fetcher, and no last.fm submitter anywhere in the codebase — the engine cannot keep recently-played items, cannot resume, cannot fetch cover art, cannot scrobble. Every write answers SettingApplied (the host serves config.set-setting) into the engine's dead in-memory store (W15-A08 F-001 S2, cross-ref W11-A02: zero persistence), so the write is accepted, read by no one, and lost on exit.
Why it matters: This is the checklist's "UI settings wired to nothing" in its purest form, on the PRIVACY screen of all places: a user who turns OFF "Submit played tracks to last.fm" or "Fetch titles and cover art from the internet" is led to believe they have disabled network behavior that does not exist to be disabled — and one screen over, the Recording area honestly disables identical-absence features with staged reasons. The honesty asymmetry between areas is a product-level lie by presentation.
What correct looks like: Either the rows render staged-inert (the recording.tsx pattern) until the consuming features land, or the hints disclose the truth ("no history is kept in this build"). The schema's featureExposed flags (all true here) need a consumer-side counterpart before a row goes live.
Confidence: HIGH — Verification: two rg sweeps (engine + UI) both empty; SettingField write path read; W15-A08 audit cross-cited; runtime behavior UNVERIFIED (no GUI binary) but the zero-consumer grep is a static certainty.
### F-002 [S3]
Where: ui/src/screens/settings/areas/privacy.tsx:86-95 (ClearHistoryRow's control cell)
Code:
```ts
          <button
            type="button"
            className="settings-action"
            disabled
            title={reason}
            aria-label={`${m("settings.privacy.clear")} — ${reason}`}
          >
```
What is wrong: The unavailability reason ("History clearing arrives with the playback-history store") is reachable ONLY by mouse hover (title on a disabled, non-focusable button). The one operable affordance on the row — the ⓘ SettingInfoDot — reveals the HINT ("Forget every remembered item and playback position."), not the reason. aria-label on a disabled button is never announced (no focus, no tab stop). A keyboard/AT user cannot learn why Clear is dead — the exact defect staged.tsx's w26.k13 fix (sr-only reason twin + aria-describedby on the disabled control, staged.tsx:94-98, :161-168) eliminated for the staged areas in the SAME directory.
Why it matters: The honest-surface rule this row exists to enforce ("visible, inert, never a dead control that looks alive" — privacy.tsx:64-65) is only half-implemented: visible to sighted pointer users, invisible to keyboard/AT users, inconsistent with the sibling pattern two files away.
What correct looks like: Add the sr-only reason span (`.settings-vh`) with aria-describedby on the disabled button — the staged.tsx w26.k13 twin.
Confidence: HIGH — Verification: read both files; SettingInfoDot contract read in SettingField.tsx:96-120.
### F-003 [S4]
Where: ui/src/screens/settings/areas/privacy.tsx:39-40
Code:
```ts
import { SettingField } from "../SettingField";
import { SettingInfoDot } from "../SettingField";
```
What is wrong: Two import statements from the same module where one suffices (same pattern as recording.tsx:29-31).
Why it matters: Hygiene; copy-paste assembly noise across the area files.
What correct looks like: `import { SettingField, SettingInfoDot } from "../SettingField";`
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `useAreaMsg` binding | 45 | REAL | — | makeAreaMsg over i13 keys; catalog-first (shared.tsx:42-50). |
| `PRIVACY_HINTS` | 48-57 | REAL | — | Covers exactly the 8 UI schema rows; a future schema row falls back to no-ⓘ (SettingField hint prop undefined — honest). |
| `ClearHistoryRow` | 68-105 | REAL | S3 | Disabled Clear + operable ⓘ; reason hover-only for keyboard/AT (F-002); no catalog command exists (verified — claim TRUE). |
| `PrivacyArea` | 112-137 | REAL | S2 | Section loop over schemaSectionViews("privacy"); hand-splices ClearHistoryRow at History's end (frame order); empty-section guard keeps a Clear-less empty section from rendering (:120); renders the 8 consumer-less live rows (F-001). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| areas.test.tsx:159 (9 ⓘ on /settings/privacy) | YES | DOM count incl. the hand Clear row. |
| areas.test.tsx:228-231 (nav to Privacy, aria-current) | YES | Interaction + attribute assertion. |
| Gap: NO test asserts the privacy rows' live-write behavior against a real host, and none pins the absence of consumers (F-001 invisible to the suite — MockTransport accepts every write). No test pins the Clear button's reason for keyboard/AT (F-002 unpinned). | | |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no playback-history store or clearing command exists in the catalog yet" | privacy.tsx:62-63 | REAL | rg `history` in generated/commands.ts → none; engine rg → no history store. |
| "the frame-verbatim row renders with its Clear button DISABLED WITH THE HONEST REASON (visible, inert, never a dead control that looks alive)" | privacy.tsx:63-65 | HALF — reason invisible to keyboard/AT (F-002) | title-only on disabled button; ⓘ reveals hint not reason. |
| "The former 'Data & privacy' telemetry section left the screen … stay CLI/config-only (ui = false)" | privacy.tsx:14-17 | REAL | schema.ts:686-746 constants exist but absent from SETTINGS_ENTRIES (:2514-2521) and SETTINGS_AREAS (:2625-2640); settings.toml:460-476 `ui = false`. |
| "The screen stays schema-driven: sections/entries/order come from the generated seed" | privacy.tsx:24-25 | REAL | shared.tsx:73-92 + schema.ts:2625-2640. |
| Row copy "Keep a list of what you played recently" / "Resume each item where you stopped" (i13 hints) | w26.i13.ts:29-30 | SHALLOW (no engine behavior backs it) | F-001: zero consumers of the written keys. |

## 7. Dead / duplicated / unreachable code
Duplicated: the clear-history copy trio exists in BOTH messages.ts:2749-2751 and i18n/keys/w26.gfixB.ts:64-66 (lane pre-registration seam — by-design until merge, but a drift risk). Unreachable: nothing here. Dead-in-product: the 8 live rows' write targets (F-001).

## 8. Dependencies & flags
shared.tsx (makeAreaMsg, schemaSectionViews), SettingField (+SettingInfoDot), i13 lane keys, central messages.ts. No transport/engine imports in this file (writes flow through SettingField → SettingsProvider). No feature gates on the privacy rows (all featureExposed: true — the gate that would honestly stage them is off).

## 9. Open questions for the next agent
- Who owns the decision that privacy rows ship live while recording rows ship staged? The schema's featureExposed for privacy is true with zero consumers — the settings-schema owner (W11 lane) should reconcile exposure with consumer existence.
- The last.fm password row (privacy.lastfm-password, schema.ts:866+) — check how SettingField renders it: if it is a plain TextField (not type=password), the credential renders in cleartext. SettingField.tsx's type→control map (read: url/list<string>/string → plain TextField) suggests YES — worth a dedicated finding by the SettingField owner; I flag it here because this screen is where the password is shown.
