# AUDIT: ui/src/screens/settings/areas/privacy-network.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 134 / 1 (NetworkArea) |
| File verdict | SHALLOW (honest staged shell — zero live controls) |
| Reachable from shipped entry point? | YES — routed /settings/network (test privacy-network.test.tsx:95 asserts the areas registry route) -> SettingsScreen -> App |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "network" schemas/settings.toml` (no network-area keys — claim TRUE; only privacy entries placed in a "network" section of the PRIVACY screen, L550-583); `sed -n '470,495p' SettingsScreen.tsx`; `sed -n '1,50p' client/config-settings.ts`; staged.tsx marker scan; test-file scan |

## 1. One-paragraph verdict
The entire Network settings area is inert decoration: nine StagedRow shells
rendering full-strength disabled controls with hard-coded display values,
marked `data-staged="schema-seed"` + tooltips. The file's justification is
TRUE (verified: schemas/settings.toml defines no network-area keys), and the
staging is visible and tested. The privacy half of the group is NOT here —
it lives on the schema-backed privacy screen, which (important cross-finding)
IS wired to the real engine config layer in the desktop shell via
client/config-settings.ts (`config.get-setting`/`set-setting`/`reset-setting`),
with localStorage only as the browser-dev-shell fallback. This qualifies the
Wave-11 "localStorage mock persistence" claim: the mock is the fallback, not
the shipped desktop path.

## 2. What actually works (verified)
— Claim check: `rg -n "network" schemas/settings.toml` -> no network-area
setting keys (the L550-583 hits are privacy.* entries placed in a "network"
SECTION of the privacy screen — different thing). Staging justification TRUE.
— Tests (privacy-network.test.tsx): 10 cases asserting staged-inert rendering,
ⓘ census, no invented proxy options, routing, and the privacy screen's
frame-verbatim rows — honest about the surface being staged. Not run (no
node_modules; baseline vitest 1705 passed cited).
— Settings wiring cross-check (SettingsScreen.tsx:489-491): provider mounts
with `persistence={config ?? localStorageSettingsPersistence}` +
`subscribe={client.subscribe}` — engine path first, hydration gate before
render (L478-486).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/settings/areas/privacy-network.tsx:51-131
Code:
```
        <StagedRow
          rowKey="network.buffer-size"
          label={t("settings.network.buffer-size")}
          control={{ kind: "enum", value: t("settings.network.value.buffer.normal") }}
          seam={seam}
          reason={staged}
          info
        />
```
What is wrong: all 9 rows are staged shells — Buffering (5), Proxy (2),
Security (2) — plus a StagedAdvancedBar with "nothing behind it" (L130).
No control persists or affects anything; the values shown ("Normal" buffer,
"System" proxy, "Balanced" cipher) are display props.
Why it matters: an entire settings screen is a picture of settings; a user
who sets a proxy here (were it not disabled) would change nothing. Honesty is
carried by markers/tooltips, matching the Wave-11 dead-settings pattern,
though never pretending to be live.
Confidence: HIGH — Verification: read + rg (schema) + staged.tsx marker scan.

F-002 [S4]
Where: ui/src/screens/settings/areas/privacy-network.tsx:113-120
Code:
```
        <StagedRow
          rowKey="network.verify-certificates"
          label={t("settings.network.verify-certificates")}
          control={{ kind: "switch", on: true }}
```
What is wrong: the staged switch displays `on: true` — a fabricated
"verify certificates: ON" state with no backing data. Marked staged, so not
fraud, but the default display implies a security posture that does not exist.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| NetworkArea | 35-134 | SHALLOW | S3 (F-001) | 9 staged rows + staged advanced bar; no live control |

(File is 134 LOC; single component — compact table per spec.)

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| "renders the frame's rows STAGED inert — visible, full-strength, never dead-but-alive" | YES | staged-marker assertions (L35) |
| "invents no live proxy options the schema does not define" | YES | option census vs schema (L87) |
| "schema truth: the telemetry rows stay CLI/config-only (ui=false)" | YES | schema assertions (L102) |
Tests validate the honesty layer of a dead surface — appropriate given none
of it is functional.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no network [[setting]] keys exist in schemas/settings.toml yet" | privacy-network.tsx:6-7 | REAL | rg over schemas/settings.toml (empty for network-area keys) |
| "the PRIVACY half ... stays schema-backed on the generated privacy screen" | privacy-network.tsx:18-19 | REAL | privacy.tsx + generated screens; engine persistence via config-settings.ts (read) |
| "(§C10 y669) the collapsed Advanced bar, staged (nothing behind it)" | privacy-network.tsx:130 | REAL | StagedAdvancedBar with staged reason |

## 7. Dead / duplicated / unreachable code
None beyond the staged-by-design surface. NOTE the filename (privacy-network)
vs content (NetworkArea only) — mild naming debt (S4-class, folded here).

## 8. Dependencies & flags
StagedRow/StagedAdvancedBar/useStagedMsg from ./staged, useMsg, staged.css.
No transport import — this area is backendless by schema gap. No feature gates.

## 9. Open questions for the next agent
- Settings persistence ownership: config-settings.ts claims "The REAL settings
  write path" — verify its transport round-trip against the ENGINE's
  config.set-setting handler (commands domain) and whether the browser
  DemoTransport honestly answers feature-not-exposed for config.* (claimed in
  its header). That decides whether Wave-11's "localStorage mock" indictment
  stands for the desktop shell (evidence here says NO — engine path mounts
  first).
