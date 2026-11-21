# AUDIT: ui/src/screens/settings/areas/interface.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 88 / 3 (useInterfaceMsg factory, INTERFACE_HINTS, InterfaceArea) |
| File verdict | SHALLOW (a real, framework-driven, test-covered renderer of 18 fully inert setting rows) |
| Reachable from shipped entry point? | YES in the UI tree: areas/index.tsx:243 `interface: InterfaceArea` ← settings area router ← /settings route. W14 ceiling (no GUI binary) applies. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg "interface.(keep-above|use-custom-skin|window-decorations)" crates` → ZERO hits (no Rust consumer for any interface.* key); same keys in ui/src outside settings → only i18n msgid definitions; `rg -i skin crates` → only substrings of "asking"/"masking" in comments — no skin engine exists; read schemas/settings.toml:247-261 (use-custom-skin `default = true # frame C2 Skin: "Use a custom skin" → ON`, skin-file `default = ""`); schema.ts row dump → 19 interface.* keys (theme→screen global; skin 2 + window 7 + playback-controls 4 + advanced window-metrics 5 = 18 rendered here); read areas/shared.tsx schemaSectionViews (featureExposed filter, hiddenKeys unused here); areas.test.tsx:147-162 read (ⓘ 11-of-13 test); W11 citation verified (audit/waves/wave-11/SUMMARY.md:22,37). |

## 1. One-paragraph verdict
The cleanest kind of inert screen: a thin, honest, schema-driven renderer — one msg factory, one hint map, one section loop — whose tests pin its Figma-fidelity details (ⓘ on 11 of 13 frame rows, two verified absences) and whose comments document every deviation from the frame. The problem is underneath: all 18 rows it renders are write-only decoration. No interface.* key has a consumer in crates/ (zero hits), none has a UI consumer outside the settings screen (only msgids), and the settings pipeline they feed is a dead in-memory map (W11: "no preference ever persists", SUMMARY.md:22). Worse, "Use a custom skin" ships defaulted ON — the default is copied from the Figma frame's depiction, not from any product capability, and no skin engine exists anywhere in the Rust tree. The "Notifications and System Tray" section header renders permanently empty.

