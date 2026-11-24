# AUDIT: ui/src/screens/settings/areas/privacy.tsx
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines / functions | 137 / 2 (ClearHistoryRow, PrivacyArea) + useAreaMsg const |
| File verdict | REAL (schema-driven area, engine-wired in the desktop shell; one honestly-dead action row) |
| Reachable from shipped entry point? | YES — SettingsScreen areas layer -> PrivacyArea (route /settings/privacy); SettingField writes flow through SettingsScreen's persistence (desktop: config-settings.ts -> lumen_dispatch; browser: localStorage) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `rg "save-recent-items\|fetch-metadata-online\|submit-lastfm" schemas/ crates/` -> REAL engine keys (settings-docs.md:257-266 + CLI flags); `rg history crates/commands/src/catalog/ids.rs` -> no history command; schema.ts:867-875 lastfm-password entry; SettingField.tsx:254 TextField for strings |

## 1. One-paragraph verdict
This area is on the REAL side of the master question: its 8 schema rows map
to genuine engine config keys (verified in schemas/generated/settings-docs.md
— each with a CLI flag twin), and Apply round-trips them through
config.set-setting over lumen_dispatch in the desktop shell. The one
action row — "Clear history now" — is a permanently disabled button, and
unlike a dead button that looks alive, it carries the honest reason in both
`title` and `aria-label` (:91-92): no playback-history store or clearing
command exists in the catalog (verified: no history command in ids.rs).
The warts: the last.fm password is a plain-text TextField (schema type
"string" -> TextField, SettingField.tsx:254) — no masked input exists in
the settings grammar, and the value is stored in engine config as
cleartext.

## 2. What actually works (verified)
— Key wiring: privacy.* keys exist engine-side (settings-docs.md:257-266),
so reads hydrate real values and writes reach the real config layer
(transport chain from config-settings.ts audit).
— Schema-driven rendering: schemaSectionViews("privacy") with visible-row
filtering; hints wired per-row via PRIVACY_HINTS (:48-57) — 8/8 rows
covered, matching the frame's 9 ⓘ circles minus the action row's own
(:78-84).
— Vitest baseline (1705 passed) includes the settings-area suites.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/privacy.tsx:87-95
Code:
          <button
            type="button"
            className="settings-action"
            disabled
            title={reason}
            aria-label={`${m("settings.privacy.clear")} — ${reason}`}
          >
What is wrong: the Clear button can never fire in any shipped build — no
history store, no clearing command (rg over catalog ids -> none; the file's
own comment :61-66 says the history seam is the Phase-4 library feature).
It is an HONEST dead control (visible reason, disabled state), but it is
still dead surface shipped in the release UI. What correct looks like:
this is arguably the correct honest posture; the alternative (hide the row
until the seam lands) is a product call. Confidence: HIGH — Verification:
read + rg.
F-002 [S3]
Where: ui/src/screens/settings/areas/privacy.tsx:54-56 (schema.ts:867-875; SettingField.tsx:254)
Code:
  "privacy.lastfm-password": "settings.hint.privacy.lastfm-password",
  "privacy.lastfm-submit-tasks": ... (schema)
What is wrong: `privacy.lastfm-password` (type "string") renders as a
PLAIN TextField — no password masking anywhere in the settings grammar
(rg `type="password"` over SettingField/TextField -> no matches). The
credential is displayed in cleartext on screen and persisted as cleartext
config (defaultValue ""). Why it matters: a UI that asks for a password
and echoes it in cleartext is a credential-handling defect, frame-verbatim
or not. What correct looks like: a masked input type in the settings
schema grammar (or defer the row until last.fm exists at all — no last.fm
submission code exists in the engine either). Confidence: HIGH —
Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| ClearHistoryRow | 68-105 | REAL | S3 | honestly-dead action (F-001); hint toggle real |
| PrivacyArea | 112-137 | REAL | — | schema-driven; sections order from seed |

## 5. Test honesty
No test file for privacy.tsx itself (areas are covered via
SettingsScreen/SettingField suites + generated-screen checks
`settings:check` script). UNVERIFIED: whether any test pins the disabled
Clear button's honest reason (mutation: enabling the button would not fail
a settings-area test I could find — the row is hand-spliced, outside the
generated pin).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no playback-history store or clearing command exists in the catalog yet" | :62-63 | REAL | rg over catalog ids.rs -> no history command |
| "rows carry no frame anywhere ... stay CLI/config-only (ui = false)" (telemetry) | :14-17 | REAL-UNPROVEN | schema.ts still lists privacy.telemetry/crash-reports/usage-stats entries (:687-729); ui=false filtering not verified in this audit (shared.ts is another agent's file) |
| "the screen stays schema-driven: sections/entries/order come from the generated seed" | :25 | REAL | :116-126 schemaSectionViews loop |

## 7. Dead / duplicated / unreachable code
The Clear button is intentionally dead (F-001). No duplicated logic;
hints table is complete for the 8 rows.

## 8. Dependencies & flags
shared.ts (makeAreaMsg, schemaSectionViews), SettingField, SettingInfoDot,
i18n w26.i13. No flags. Backend proximity: one persistence hop
(SettingsScreen) then lumen_dispatch.

## 9. Open questions for the next agent
- shared.ts auditor: does schemaSectionViews actually filter ui=false rows
  (telemetry/crash-reports/usage-stats)? If not, three engine-only rows
  render in the privacy screen contrary to :14-17.
- Engine side: is there ANY last.fm client code (crates/) or are these
  three lastfm rows config keys that nothing consumes (dead config)?