## 2. What actually works (verified)
— verified by reading areas.test.tsx:147-162: routed("/settings/interface") renders the area; the test asserts exactly 11 `.settings-field__info` elements and the two honest absences (remember-window-geometry, hide-controls-after) — this FAILS if the hint wiring or the schema rows drift. Executed in baseline #23 (vitest 136 files).
— verified by schema.ts row dump: the section counts match the header's inventory (skin 2, window 7, playback-controls 4) plus 5 advanced window-metrics rows, all rendered flat per the disclosed "advanced-flagged or not" rule (:18-19); interface.theme correctly routes to the global screen (screen: "global"), not here.
— verified by rg: all 11 hint msgids and all row labels exist in messages.ts (l10n gate baseline #24 PASS backs catalog/usage consistency).
— reachability: areas/index.tsx:243 mounts InterfaceArea for the interface route.

## 3. Findings
### F-001 [S2]
Where: ui/src/screens/settings/areas/interface.tsx:74-76 (+ schemas/settings.toml §interface)
Code:
```
{visible.map((entry) => (
  <SettingField key={entry.key} entry={entry} hint={INTERFACE_HINTS[entry.key]} />
))}
```
What is wrong: Every row rendered by this loop is inert end-to-end. (a) No interface.* key has a Rust consumer: `rg "interface.(keep-above|use-custom-skin|window-decorations)" crates` → zero hits — window-management rows (keep-above, decorations, fullscreen-on, geometry) would need host APIs that do not exist (the tauri host does not even compile, W14). (b) No UI consumer: outside the settings screen these keys appear only as i18n msgids — nothing in the player or chrome reads interface.play-video-in-main-window or auto-hide-controls. (c) The apply path is a dead end: W11 proved settings.apply lands in a process-local BTreeMap — "no preference ever persists" (wave-11 SUMMARY.md:22), so restartRequired rows (window-metrics ×5) demand a restart that cannot change anything. Third confirmed settings area with this anatomy (after subtitles.tsx, video.tsx).
Why it matters: This is the Settings → Interface screen a real user would open to make the window behave; all 18 controls (toggles, enum, path picker, slider) are pixel-perfect no-ops. Same S2 as video.tsx/subtitles.tsx for consistency.
What correct looks like: wire the keys to host calls (tauri window API) or mark the area staged like the dynamics tools do (data-staged-tool + note) instead of live-looking controls.
Confidence: HIGH — Verification: rg across crates and ui/src; W11 summary read; schema read.

### F-002 [S3]
Where: ui/src/screens/settings/areas/interface.tsx:51 (+ schemas/settings.toml:247-249)
Code:
```
  "interface.use-custom-skin": "settings.hint.interface.use-custom-skin",
```
(and schemas/settings.toml:249)
```
default = true             # frame C2 Skin: "Use a custom skin" → ON
```
What is wrong: "Use a custom skin" is defaulted ON because the Figma frame drew the toggle ON — the schema comment says so verbatim. No skin engine exists: the only "skin" occurrences in crates/ are substrings of "asking"/"masking" in comments. The companion interface.skin-file defaults to an empty path. So the shipped default state claims custom-skin mode active with no file and no loader — a product default manufactured from a drawing.
Why it matters: Defaults are product decisions, not frame pixels; a default-on toggle for a nonexistent feature is a lie in the product's baseline state, and the schema comment openly documents that the frame, not the product, decided it.
What correct looks like: default false until a skin loader exists (the frame's depiction is a mock, not a default), or gate the row out of the seeded inventory.
Confidence: HIGH — Verification: settings.toml read; crates rg; schema.ts:326-344 read.

### F-003 [S4]
Where: ui/src/screens/settings/areas/interface.tsx:83-85
Code:
```
      <section className="settings-section" data-section="notifications">
        <h2 className="settings-section__title">{t("settings.section.interface.notifications")}</h2>
      </section>
```
What is wrong: A section header with no rows ships in the UI permanently — the frame's fourth section is header-only (its rows sit below the exported fold), so users see "Notifications and System Tray" followed by nothing, with no explanation (the honesty lives in a code comment users never see).
Why it matters: Dead heading in a shipped screen; users will hunt for the missing rows. The alternative (omitting the header, or a "coming soon" note) is strictly better UX.
What correct looks like: omit the section until rows exist, or render a visible placeholder note.
Confidence: HIGH — Verification: read; frame rationale documented :26-29, :80-82.
Severity: S4 — disclosed in comments, frame-faithful, zero functional impact.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| useInterfaceMsg | 45 | REAL | — | makeAreaMsg over 5 lane key files; standard area pattern |
| INTERFACE_HINTS | 50-62 | REAL | — | 11 entries = test-pinned frame truth; the 2 absences + 5 advanced rows correctly hint-less |
| InterfaceArea | 64-88 | SHALLOW | S2/S3/S4 | F-001 inert rows; F-002 renders the frame-defaulted-ON skin toggle; F-003 empty section; renderer itself correct (section loop, visible.length===0 → null for empty schema sections — but the hand-written notifications section bypasses that guard) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| areas.test.tsx "wires the frame's ⓘ on interface — 11 of 13 rows, the two honest absences" | YES | asserts exact count (11) + two per-row absences via data-setting selectors |
| (no InterfaceArea-specific apply/persistence test) | N/A | nothing to test — no apply path exists; the suite never pretends the rows do anything (consistent with W11's findings) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "renders its schema sections FLAT: every seeded interface row (advanced-flagged or not)" | :18-19 | REAL | schemaSectionViews returns all featureExposed entries; no hiddenKeys passed (shared.tsx read) |
| "Notifications… is a HEADER ONLY… no fabricated rows" | :26-29 | REAL | schema has no notifications rows; section rendered bare (F-003) |
| "11 rows the interface frame carries a circle for; the 2 absent rows… honestly" | :47-49 | REAL | areas.test.tsx:147-162 pins it |
| "no search filter — the shell swaps in cross-area results" | :40-42 | UNVERIFIED (shell behavior, out of this file's scope) | not re-verified here; shell audited by another agent |

## 7. Dead / duplicated / unreachable code
No dead code in-file. The INTERFACE_HINTS typing `keyof typeof i13Keys` correctly constrains to real lane keys. Outward deadness: all 18 rendered rows (F-001).

## 8. Dependencies & flags
SettingField + shared area layer + 5 lane key modules. No feature gates in play (featureGate: "" on the interface rows per schema dump; featureExposed all true). No transport calls in this file — the apply path enters via SettingField's framework (dead per W11).

## 9. Open questions for the next agent
- The window-metrics rows (start-maximized, window-* sizes) are advanced + restartRequired — if settings persistence ever lands, who consumes them? The tauri shell (app.rs) reads none today (W14 audit of the host crate).
- schemas/generated/settings-help.txt advertises --interface-use-custom-skin as a CLI flag — does the lumen CLI parse interface.* flags? (If not, that's a docs/generated-artifact FALSE worth a schema-side audit.)
